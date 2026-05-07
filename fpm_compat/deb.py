from __future__ import annotations

import io
import os
import struct
import tarfile
from pathlib import Path
from typing import Optional

from fpkg_lib.manifest import (
    Manifest, ManifestPackage, ManifestDependencies,
    ManifestInstall, ManifestCompat,
)
from fpkg_lib.package import FpkgWriter
from .arch import normalise as norm_arch


AR_MAGIC = b"!<arch>\n"
AR_HEADER_SIZE = 60


def _parse_ar(data: bytes) -> dict[str, bytes]:
    if not data.startswith(AR_MAGIC):
        raise ValueError("Not an ar archive")
    pos = len(AR_MAGIC)
    members: dict[str, bytes] = {}
    while pos + AR_HEADER_SIZE <= len(data):
        name = data[pos:pos+16].decode("ascii", errors="replace").rstrip("/ ")
        size = int(data[pos+48:pos+58].decode().strip())
        pos += AR_HEADER_SIZE
        members[name] = data[pos:pos+size]
        pos += size + (size % 2)
    return members


def _extract_tar(raw: bytes, compression: str) -> dict[str, bytes]:
    mode = "r:" + compression
    with tarfile.open(fileobj=io.BytesIO(raw), mode=mode) as tf:
        files: dict[str, bytes] = {}
        for member in tf.getmembers():
            if not member.isfile():
                continue
            f = tf.extractfile(member)
            if f:
                files[member.name.lstrip("./")] = f.read()
        return files


def _parse_control(raw: bytes) -> dict[str, str]:
    fields: dict[str, str] = {}
    current_key = ""
    for line in raw.decode(errors="replace").splitlines():
        if line.startswith(" "):
            if current_key:
                fields[current_key] = fields.get(current_key, "") + "\n" + line.strip()
        elif ": " in line:
            k, _, v = line.partition(": ")
            current_key = k.lower()
            fields[current_key] = v.strip()
    return fields


def _split_deps(dep_str: str) -> list[str]:
    deps = []
    for part in dep_str.split(","):
        part = part.strip()
        if not part:
            continue
        alts = part.split("|")
        name = alts[0].strip().split(" ")[0]
        if name:
            deps.append(name)
    return deps


class DebConverter:
    def __init__(self, src: str, out: Optional[str], arch_override: Optional[str]):
        self.src = src
        self.out = out
        self.arch_override = arch_override

    def convert(self) -> str:
        raw = Path(self.src).read_bytes()
        members = _parse_ar(raw)

        control_raw = self._find_tar(members, "control")
        data_raw, data_comp = self._find_data_tar(members)

        control_files = _extract_tar(control_raw, control_raw[:2] == b"\x1f\x8b" and "gz" or self._detect_comp(control_raw))
        control = _parse_control(control_files.get("control", b"") if isinstance(control_files.get("control"), bytes) else b"")

        data_files = _extract_tar(data_raw, data_comp)

        arch = self.arch_override or norm_arch(control.get("architecture", "amd64"))
        name = control.get("package", Path(self.src).stem)
        version = control.get("version", "0.0.0").lstrip("0123456789:").lstrip()
        version = version or control.get("version", "0.0.0")

        deps = _split_deps(control.get("depends", ""))
        suggests = _split_deps(control.get("recommends", "") + "," + control.get("suggests", ""))
        conflicts = _split_deps(control.get("conflicts", ""))
        provides = _split_deps(control.get("provides", ""))

        manifest = Manifest(
            package=ManifestPackage(
                name=name,
                version=version,
                arch=arch,
                license=control.get("license", ""),
                summary=control.get("description", "").splitlines()[0] if control.get("description") else "",
                description=control.get("description", ""),
                homepage=control.get("homepage", ""),
                maintainer=control.get("maintainer", ""),
            ),
            dependencies=ManifestDependencies(
                requires=deps,
                suggests=suggests,
                conflicts=conflicts,
                provides=provides,
            ),
            install=ManifestInstall(mode="system"),
            compat=ManifestCompat(),
        )

        out_path = self.out or str(Path(self.src).with_suffix(".fpkg"))
        writer = FpkgWriter(out_path)
        writer.set_manifest(manifest)
        writer.set_origin("deb")

        for rel_path, content in data_files.items():
            writer.add_data_file(rel_path, content)

        for script_name, hook_name in [
            ("preinst", "pre-install.sh"),
            ("postinst", "post-install.sh"),
            ("prerm", "pre-remove.sh"),
            ("postrm", "post-remove.sh"),
        ]:
            if script_name in control_files:
                writer.set_script(hook_name, control_files[script_name])

        writer.write()
        return out_path

    def _find_tar(self, members: dict[str, bytes], prefix: str) -> bytes:
        for comp in [".tar.gz", ".tar.xz", ".tar.zst", ".tar.bz2", ".tar"]:
            key = prefix + comp
            if key in members:
                return members[key]
        for key, val in members.items():
            if key.startswith(prefix + ".") or key == prefix:
                return val
        raise ValueError(f"No {prefix} tar found in .deb")

    def _find_data_tar(self, members: dict[str, bytes]) -> tuple[bytes, str]:
        for key, comp in [
            ("data.tar.zst", "zst"),
            ("data.tar.xz", "xz"),
            ("data.tar.gz", "gz"),
            ("data.tar.bz2", "bz2"),
            ("data.tar", ""),
        ]:
            if key in members:
                return members[key], comp
        raise ValueError("No data.tar.* found in .deb")

    def _detect_comp(self, raw: bytes) -> str:
        if raw[:2] == b"\x1f\x8b":
            return "gz"
        if raw[:6] == b"\xfd7zXZ\x00":
            return "xz"
        if raw[:4] == b"\x28\xb5\x2f\xfd":
            return "zst"
        if raw[:2] == b"BZ":
            return "bz2"
        return ""
