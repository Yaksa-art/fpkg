from __future__ import annotations

import os
import sys
import shutil
import subprocess
import tempfile
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional
from datetime import datetime, timezone

from .manifest import (
    Manifest, ManifestPackage, ManifestVerification,
    ManifestDependencies, ManifestInstall, ManifestRepository,
    ManifestCompat, PackageSize, PackageFlags,
)
from .package import FpkgWriter


class BuildError(Exception):
    pass


@dataclass
class PkgBuildSource:
    url: str = ""
    sha256: str = ""
    local: str = ""


@dataclass
class PkgBuildRuntime:
    requires: list[str] = field(default_factory=list)
    suggests: list[str] = field(default_factory=list)
    conflicts: list[str] = field(default_factory=list)
    provides: list[str] = field(default_factory=list)


@dataclass
class PkgBuildSection:
    build_depends: list[str] = field(default_factory=list)
    script: str = ""


@dataclass
class PkgBuildInstallSection:
    script: str = ""
    config_files: list[str] = field(default_factory=list)
    desktop_files: list[str] = field(default_factory=list)
    mode: str = "both"


@dataclass
class PkgBuild:
    name: str
    version: str
    release: int = 1
    arch: str = "x86_64"
    license: str = ""
    summary: str = ""
    description: str = ""
    homepage: str = ""
    maintainer: str = ""
    categories: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)
    source: PkgBuildSource = field(default_factory=PkgBuildSource)
    build: PkgBuildSection = field(default_factory=PkgBuildSection)
    package_install: PkgBuildInstallSection = field(default_factory=PkgBuildInstallSection)
    runtime: PkgBuildRuntime = field(default_factory=PkgBuildRuntime)
    scripts: dict[str, str] = field(default_factory=dict)
    changelog: str = ""

    @classmethod
    def from_file(cls, path: str) -> PkgBuild:
        with open(path, "rb") as f:
            data = tomllib.load(f)
        return cls.from_dict(data)

    @classmethod
    def from_dict(cls, data: dict) -> PkgBuild:
        pkg = data.get("package", {})
        src = data.get("source", {})
        bld = data.get("build", {})
        inst = data.get("package_install", {})
        rt = data.get("runtime", {})
        scr = data.get("scripts", {})

        return cls(
            name=pkg.get("name", ""),
            version=pkg.get("version", ""),
            release=pkg.get("release", 1),
            arch=pkg.get("arch", "x86_64"),
            license=pkg.get("license", ""),
            summary=pkg.get("summary", ""),
            description=pkg.get("description", ""),
            homepage=pkg.get("homepage", ""),
            maintainer=pkg.get("maintainer", ""),
            categories=pkg.get("categories", []),
            tags=pkg.get("tags", []),
            source=PkgBuildSource(
                url=src.get("url", ""),
                sha256=src.get("sha256", ""),
                local=src.get("local", ""),
            ),
            build=PkgBuildSection(
                build_depends=bld.get("build_depends", []),
                script=bld.get("script", ""),
            ),
            package_install=PkgBuildInstallSection(
                script=inst.get("script", ""),
                config_files=inst.get("config_files", []),
                desktop_files=inst.get("desktop_files", []),
                mode=inst.get("mode", "both"),
            ),
            runtime=PkgBuildRuntime(
                requires=rt.get("requires", []),
                suggests=rt.get("suggests", []),
                conflicts=rt.get("conflicts", []),
                provides=rt.get("provides", []),
            ),
            scripts=scr,
            changelog=data.get("changelog", ""),
        )

    def validate(self) -> list[str]:
        errors = []
        if not self.name:
            errors.append("package.name is required")
        if not self.version:
            errors.append("package.version is required")
        if self.arch not in ("x86_64", "aarch64", "riscv64", "any"):
            errors.append("package.arch must be x86_64 | aarch64 | riscv64 | any")
        return errors

    def output_filename(self) -> str:
        return f"{self.name}-{self.version}-{self.release}-{self.arch}.fpkg"


