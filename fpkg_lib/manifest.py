from __future__ import annotations

import tomllib
import tomli_w
from dataclasses import dataclass, field, asdict
from typing import Optional
from datetime import datetime, timezone


@dataclass
class PackageSize:
    installed: int = 0
    compressed: int = 0
    delta_base: str = ""


@dataclass
class PackageFlags:
    system_config: bool = False
    has_services: bool = False
    selinux_aware: bool = False
    has_suid: bool = False


@dataclass
class ManifestPackage:
    name: str
    version: str
    release: int = 1
    arch: str = "x86_64"
    license: str = ""
    summary: str = ""
    description: str = ""
    homepage: str = ""
    source_url: str = ""
    maintainer: str = ""
    build_date: str = ""
    categories: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)
    size: PackageSize = field(default_factory=PackageSize)
    flags: PackageFlags = field(default_factory=PackageFlags)

    def __post_init__(self):
        if not self.build_date:
            self.build_date = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        if isinstance(self.size, dict):
            self.size = PackageSize(**self.size)
        if isinstance(self.flags, dict):
            self.flags = PackageFlags(**self.flags)


@dataclass
class ManifestVerification:
    manifest_hash: str = ""
    content_tree: str = ""
    signature_algo: str = "ed25519"


@dataclass
class DepEntry:
    name: str
    version: str = ""
    optional: bool = False
    reason: str = ""
    arch: list[str] = field(default_factory=list)
    group: str = ""


@dataclass
class ManifestDependencies:
    requires: list[str] = field(default_factory=list)
    suggests: list[str] = field(default_factory=list)
    conflicts: list[str] = field(default_factory=list)
    provides: list[str] = field(default_factory=list)
    before: list[str] = field(default_factory=list)
    after: list[str] = field(default_factory=list)


@dataclass
class ManifestInstall:
    mode: str = "both"
    config_files: list[str] = field(default_factory=list)
    desktop_files: list[str] = field(default_factory=list)


@dataclass
class ManifestRepository:
    origin_repo: str = ""
    origin_url: str = ""
    fetched_at: str = ""


@dataclass
class ManifestCompat:
    min_fpm_version: str = "0.1.0"
    fpkg_format: str = "1"


@dataclass
class Manifest:
    package: ManifestPackage
    verification: ManifestVerification = field(default_factory=ManifestVerification)
    dependencies: ManifestDependencies = field(default_factory=ManifestDependencies)
    install: ManifestInstall = field(default_factory=ManifestInstall)
    repository: ManifestRepository = field(default_factory=ManifestRepository)
    compat: ManifestCompat = field(default_factory=ManifestCompat)

    def validate(self) -> list[str]:
        errors = []
        if not self.package.name:
            errors.append("package.name is required")
        if not self.package.version:
            errors.append("package.version is required")
        if self.install.mode not in ("system", "user", "both"):
            errors.append("install.mode must be 'system', 'user', or 'both'")
        if self.package.arch not in ("x86_64", "aarch64", "riscv64", "any"):
            errors.append("package.arch must be x86_64 | aarch64 | riscv64 | any")
        return errors

    def to_dict(self) -> dict:
        return {
            "package": {
                "name": self.package.name,
                "version": self.package.version,
                "release": self.package.release,
                "arch": self.package.arch,
                "license": self.package.license,
                "summary": self.package.summary,
                "description": self.package.description,
                "homepage": self.package.homepage,
                "source_url": self.package.source_url,
                "maintainer": self.package.maintainer,
                "build_date": self.package.build_date,
                "categories": self.package.categories,
                "tags": self.package.tags,
                "size": {
                    "installed": self.package.size.installed,
                    "compressed": self.package.size.compressed,
                    "delta_base": self.package.size.delta_base,
                },
                "flags": {
                    "system_config": self.package.flags.system_config,
                    "has_services": self.package.flags.has_services,
                    "selinux_aware": self.package.flags.selinux_aware,
                    "has_suid": self.package.flags.has_suid,
                },
            },
            "verification": {
                "manifest_hash": self.verification.manifest_hash,
                "content_tree": self.verification.content_tree,
                "signature_algo": self.verification.signature_algo,
            },
            "dependencies": {
                "requires": self.dependencies.requires,
                "suggests": self.dependencies.suggests,
                "conflicts": self.dependencies.conflicts,
                "provides": self.dependencies.provides,
                "before": self.dependencies.before,
                "after": self.dependencies.after,
            },
            "install": {
                "mode": self.install.mode,
                "config_files": self.install.config_files,
                "desktop_files": self.install.desktop_files,
            },
            "repository": {
                "origin_repo": self.repository.origin_repo,
                "origin_url": self.repository.origin_url,
                "fetched_at": self.repository.fetched_at,
            },
            "compat": {
                "min_fpm_version": self.compat.min_fpm_version,
                "fpkg_format": self.compat.fpkg_format,
            },
        }

    def to_toml(self) -> bytes:
        return tomli_w.dumps(self.to_dict()).encode()

    @classmethod
    def from_dict(cls, data: dict) -> Manifest:
        pkg_data = data.get("package", {})
        pkg = ManifestPackage(
            name=pkg_data.get("name", ""),
            version=pkg_data.get("version", ""),
            release=pkg_data.get("release", 1),
            arch=pkg_data.get("arch", "x86_64"),
            license=pkg_data.get("license", ""),
            summary=pkg_data.get("summary", ""),
            description=pkg_data.get("description", ""),
            homepage=pkg_data.get("homepage", ""),
            source_url=pkg_data.get("source_url", ""),
            maintainer=pkg_data.get("maintainer", ""),
            build_date=pkg_data.get("build_date", ""),
            categories=pkg_data.get("categories", []),
            tags=pkg_data.get("tags", []),
            size=PackageSize(**pkg_data.get("size", {})),
            flags=PackageFlags(**pkg_data.get("flags", {})),
        )

        ver_data = data.get("verification", {})
        verification = ManifestVerification(
            manifest_hash=ver_data.get("manifest_hash", ""),
            content_tree=ver_data.get("content_tree", ""),
            signature_algo=ver_data.get("signature_algo", "ed25519"),
        )

        dep_data = data.get("dependencies", {})
        dependencies = ManifestDependencies(
            requires=dep_data.get("requires", []),
            suggests=dep_data.get("suggests", []),
            conflicts=dep_data.get("conflicts", []),
            provides=dep_data.get("provides", []),
            before=dep_data.get("before", []),
            after=dep_data.get("after", []),
        )

        inst_data = data.get("install", {})
        install = ManifestInstall(
            mode=inst_data.get("mode", "both"),
            config_files=inst_data.get("config_files", []),
            desktop_files=inst_data.get("desktop_files", []),
        )

        repo_data = data.get("repository", {})
        repository = ManifestRepository(
            origin_repo=repo_data.get("origin_repo", ""),
            origin_url=repo_data.get("origin_url", ""),
            fetched_at=repo_data.get("fetched_at", ""),
        )

        compat_data = data.get("compat", {})
        compat = ManifestCompat(
            min_fpm_version=compat_data.get("min_fpm_version", "0.1.0"),
            fpkg_format=compat_data.get("fpkg_format", "1"),
        )

        return cls(
            package=pkg,
            verification=verification,
            dependencies=dependencies,
            install=install,
            repository=repository,
            compat=compat,
        )

    @classmethod
    def from_toml(cls, data: bytes) -> Manifest:
        return cls.from_dict(tomllib.loads(data.decode()))

    @classmethod
    def from_file(cls, path: str) -> Manifest:
        with open(path, "rb") as f:
            return cls.from_toml(f.read())
