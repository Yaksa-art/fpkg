from __future__ import annotations

import io
import struct
import tarfile
import cpio
from pathlib import Path
from typing import Optional

from fpkg_lib.manifest import (
    Manifest, ManifestPackage, ManifestDependencies,
    ManifestInstall, ManifestCompat,
)
from fpkg_lib.package import FpkgWriter
from .arch import normalise as norm_arch

RPM_MAGIC = b"\xed\xab\xee\xdb"

TAG_NAME        = 1000
TAG_VERSION     = 1001
TAG_RELEASE     = 1002
TAG_SUMMARY     = 1004
TAG_DESCRIPTION = 1005
TAG_ARCH        = 1022
TAG_LICENSE     = 1014
TAG_URL         = 1020
TAG_REQUIRENAME = 1049
TAG_CONFLICTNAME = 1054
TAG_PROVIDENAME  = 1047
TAG_SUGGESTNAME  = 5049


def _read_header(data: bytes, offset: int) -> tuple[dict[int, object], int]:
    magic = data[offset:offset+3]
    assert magic == b"\x8e\xad\xe8", f"Bad header magic at {offset:#x}"
    offset += 3
    offset += 1
    offset += 4
    nindex = struct.unpack_from(">I", data, offset)[0]; offset += 4
    hsize  = struct.unpack_from(">I", data, offset)[0]; offset += 4

    index_start = offset
    store_start = offset + nindex * 16

    tags: dict[int, object] = {}
    for i in range(nindex):
        base = index_start + i * 16
        tag   = struct.unpack_from(">I", data, base)[0]
        ttype = struct.unpack_from(">I", data, base+4)[0]
        off   = struct.unpack_from(">I", data, base+8)[0]
        count = struct.unpack_from(">I", data, base+12)[0]
        tags[tag] = _decode_tag(data, store_start, ttype, off, count)

    return tags, store_start + hsize


def _decode_tag(data: bytes, store: int, ttype: int, off: int, count: int) -> object:
    base = store + off
    if ttype == 6:
        end = data.index(b"\x00", base)
        return data[base:end].decode(errors="replace")
    if ttype == 8 or ttype == 9:
        strings = []
        pos = base
        for _ in range(count):
            end = data.index(b"\x00", pos)
            strings.append(data[pos:end].decode(errors="replace"))
            pos = end + 1
        return strings
    return None


def _str(tags: dict, key: int, default: str = "") -> str:
    v = tags.get(key, default)
    if isinstance(v, list):
        return v[0] if v else default
    return v or default


def _lst(tags: dict, key: int) -> list[str]:
    v = tags.get(key, [])
    if isinstance(v, list):
        return [x for x in v if x]
    return [v] if v else []


class RpmConverter:
    def __init__(self, src: str, out: Optional[str], arch_override: Optional[str]):
        self.src = src
        self.out = out
        self.arch_override = arch_override

    def convert(self) -> str:
        raw = Path(self.src).read_bytes()
        assert raw[:4] == RPM_MAGIC, "Not an RPM file"

        offset = 8
        _, offset = _read_header(raw, offset)

        tags, payload_offset = _read_header(raw, offset)

        name    = _str(tags, TAG_NAME, Path(self.src).stem)
        version = _str(tags, TAG_VERSION, "0.0.0")
        arch    = self.arch_override or norm_arch(_str(tags, TAG_ARCH, "x86_64"))

        manifest = Manifest(
            package=ManifestPackage(
                name=name,
                version=version,
                arch=arch,
                license=_str(tags, TAG_LICENSE),
                summary=_str(tags, TAG_SUMMARY),
                description=_str(tags, TAG_DESCRIPTION),
                homepage=_str(tags, TAG_URL),
            ),
            dependencies=ManifestDependencies(
                requires=_lst(tags, TAG_REQUIRENAME),
                suggests=_lst(tags, TAG_SUGGESTNAME),
                conflicts=_lst(tags, TAG_CONFLICTNAME),
                provides=_lst(tags, TAG_PROVIDENAME),
            ),
            install=ManifestInstall(mode="system"),
            compat=ManifestCompat(),
        )

        payload = raw[payload_offset:]
        data_files = self._extract_cpio(payload)

        out_path = self.out or str(Path(self.src).with_suffix(".fpkg"))
        writer = FpkgWriter(out_path)
        writer.set_manifest(manifest)
        writer.set_origin("rpm")

        for rel_path, content in data_files.items():
            writer.add_data_file(rel_path, content)

        writer.write()
        return out_path

    def _extract_cpio(self, payload: bytes) -> dict[str, bytes]:
        import gzip, lzma, zlib

        if payload[:2] == b"\x1f\x8b":
            payload = gzip.decompress(payload)
        elif payload[:6] == b"\xfd7zXZ\x00":
            payload = lzma.decompress(payload)
        elif payload[:4] == b"\x28\xb5\x2f\xfd":
            import zstandard
            payload = zstandard.ZstdDecompressor().decompress(payload)

        return self._parse_cpio_newc(payload)

    def _parse_cpio_newc(self, data: bytes) -> dict[str, bytes]:
        files: dict[str, bytes] = {}
        pos = 0
        while pos + 110 <= len(data):
            if data[pos:pos+6] != b"070701":
                break
            namesize = int(data[pos+94:pos+102], 16)
            filesize = int(data[pos+54:pos+62], 16)
            pos += 110
            name_raw = data[pos:pos+namesize].rstrip(b"\x00").decode(errors="replace")
            pos += namesize
            pos = (pos + 3) & ~3
            if name_raw in (".", "TRAILER!!!"):
                pos += filesize
                pos = (pos + 3) & ~3
                continue
            content = data[pos:pos+filesize]
            pos += filesize
            pos = (pos + 3) & ~3
            rel = name_raw.lstrip("./")
            if rel:
                files[rel] = content
        return files
