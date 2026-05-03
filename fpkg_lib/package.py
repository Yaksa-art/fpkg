from __future__ import annotations

import os
import hashlib
import zipfile
import tomllib
import tomli_w
from io import BytesIO
from pathlib import Path
from typing import Optional

try:
    import blake3 as _blake3
    def _hash_bytes(data: bytes) -> str:
        return "blake3:" + _blake3.blake3(data).hexdigest()
    def _hash_file(path: str) -> str:
        h = _blake3.blake3()
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(65536), b""):
                h.update(chunk)
        return "blake3:" + h.hexdigest()
    HASH_ALGO = "blake3"
except ImportError:
    def _hash_bytes(data: bytes) -> str:
        return "sha256:" + hashlib.sha256(data).hexdigest()
    def _hash_file(path: str) -> str:
        h = hashlib.sha256()
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(65536), b""):
                h.update(chunk)
        return "sha256:" + h.hexdigest()
    HASH_ALGO = "sha256"

from .manifest import Manifest


FPKG_VERSION = "1"
REQUIRED_META = {"META/manifest.toml", "META/checksums.blake3"}


class FpkgError(Exception):
    pass


class FpkgReader:
    def __init__(self, path: str):
        self.path = path
        self._zf: Optional[zipfile.ZipFile] = None

    def __enter__(self):
        self._zf = zipfile.ZipFile(self.path, "r")
        return self

    def __exit__(self, *_):
        if self._zf:
            self._zf.close()

    def _require_open(self):
        if not self._zf:
            raise FpkgError("Use FpkgReader as context manager")

    def names(self) -> list[str]:
        self._require_open()
        return self._zf.namelist()

    def read(self, name: str) -> bytes:
        self._require_open()
        try:
            return self._zf.read(name)
        except KeyError:
            raise FpkgError(f"Missing file in archive: {name}")

    def manifest(self) -> Manifest:
        raw = self.read("META/manifest.toml")
        return Manifest.from_toml(raw)

    def checksums(self) -> dict[str, str]:
        raw = self.read("META/checksums.blake3")
        result = {}
        for line in raw.decode().splitlines():
            line = line.strip()
            if not line:
                continue
            parts = line.split("  ", 1)
            if len(parts) == 2:
                result[parts[1]] = parts[0]
        return result

    def dependencies_toml(self) -> Optional[dict]:
        try:
            raw = self.read("META/dependencies.toml")
            return tomllib.loads(raw.decode())
        except FpkgError:
            return None

    def data_files(self) -> list[str]:
        self._require_open()
        return [n for n in self._zf.namelist() if n.startswith("DATA/") and not n.endswith("/")]

    def compat_origin(self) -> str:
        try:
            return self.read("COMPAT/origin_format.txt").decode().strip()
        except FpkgError:
            return "native"

    def verify(self) -> tuple[bool, list[str]]:
        self._require_open()
        errors = []

        for req in REQUIRED_META:
            if req not in self._zf.namelist():
                errors.append(f"Missing required file: {req}")

        if errors:
            return False, errors

        checksums = self.checksums()
        for filepath, expected_hash in checksums.items():
            try:
                data = self._zf.read(filepath)
                actual = _hash_bytes(data)
                if actual != expected_hash:
                    errors.append(f"Checksum mismatch: {filepath}")
            except KeyError:
                errors.append(f"File referenced in checksums not found: {filepath}")

        return len(errors) == 0, errors

    def extract(self, dest: str, data_only: bool = True):
        self._require_open()
        dest_path = Path(dest)
        for name in self._zf.namelist():
            if data_only and not name.startswith("DATA/"):
                continue
            target_name = name[len("DATA/"):] if data_only else name
            if not target_name or target_name.endswith("/"):
                continue
            target = dest_path / target_name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(self._zf.read(name))


