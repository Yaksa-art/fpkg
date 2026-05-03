# fpkg

FSociety Package Manager — native package format and build tooling for FSocietyOS.

## Requirements

- Python 3.11+
- `blake3` — `pip install blake3`
- `tomli-w` — `pip install tomli-w`

## Tools

| Tool | Purpose |
|---|---|
| `fpkg` | Inspect, verify, and extract `.fpkg` archives |
| `fpkg-build` | Build `.fpkg` packages from `PKGBUILD.toml` |

## fpkg — Package Inspector

```sh
# Show package metadata
./fpkg info package.fpkg

# Verify checksums
./fpkg verify package.fpkg

# List archive contents
./fpkg inspect package.fpkg

# Print raw manifest.toml
./fpkg manifest package.fpkg

# Extract DATA/ to a directory
./fpkg extract package.fpkg --dest ./out

# Create a minimal .fpkg from a directory
./fpkg create --name myapp --version 1.0.0 --data ./dist --output myapp.fpkg
```

## fpkg-build — Package Builder (M10)

Create a `PKGBUILD.toml` in your project directory:

```toml
[package]
name        = "my-tool"
version     = "1.0.0"
release     = 1
arch        = "x86_64"
license     = "MIT"
summary     = "My amazing tool"
maintainer  = "Your Name <email@example.com>"

[source]
local = "./src"

[build]
build_depends = ["gcc", "make"]
script = """
make PREFIX=/usr
"""

[package_install]
mode   = "both"
script = """
make PREFIX=/usr DESTDIR="$FPM_DESTDIR" install
"""

[runtime]
requires = ["glibc >=2.35"]
```

Then build:

```sh
./fpkg-build PKGBUILD.toml
# [*] Building my-tool 1.0.0-1 (x86_64)
# [✓] Source: ./src
# [✓] Build complete
# [✓] Install script complete
# [*] Packing N file(s)...
# [✓] Package created: my-tool-1.0.0-1-x86_64.fpkg

# Validate without building
./fpkg-build PKGBUILD.toml --dry-run

# Custom output directory
./fpkg-build PKGBUILD.toml --output-dir ./dist
```

### Build environment variables

| Variable | Value |
|---|---|
| `FPM_SRCDIR` | Directory where sources are placed |
| `FPM_DESTDIR` | Staging directory — install files here |
| `FPM_NAME` | Package name |
| `FPM_VERSION` | Package version |
| `FPM_ARCH` | Target architecture |

## .fpkg Format

`.fpkg` is a ZIP archive with the following structure:

```
package.fpkg
├── META/
│   ├── manifest.toml        # Package metadata (name, version, deps, checksums)
│   ├── checksums.blake3     # BLAKE3 hash of every file in DATA/
│   ├── dependencies.toml    # Dependency graph
│   ├── changelog.md         # Version history
│   └── scripts/
│       ├── pre-install.sh
│       ├── post-install.sh
│       ├── pre-remove.sh
│       └── post-remove.sh
├── DATA/
│   └── usr/
│       ├── bin/
│       ├── lib/
│       └── share/
└── COMPAT/
    └── origin_format.txt    # "native" | "deb" | "rpm" | "apk" | ...
```

### manifest.toml fields

| Section | Key | Description |
|---|---|---|
| `[package]` | `name` | Package identifier |
| `[package]` | `version` | Semantic version |
| `[package]` | `release` | Build number |
| `[package]` | `arch` | `x86_64` / `aarch64` / `riscv64` / `any` |
| `[package]` | `license` | SPDX license identifier |
| `[package]` | `summary` | One-line description |
| `[package]` | `maintainer` | `Name <email>` |
| `[package.size]` | `installed` | Bytes on disk after install |
| `[package.size]` | `compressed` | Bytes in archive |
| `[package.flags]` | `has_services` | Contains systemd units |
| `[package.flags]` | `has_suid` | Contains SUID/SGID files |
| `[verification]` | `manifest_hash` | BLAKE3 hash of this manifest |
| `[verification]` | `content_tree` | BLAKE3 Merkle root of DATA/ |
| `[verification]` | `signature_algo` | `ed25519` |
| `[dependencies]` | `requires` | Runtime requirements |
| `[dependencies]` | `suggests` | Optional enhancements |
| `[dependencies]` | `conflicts` | Incompatible packages |
| `[dependencies]` | `provides` | Virtual package names |
| `[install]` | `mode` | `system` / `user` / `both` |
| `[install]` | `config_files` | Paths that survive upgrades |
| `[compat]` | `min_fpm_version` | Minimum fpkg version required |

## Example

```sh
# Build the included example
cd example
../fpkg-build PKGBUILD.toml

# Inspect it
../fpkg info hello-world-1.0.0-1-x86_64.fpkg
../fpkg verify hello-world-1.0.0-1-x86_64.fpkg
../fpkg inspect hello-world-1.0.0-1-x86_64.fpkg
```
