# fpm-compat — M9

Converts foreign packages to `.fpkg`.

## Supported formats

| Format | Extension | Backend |
|--------|-----------|-------------------|
| Debian | `.deb` | `fpm_compat.deb` |
| RPM | `.rpm` | `fpm_compat.rpm` |
| Alpine | `.apk` | `fpm_compat.apk` |

## Usage

```sh
fpm-compat convert package.deb
fpm-compat convert package.rpm --out /tmp/out.fpkg
fpm-compat convert package.apk --arch aarch64
```

## Python API

```python
from fpm_compat import convert
out = convert("package.deb")
print(out) # /path/to/package.fpkg
```

## Notes

- Dependency names are preserved verbatim from the source format.
- Architecture is normalised: `amd64` → `x86_64`, `arm64` → `aarch64`.
- Scripts (`preinst`, `postinst`, `prerm`, `postrm`) are carried over when
  present.
- The resulting `.fpkg` has `COMPAT/origin_format.txt` set to the source
  format (`deb`, `rpm`, or `apk`).