class FpkgWriter:
    def __init__(self, output_path: str):
        self.output_path = output_path
        self._manifest: Optional[Manifest] = None
        self._data_files: list[tuple[str, bytes]] = []
        self._scripts: dict[str, bytes] = {}
        self._origin: str = "native"
        self._changelog: str = ""

    def set_manifest(self, manifest: Manifest):
        self._manifest = manifest

    def add_data_file(self, archive_path: str, content: bytes):
        if not archive_path.startswith("DATA/"):
            archive_path = "DATA/" + archive_path.lstrip("/")
        self._data_files.append((archive_path, content))

    def add_data_directory(self, source_dir: str, prefix: str = ""):
        source = Path(source_dir)
        for file_path in source.rglob("*"):
            if not file_path.is_file():
                continue
            rel = file_path.relative_to(source)
            arc_path = "DATA/" + (prefix + "/" if prefix else "") + str(rel)
            self.add_data_file(arc_path, file_path.read_bytes())

    def set_script(self, name: str, content: bytes):
        allowed = {"pre-install.sh", "post-install.sh", "pre-remove.sh", "post-remove.sh"}
        if name not in allowed:
            raise FpkgError(f"Unknown script: {name}. Allowed: {allowed}")
        self._scripts[name] = content

    def set_origin(self, origin: str):
        self._origin = origin

    def set_changelog(self, content: str):
        self._changelog = content

    def _build_checksums(self) -> bytes:
        lines = []
        for arc_path, content in self._data_files:
            h = _hash_bytes(content)
            lines.append(f"{h}  {arc_path}")
        return "\n".join(lines).encode()

    def _compute_merkle_root(self) -> str:
        if not self._data_files:
            return _hash_bytes(b"")
        hashes = [_hash_bytes(content) for _, content in self._data_files]
        combined = "\n".join(sorted(hashes)).encode()
        return _hash_bytes(combined)

    def _build_deps_toml(self) -> bytes:
        if not self._manifest:
            return b""
        deps = self._manifest.dependencies
        entries = []
        for req in deps.requires:
            parts = req.split(" ", 1)
            name = parts[0]
            version = parts[1] if len(parts) > 1 else ""
            entry = {"name": name, "version": version, "optional": False, "reason": ""}
            entries.append(entry)
        for sug in deps.suggests:
            parts = sug.split(" ", 1)
            name = parts[0]
            version = parts[1] if len(parts) > 1 else ""
            entry = {"name": name, "version": version, "optional": True, "reason": "", "group": "optional"}
            entries.append(entry)
        return tomli_w.dumps({"dep": entries}).encode() if entries else b""

    def write(self) -> str:
        if not self._manifest:
            raise FpkgError("Manifest is required")

        errors = self._manifest.validate()
        if errors:
            raise FpkgError("Manifest validation failed:\n" + "\n".join(f"  - {e}" for e in errors))

        checksums = self._build_checksums()
        merkle_root = self._compute_merkle_root()

        self._manifest.verification.content_tree = merkle_root

        manifest_toml = self._manifest.to_toml()
        manifest_hash = _hash_bytes(manifest_toml)
        self._manifest.verification.manifest_hash = manifest_hash
        manifest_toml = self._manifest.to_toml()

        total_data = sum(len(c) for _, c in self._data_files)
        self._manifest.package.size.installed = total_data

        with zipfile.ZipFile(self.output_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
            zf.writestr("META/manifest.toml", manifest_toml)
            zf.writestr("META/checksums.blake3", checksums)

            deps_toml = self._build_deps_toml()
            if deps_toml:
                zf.writestr("META/dependencies.toml", deps_toml)

            if self._changelog:
                zf.writestr("META/changelog.md", self._changelog.encode())

            for script_name, content in self._scripts.items():
                zf.writestr(f"META/scripts/{script_name}", content)

            for arc_path, content in self._data_files:
                zf.writestr(arc_path, content)

            zf.writestr("COMPAT/origin_format.txt", self._origin.encode())

        final_size = os.path.getsize(self.output_path)
        self._manifest.package.size.compressed = final_size

        return self.output_path
