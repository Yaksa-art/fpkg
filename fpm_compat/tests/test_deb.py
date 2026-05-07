import io
import os
import struct
import tarfile
import tempfile
import textwrap
from pathlib import Path

import pytest
from fpm_compat.deb import DebConverter, _parse_ar, _parse_control, _split_deps


def _make_tar_gz(files: dict[str, bytes]) -> bytes:
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tf:
        for name, content in files.items():
            info = tarfile.TarInfo(name=name)
            info.size = len(content)
            tf.addfile(info, io.BytesIO(content))
    return buf.getvalue()


def _make_ar(members: dict[str, bytes]) -> bytes:
    out = bytearray(b"!<arch>\n")
    for name, content in members.items():
        header = (
            name.ljust(16)[:16]
            + "0".ljust(12)
            + "0".ljust(6)
            + "0".ljust(6)
            + "100644".ljust(8)
            + str(len(content)).ljust(10)
            + "`\n"
        )
        out += header.encode("ascii")
        out += content
        if len(content) % 2:
            out += b"\n"
    return bytes(out)


def _make_deb(name="mypkg", version="1.2.3", depends="libc") -> bytes:
    control_content = textwrap.dedent(f"""\
        Package: {name}
        Version: {version}
        Architecture: amd64
        Depends: {depends}
        Description: Test package
    """).encode()
    control_tar = _make_tar_gz({"./control": control_content})
    data_tar = _make_tar_gz({"./usr/bin/hello": b"#!/bin/sh\necho hello\n"})
    return _make_ar({
        "debian-binary": b"2.0\n",
        "control.tar.gz": control_tar,
        "data.tar.gz": data_tar,
    })


def test_parse_control():
    raw = b"Package: foo\nVersion: 1.0\nDepends: bar, baz\n"
    c = _parse_control(raw)
    assert c["package"] == "foo"
    assert c["version"] == "1.0"


def test_split_deps():
    assert _split_deps("libc6 (>= 2.17), libssl3") == ["libc6", "libssl3"]
    assert _split_deps("a | b") == ["a"]
    assert _split_deps("") == []


def test_full_conversion():
    deb = _make_deb(name="hello", version="2.0", depends="libc6")
    with tempfile.TemporaryDirectory() as tmp:
        src = os.path.join(tmp, "hello.deb")
        out = os.path.join(tmp, "hello.fpkg")
        Path(src).write_bytes(deb)
        result = DebConverter(src, out, None).convert()
        assert result == out
        assert os.path.exists(out)
        from fpkg_lib.package import FpkgReader
        with FpkgReader(out) as r:
            ok, errs = r.verify()
            assert ok, errs
            m = r.manifest()
            assert m.package.name == "hello"
            assert m.package.version == "2.0"
            assert m.package.arch == "x86_64"
            assert "libc6" in m.dependencies.requires
            assert r.compat_origin() == "deb"
