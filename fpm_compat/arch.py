from __future__ import annotations

ARCH_MAP: dict[str, str] = {
    "amd64": "x86_64",
    "x86_64": "x86_64",
    "arm64": "aarch64",
    "aarch64": "aarch64",
    "riscv64": "riscv64",
    "noarch": "any",
    "all": "any",
    "any": "any",
    "x86": "x86_64",
    "i386": "x86_64",
    "i686": "x86_64",
}


def normalise(arch: str) -> str:
    return ARCH_MAP.get(arch.lower(), "x86_64")
