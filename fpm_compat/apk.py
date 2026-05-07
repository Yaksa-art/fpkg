from __future__ import annotations

import gzip
import io
import tarfile
from pathlib import Path
from typing import Optional

from fpkg_lib.manifest import (
    Manifest, ManifestPackage, ManifestDependencies,
    ManifestInstall, ManifestCompat,
)
from fpkg_lib.package import FpkgWriter
from .arch import normalise as norm_arch


def _split_gzip_streams(raw: bytes) -> list[bytes]:
    streams = []
    pos = 0
    while pos < len(raw):
        if raw[pos:pos+2] != b"\x1f\x8b":
            break
        end = pos + 10
        while end < len(raw) - 1:
            if raw[end:end+2] == b"\x1f\x8b":
                break
            end += 1
        else:
            end = len(raw)
        streams.append(raw[pos:end])
        pos = end
    return streams


def _read_tar_stream(data: bytes) -> dict[str, bytes]:
    try:
        raw = gzip.decompress(data)
    except Exception:
        raw = data
    files: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:") as tf:
        for member in tf.getmembers():
            if not member.isfile():
                continue
            f = tf.extractfile(member)
            if f:
                files[member.name.lstrip("./")] = f.read()
    return files


def _parse_pkginfo(raw: bytes) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in raw.decode(errors="replace").splitlines():
        if line.startswith("#"):
            continue
        if " = " in line:
            k, _, v = line.partition(" = ")
            fields[k.strip()] = v.strip()
    return fields


def _pkginfo_list(info: dict[str, str], key: str) -> list[str]:
    val = info.get(key, "")
    if not val:
        return []
    return [x.strip().split(">")[0].split("=")[0].strip() for x in val.split() if x.strip()]


class ApkConverter:
    def __init__(self, src: str, out: Optional[str], arch_override: Optional[str]):
        self.src = src
        self.out = out
        self.arch_override = arch_override

    def convert(self) -> str:
        raw = Path(self.src).read_bytes()
        streams = _split_gzip_streams(raw)

        meta_files: dict[str, bytes] = {}
        data_files: dict[str, bytes] = {}

        if len(streams) >= 1:
            meta_files = _read_tar_stream(streams[0])
        if len(streams) >= 2:
            data_files = _read_tar_stream(streams[1])
        if len(streams) >= 3:
            data_files.update(_read_tar_stream(streams[2]))

        pkginfo_raw = meta_files.get(".PKGINFO", b"")
        info = _parse_pkginfo(pkginfo_raw)

        arch = self.arch_override or norm_arch(info.get("arch", "x86_64"))
        name = info.get("pkgname", Path(self.src).stem)
        version = info.get("pkgver", "0.0.0")
        version = version.split("-r")[0]

        manifest = Manifest(
            package=ManifestPackage(
                name=name,
                version=version,
                arch=arch,
                license=info.get("license", ""),
                summary=info.get("pkgdesc", ""),
                description=info.get("pkgdesc", ""),
                homepage=info.get("url", ""),
                maintainer=info.get("maintainer", ""),
            ),
            dependencies=ManifestDependencies(
                requires=_pkginfo_list(info, "depend"),
                suggests=_pkginfo_list(info, "install_if"),
                provides=_pkginfo_list(info, "provides"),
            ),
            install=ManifestInstall(mode="system"),
            compat=ManifestCompat(),
        )

        out_path = self.out or str(Path(self.src).with_suffix(".fpkg"))
        writer = FpkgWriter(out_path)
        writer.set_manifest(manifest)
        writer.set_origin("apk")

        for rel_path, content in data_files.items():
            if rel_path.startswith(".SIGN") or rel_path == ".PKGINFO":
                continue
            writer.add_data_file(rel_path, content)

        for script_name, hook_name in [
            (".pre-install", "pre-install.sh"),
            (".post-install", "post-install.sh"),
            (".pre-deinstall", "pre-remove.sh"),
            (".post-deinstall", "post-remove.sh"),
        ]:
            if script_name in meta_files:
                writer.set_script(hook_name, meta_files[script_name])

        writer.write()
        return out_path