class Builder:
    def __init__(self, pkgbuild: PkgBuild, output_dir: str = ".", verbose: bool = False):
        self.pkgbuild = pkgbuild
        self.output_dir = Path(output_dir)
        self.verbose = verbose

    def _log(self, msg: str):
        print(msg, flush=True)

    def _run_script(self, script: str, env: dict[str, str]):
        if not script.strip():
            return
        result = subprocess.run(
            ["/bin/sh", "-e", "-c", script],
            env={**os.environ, **env},
            capture_output=not self.verbose,
        )
        if result.returncode != 0:
            stderr = result.stderr.decode() if result.stderr else ""
            raise BuildError(f"Script failed (exit {result.returncode}):\n{stderr}")

    def build(self) -> str:
        errors = self.pkgbuild.validate()
        if errors:
            raise BuildError("PKGBUILD validation failed:\n" + "\n".join(f"  - {e}" for e in errors))

        pkg = self.pkgbuild
        output_path = self.output_dir / pkg.output_filename()

        self._log(f"[*] Building {pkg.name} {pkg.version}-{pkg.release} ({pkg.arch})")

        with tempfile.TemporaryDirectory(prefix="fpkg-build-") as tmpdir:
            srcdir = os.path.join(tmpdir, "src")
            destdir = os.path.join(tmpdir, "dest")
            os.makedirs(srcdir, exist_ok=True)
            os.makedirs(destdir, exist_ok=True)

            if pkg.source.local:
                local_src = Path(pkg.source.local)
                if not local_src.exists():
                    raise BuildError(f"Local source not found: {pkg.source.local}")
                if local_src.is_dir():
                    shutil.copytree(str(local_src), srcdir, dirs_exist_ok=True)
                else:
                    shutil.copy2(str(local_src), srcdir)
                self._log(f"[✓] Source: {pkg.source.local}")
            elif pkg.source.url:
                self._log(f"[*] Fetching source: {pkg.source.url}")
                self._fetch_source(pkg.source.url, pkg.source.sha256, srcdir)
                self._log(f"[✓] Source fetched")
            else:
                self._log(f"[!] No source specified, using empty DATA/")

            if pkg.build.script.strip():
                self._log(f"[*] Running build script...")
                self._run_script(pkg.build.script, {
                    "FPM_SRCDIR": srcdir,
                    "FPM_DESTDIR": destdir,
                    "FPM_NAME": pkg.name,
                    "FPM_VERSION": pkg.version,
                    "FPM_ARCH": pkg.arch,
                })
                self._log(f"[✓] Build complete")

            if pkg.package_install.script.strip():
                self._log(f"[*] Running install script...")
                self._run_script(pkg.package_install.script, {
                    "FPM_SRCDIR": srcdir,
                    "FPM_DESTDIR": destdir,
                    "FPM_NAME": pkg.name,
                    "FPM_VERSION": pkg.version,
                    "FPM_ARCH": pkg.arch,
                })
                self._log(f"[✓] Install script complete")

            manifest = self._build_manifest(pkg)
            writer = FpkgWriter(str(output_path))
            writer.set_manifest(manifest)

            data_files_count = 0
            for file_path in Path(destdir).rglob("*"):
                if not file_path.is_file():
                    continue
                rel = file_path.relative_to(destdir)
                writer.add_data_file(str(rel), file_path.read_bytes())
                data_files_count += 1

            for script_name, script_content in pkg.scripts.items():
                writer.set_script(script_name, script_content.encode())

            if pkg.changelog:
                writer.set_changelog(pkg.changelog)

            writer.set_origin("native")

            self._log(f"[*] Packing {data_files_count} file(s)...")
            writer.write()
            self._log(f"[✓] Package created: {output_path}")

        return str(output_path)

    def _fetch_source(self, url: str, expected_sha256: str, destdir: str):
        import urllib.request
        import hashlib

        filename = url.split("/")[-1].split("?")[0] or "source"
        dest_file = os.path.join(destdir, filename)

        urllib.request.urlretrieve(url, dest_file)

        if expected_sha256:
            with open(dest_file, "rb") as f:
                actual = hashlib.sha256(f.read()).hexdigest()
            if actual != expected_sha256:
                raise BuildError(f"SHA256 mismatch: expected {expected_sha256}, got {actual}")

        if filename.endswith((".tar.gz", ".tgz", ".tar.bz2", ".tar.xz", ".tar.zst")):
            import tarfile
            with tarfile.open(dest_file) as tf:
                tf.extractall(destdir)
            os.unlink(dest_file)
        elif filename.endswith(".zip"):
            import zipfile
            with zipfile.ZipFile(dest_file) as zf:
                zf.extractall(destdir)
            os.unlink(dest_file)

    def _build_manifest(self, pkg: PkgBuild) -> Manifest:
        return Manifest(
            package=ManifestPackage(
                name=pkg.name,
                version=pkg.version,
                release=pkg.release,
                arch=pkg.arch,
                license=pkg.license,
                summary=pkg.summary,
                description=pkg.description,
                homepage=pkg.homepage,
                source_url=pkg.source.url or pkg.source.local,
                maintainer=pkg.maintainer,
                build_date=datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                categories=pkg.categories,
                tags=pkg.tags,
            ),
            verification=ManifestVerification(
                signature_algo="ed25519",
            ),
            dependencies=ManifestDependencies(
                requires=pkg.runtime.requires,
                suggests=pkg.runtime.suggests,
                conflicts=pkg.runtime.conflicts,
                provides=pkg.runtime.provides,
            ),
            install=ManifestInstall(
                mode=pkg.package_install.mode,
                config_files=pkg.package_install.config_files,
                desktop_files=pkg.package_install.desktop_files,
            ),
            repository=ManifestRepository(),
            compat=ManifestCompat(min_fpm_version="0.1.0", fpkg_format="1"),
        )
