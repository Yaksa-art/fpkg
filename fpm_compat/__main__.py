from __future__ import annotations

import argparse
import sys
from .converter import convert, detect_format


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="fpm-compat",
        description="M9 — convert .deb/.rpm/.apk packages to .fpkg",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_conv = sub.add_parser("convert", help="convert a package")
    p_conv.add_argument("src", help="source package file (.deb / .rpm / .apk)")
    p_conv.add_argument("--out", default=None, help="output .fpkg path")
    p_conv.add_argument("--arch", default=None, help="override target arch")

    p_det = sub.add_parser("detect", help="detect package format")
    p_det.add_argument("src")

    args = parser.parse_args()

    if args.cmd == "detect":
        print(detect_format(args.src))
        return

    try:
        out = convert(args.src, out=args.out, arch_override=args.arch)
        print(f"ok: {out}")
    except Exception as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
