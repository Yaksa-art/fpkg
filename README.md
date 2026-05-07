<div align="center">
  <img width="1080" height="589" alt="FSociety Package Manager" src="https://github.com/user-attachments/assets/fd7cacad-77d5-4c2b-b2c0-bc9a576ec347" />

  <h1>fpm — FSociety Package Manager</h1>

  <p>
    <a href="https://github.com/Yaksa-art/fpkg"><img src="https://sloc.xyz/github/Yaksa-art/fpkg/?category=code" alt="Code"></a>
    <a href="https://github.com/Yaksa-art/fpkg/"><img src="https://sloc.xyz/github/Yaksa-art/fpkg/?category=lines" alt="Lines"></a>
    <a href="https://www.gnu.org/licenses/gpl-3.0"><img src="https://img.shields.io/badge/License-GPL%203.0-blue.svg?style=flat-square" alt="License"></a>
    <a href="https://github.com/Yaksa-art/fpkg/issues"><img src="https://img.shields.io/github/issues/Yaksa-art/fpkg?style=flat-square&logo=github" alt="Issues"></a>
  </p>

  <p><b>The foundational package manager for FSocietyOS. Built from scratch for atomicity, cryptographic security, and dual-mode (system/user) operations.</b></p>
</div>

---

## 📖 Core Philosophy

`fpm` is not just a utility; it is the core of FSocietyOS's independence. We looked at APT, DNF, Pacman, and APK, and built a system that solves their historical flaws without relying on heavy containerization like Flatpak.

* **🛡️ Secure by Default:** Every package, file, and index is cryptographically verified (Ed25519 signatures, BLAKE3 Merkle trees, and checksums).
* **⚛️ Atomic Transactions:** Installations either complete fully or not at all. Aborted updates will never leave your system in a broken state. `fpm rollback` is supported natively.
* **👥 Dual-Mode (System & User):** Run as `root` for global installs (`/usr`), or safely as a normal user (`~/.local/fpm`) with user-namespace overlay isolation.
* **📦 Content Addressing:** Files are identified by their BLAKE3 content hash.
* **⚡ Blazing Fast:** Async parallel downloading, PubGrub SAT dependency resolution, HTTP ETag caching, and MessagePack-based binary indexes.
* **🔗 Universal Compatibility:** Native `.fpkg` format, but features an integrated compatibility layer to convert and install `.deb`, `.rpm`, `.apk`, and `.pkg.tar.zst` seamlessly.

---

<img width="1328" height="744" alt="Untitled" src="https://github.com/user-attachments/assets/6d00cb84-5971-4b80-8119-1036edc991d6" />


### Module Status

All foundational modules are actively implemented in Rust (with C++20 for cryptography).

| Module | Component | Lang | Status | Responsibility |
|---|---|---|---|---|
| **M1** | `fpm-solver` | Rust | ✅ | Dependency resolution (PubGrub SAT), conflicts, virtual packages |
| **M2** | `fpm-fetcher` | Rust | ✅ | Async parallel download, HTTP Range resume, ETag caching |
| **M3** | `fpm-verifier`| C++20 | ✅ | Ed25519 signatures, BLAKE3 Merkle trees, PKI validation |
| **M4** | `fpm-core` | Rust | ✅ | Atomic CoW generation snapshots, Rollbacks, Transaction state |
| **M5** | `fpm-installer`| Rust | ✅ | Extract `DATA/`, layout fixups, file manifests, conflicts |
| **M6** | `fpm-index` | Rust | ✅ | Repo index sync (delta, ETag, MessagePack) |
| **M7** | `fpm-hooks` | Rust | ✅ | Pre/post-install script runner, timeouts |
| **M8** | `fpm-db` | Rust | ✅ | SQLite state (packages, files, generations, holds, r2d2 pool) |
| **M9** | `fpm-compat` | Rust | ✅ | Convert foreign formats (`.deb`, `.rpm`, `.apk`, Arch `.zst`) |
| **M10**| `fpkg-build` | Rust | ✅ | Build `.fpkg` archives from `PKGBUILD.toml` |
| **M11**| `fpm-sandbox` | Rust | ✅ | User-namespace overlay, `bwrap`, isolation for user-mode |
| **Daemon**| `fpmd` | Rust | ✅ | JSON-RPC 2.0 orchestrator over Unix sockets |

---

## 📦 The `.fpkg` Archive Format

A `.fpkg` file is a `tar.zst` (Zstandard compressed tarball) structured to allow arbitrary file extraction and immediate cryptographic validation.

