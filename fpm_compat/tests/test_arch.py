import pytest
from fpm_compat.arch import normalise


def test_amd64():
    assert normalise("amd64") == "x86_64"

def test_arm64():
    assert normalise("arm64") == "aarch64"

def test_noarch():
    assert normalise("noarch") == "any"

def test_passthrough():
    assert normalise("riscv64") == "riscv64"

def test_unknown_defaults_to_x86_64():
    assert normalise("mips") == "x86_64"
