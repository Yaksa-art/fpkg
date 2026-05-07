from __future__ import annotations

import os
from pathlib import Path


def detect_format(path: str) -> str:
    p = Path(path)
    suffix = p.suffix.lower()
    if suffix == ".deb":
        return "deb"
    if suffix == ".rpm":
        return "rpm"
    if suffix == ".apk":
        return "apk"
    with open(path, "rb") as f:
        magic = f.read(8)
    if magic[:2] == b"!<":
        return "deb"
    if magic[:4] == b"\xed\xab\xee\xdb":
        return "rpm"
    if magic[:2] == b"\x1f\x8b":
        return "apk"
    raise ValueError(f"Cannot detect package format for: {path}")


def convert(src: str, out: str | None = None, arch_override: str | None = None) -> str:
    fmt = detect_format(src)
    if fmt == "deb":
        from .deb import DebConverter
        return DebConverter(src, out, arch_override).convert()
    if fmt == "rpm":
        from .rpm import RpmConverter
        return RpmConverter(src, out, arch_override).convert()
    if fmt == "apk":
        from .apk import ApkConverter
        return ApkConverter(src, out, arch_override).convert()
    raise ValueError(f"Unsupported format: {fmt}")