```text
package.fpkg
├── META/
│   ├── manifest.toml        # Package name, version, architecture, dependencies
│   ├── signature.ed25519    # Detached Ed25519 signature of manifest.toml
│   ├── checksums.blake3     # "<blake3-hex>  <rel-path>" for every file
│   ├── content_tree.txt     # BLAKE3 Merkle root of the DATA/ directory
│   ├── dependencies.toml    # Detailed dependency graph
│   └── scripts/
│       ├── pre-install.sh   # Run before extraction (sandboxed)
│       └── post-install.sh  # Run after extraction (sandboxed)
├── DATA/                    # Actual installed files
│   ├── usr/bin/...
│   └── etc/...
└── COMPAT/                  # (Optional) Retained metadata from converted packages
    └── origin_format.txt    # "native", "deb", "rpm", "apk", etc.
```

---

## 👥 Dual-Mode: System vs. User Installations

`fpm` is designed from day one to operate without requiring `root` for isolated application installs. 

### System Mode (`sudo fpm install`)
* **Files:** Installed globally (`/usr`, `/etc`, `/var`).
* **Database:** `/var/lib/fpm/db.sqlite`
* **Target:** Core OS packages, drivers, global utilities.

### User Mode (`fpm install --user`)
* **Files:** Installed in `~/.local/fpm/packages/<name>/` and symlinked to `~/.local/bin/`.
* **Database:** `~/.local/share/fpm/db.sqlite`
* **Sandbox (M11):** Configurable isolation (None, Overlay, Bubblewrap, Full Container).
* **Target:** User applications, IDEs, web browsers, safely contained.

If a package exists in both scopes, the User version takes priority via the `$PATH` environment variable.

---

## ⚙️ Modules Deep Dive

* **M1 Solver**: Uses the PubGrub algorithm (same as Dart/Cargo) ensuring fast, conflict-aware resolution without the historical slowdowns of `apt`'s SAT solver.
* **M3 Verifier (C++20)**: Rebuilds a Merkle tree from the `DATA/` directory on-the-fly and compares it against `META/content_tree.txt` signed by the maintainer's Ed25519 key. Fast, using SIMD-accelerated BLAKE3.
* **M4 Transaction Manager**: Creates atomic "Generations". Every install/remove acts on a `pending/` snapshot, effectively enabling `fpm rollback --to <gen>` if an update breaks your system.
* **M8 Database**: Maintains strict file-level ownership in a local SQLite DB, enabling instant queries like `fpm-db owns /usr/bin/firefox`.
* **M9 Compat Layer**: Transparently handles foreign packages. Run `fconv package.deb` and fpm translates the Debian `control` file into a `manifest.toml`, remaps dependencies via a compatibility dictionary, and outputs a clean `.fpkg`.
* **M11 Sandbox**: Wraps hook scripts and user-mode installs in namespaces using `bwrap` and `fuse-overlayfs` to prevent malicious scripts from modifying the host during installation.

---

## 🛠️ Additional Tools

Included in this repository are standalone developer tools:

* **`fconv`**: Converts `.deb`, `.rpm`, `.apk`, and Arch `.pkg.tar.zst` packages into `.fpkg`.
* **`fpkg-build`**: Compiles source code into a `.fpkg` by reading a declarative `PKGBUILD.toml` file.
* **`fpkg-sign`**: Key generation and signing utility for repository maintainers.
* **`fpmd`**: The background daemon. Listens on `/run/fpm/fpmd.sock` (or `/run/user/<uid>/fpm/fpmd.sock`) to coordinate all M1-M11 operations asynchronously.

---

## 🏗️ Building from Source

Ensure you have CMake 3.20+, a C++20 compiler (GCC/Clang), Rust (Cargo), and `libsodium` installed.

```sh
# 1. Build M3 Verifier (C++ Static Library)
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)

# 2. Build Rust crates (order matters for FFI linking)
cd ../fpm-solver    && cargo build --release
cd ../fpm-fetcher   && cargo build --release
cd ../fpm-core      && cargo build --release
cd ../fpm-installer && cargo build --release
cd ../fpm-db        && cargo build --release
cd ../fpm-index     && cargo build --release
cd ../fpm-sandbox   && cargo build --release
cd ../fpm-hooks     && cargo build --release
cd ../fpm-compat    && cargo build --release
cd ../fpkg-build    && cargo build --release
cd ../fpmd          && cargo build --release

# 3. Run Test Suites
cd ../fpm-verifier  && ctest --test-dir build --output-on-failure
cd ../fpm-solver    && cargo test
cd ../fpm-fetcher   && cargo test
cd ../fpm-core      && cargo test
cd ../fpm-installer && cargo test
cd ../fpm-db        && cargo test
```
