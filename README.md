# fpkg — FSociety Package Manager

Native package format, cryptographic verification, dependency resolution and build tooling for FSocietyOS.

## Architecture

The package manager is split into focused modules. Each module is an independent crate / library that is linked or called via a defined interface.

| Module | Language | Status | Responsibility |
|--------|----------|--------|----------------|
| **M1 fpm-solver** | Rust | ✅ implemented | Dependency resolution (pubgrub SAT solver), conflict reporting, virtual-package aliases |
| **M3 fpm-verifier** | C++20 | ✅ implemented | Ed25519 signature verification, BLAKE3 Merkle tree, per-file checksums, PKI chain |
| **M8 fpm-db** | — | planned | Installed-package database |
| **M10 fpm-builder** | — | planned | Build packages from PKGBUILD.toml |
| **M2 fpm-fetcher** | Rust | planned | Download packages & signatures from repo mirrors |
| **M5 fpm-installer** | Rust | planned | Unpack, run scripts, record in M8 DB |

The `fpkg` Python CLI (root of this repo) provides package inspection, verification, and creation for the `.fpkg` archive format.

---

## .fpkg Archive Format

A `.fpkg` file is a zstd-compressed TAR archive with the following layout:

```
package.fpkg  (tar.zst)
├── META/
│   ├── manifest.toml       # Package metadata and dependency declarations
│   ├── checksums.blake3    # "<blake3-hex>  <relative-path>" for every DATA/ file
│   ├── content_tree.txt    # Single line: BLAKE3 Merkle root of DATA/
│   ├── signature.ed25519   # Ed25519 detached signature over manifest.toml (64 bytes raw)
│   └── scripts/
│       ├── pre-install.sh
│       └── post-install.sh
└── DATA/                   # Installed files (mirrors filesystem root)
    └── usr/
        └── ...
```

---

## M1 — Dependency Solver (`fpm-solver/`)

Rust library + CLI that resolves the full install set for a package given a local package index.

### Features

- Parses `manifest.toml` — reads `[dependencies.requires]` in both string (`"libfoo >= 1.2.0"`) and table form
- Supports `provides` virtual names (e.g. `libc` resolved to `glibc` or `musl`)
- `conflicts` declarations cause the solver to produce a clear error
- Optional dependencies are excluded from resolution unless explicitly requested
- Human-readable conflict reports via `pubgrub`

### Build

```sh
cd fpm-solver
cargo build --release
```

### Usage

```sh
# Resolve all dependencies for a package
fpm-solver resolve --manifest ./path/to/manifest.toml --index ./repo/index

# Validate that a manifest.toml is well-formed
fpm-solver check --manifest ./path/to/manifest.toml
```

---

## M3 — Verifier (`fpm-verifier/`)

C++20 static library + CLI that cryptographically verifies a `.fpkg` before installation.

### Verification pipeline

1. **Ed25519** — signature over `META/manifest.toml` checked against package public key
2. **Per-file checksums** — every file in `DATA/` verified against `META/checksums.blake3`
3. **Merkle root** — BLAKE3 Merkle tree rebuilt from `DATA/` and compared to `META/content_tree.txt`
4. **PKI chain** (optional) — package public key verified against repo root key via `chain_sig`

### Dependencies

- CMake 3.20+, GCC 12+ / Clang 15+ with C++20
- `libsodium` (`libsodium-dev`)
- BLAKE3 C sources vendored in `vendor/blake3/` (see `BUILD.md`)

### Build

```sh
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)
```

Outputs:
- `build/libfpm_verifier.a` — static library for FFI
- `build/fpm-verifier` — CLI binary

### CLI usage

```sh
# Verify a fully extracted .fpkg directory
fpm-verifier package  <extracted-dir>  <pubkey>

# Recompute and compare Merkle root
fpm-verifier merkle   <data-dir>  <expected-root-hex>

# Verify per-file checksums
fpm-verifier checksum <data-dir>  <checksums.blake3>

# Verify PKI chain (repo root signed package pubkey)
fpm-verifier pki      <root-pubkey>  <pkg-pubkey>  <chain-sig>

# Print BLAKE3 hash of any file
fpm-verifier hash     <file>
```

### FFI from Rust

The C API is declared in `include/fpm_verifier.h`. Link from Rust via `build.rs`:

```rust
println!("cargo:rustc-link-lib=static=fpm_verifier");
println!("cargo:rustc-link-search=native=../fpm-verifier/build");
println!("cargo:rustc-link-lib=sodium");
```

Error codes returned from all `fpm_verify_*` functions:

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `FPM_OK` | Success |
| 1 | `FPM_ERR_SIGNATURE` | Ed25519 verification failed |
| 2 | `FPM_ERR_MERKLE` | Merkle root mismatch |
| 3 | `FPM_ERR_CHECKSUM` | Per-file checksum mismatch |
| 4 | `FPM_ERR_PKI` | PKI chain verification failed |
| 5 | `FPM_ERR_IO` | File I/O error |
| 6 | `FPM_ERR_INVALID_INPUT` | Bad key/signature length |

---

## fpkg — Package Inspector CLI

Python 3.11+ tool for inspecting, verifying, and creating `.fpkg` files.

### Requirements

```sh
pip install blake3 tomli-w
```

### Commands

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

---

## Repository layout

```
fpkg/
├── fpkg               # Python package inspector CLI
├── fpkg-build         # Python package builder
├── fpm-solver/        # M1 — Rust dependency solver
│   ├── src/
│   │   ├── types.rs   # Version, VersionReq, Dep primitives
│   │   ├── manifest.rs
│   │   ├── index.rs
│   │   ├── solver.rs  # pubgrub integration
│   │   └── main.rs
│   └── Cargo.toml
└── fpm-verifier/      # M3 — C++20 cryptographic verifier
    ├── include/
    │   ├── fpm_verifier.h    # C API for FFI
    │   └── fpm_verifier.hpp  # C++ API
    ├── src/
    │   ├── blake3_hasher.cpp
    │   ├── merkle.cpp
    │   ├── ed25519.cpp
    │   ├── checksum_file.cpp
    │   ├── pki.cpp
    │   ├── verifier.cpp
    │   ├── c_api.cpp
    │   └── main.cpp
    ├── tests/
    ├── vendor/blake3/
    ├── CMakeLists.txt
    └── BUILD.md
```
