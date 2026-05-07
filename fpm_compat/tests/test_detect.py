import os
import tempfile
from pathlib import Path

import pytest
from fpm_compat.converter import detect_format


def test_detect_by_extension():
    with tempfile.NamedTemporaryFile(suffix=".deb", delete=False) as f:
        f.write(b"!<arch>\n")
        name = f.name
    try:
        assert detect_format(name) == "deb"
    finally:
        os.unlink(name)


def test_detect_rpm_magic():
    with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as f:
        f.write(b"\xed\xab\xee\xdb" + b"\x00" * 100)
        name = f.name
    try:
        assert detect_format(name) == "rpm"
    finally:
        os.unlink(name)


def test_detect_unknown_raises():
    with tempfile.NamedTemporaryFile(suffix=".xyz", delete=False) as f:
        f.write(b"hello world")
        name = f.name
    try:
        with pytest.raises(ValueError):
            detect_format(name)
    finally:
        os.unlink(name)
