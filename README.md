# fpkg — FSociety Package Manager

Native package format, cryptographic verification, dependency resolution, async downloading and build tooling for FSocietyOS.

## Module Pipeline

```
fpm install firefox
       │
       ▼
  M1 fpm-solver     ── Resolve dependency graph (pubgrub SAT)
       │  returns Vec<ResolvedPackage>
       ▼
  M2 fpm-fetcher    ── Download .fpkg files (async/tokio, parallel, resume)
       │  calls ▼ after each download
  M3 fpm-verifier   ── Ed25519 + BLAKE3 Merkle + checksums + PKI chain (C++20 via FFI)
       │  returns Vec<FetchResult> with verified paths
       ▼
  M4 fpm-core/trx   ── Atomic generation snapshot, trx.begin()
       │  Transaction handle (root_dir, plan)
       ▼
  M5 fpm-installer  ── Extract DATA/, run hooks, write file manifest
       │  calls trx.commit() on success, trx.abort() on error
       ▼
  M8 fpm-db         ── (planned) SQLite record of installed packages
```

## Module Status

| Module | Language | Status | Responsibility |
|--------|----------|--------|----------------|
| **M1 fpm-solver** | Rust | ✅ implemented | Dependency resolution, conflict reporting, virtual-package aliases |
| **M2 fpm-fetcher** | Rust | ✅ implemented | Async download, parallel, HTTP Range resume, mirror failover, M3 FFI call |
| **M3 fpm-verifier** | C++20 | ✅ implemented | Ed25519, BLAKE3 Merkle tree, per-file checksums, PKI chain |
| **M4 fpm-core (trx)** | Rust | ✅ implemented | Atomic CoW generation snapshot, rollback, install plan |
| **M5 fpm-installer** | Rust | ✅ implemented | Extract DATA/, hooks, file manifest, conflict detection, remove |
| **M6 fpm-index** | Rust | planned | Repo index sync (delta, ETag, MessagePack) |
| **M7 fpm-hooks** | Rust/Shell | planned | Pre/post-install script runner, sandboxed via bwrap |
| **M8 fpm-db** | Rust + SQLite | planned | Installed packages, files, generations, holds |
| **M9 fpm-compat** | Python/Rust | planned | .deb / .rpm / .apk → .fpkg conversion |
| **M10 fpm-builder** | Rust/Shell | planned | Build from PKGBUILD.toml |
| **M11 fpm-sandbox** | C/Rust | planned | User-namespace overlay, seccomp |
| **M12 fconv** | Python | planned | Standalone format converter CLI |

The `fpkg` Python CLI (root of this repo) provides package inspection, verification, and creation for the `.fpkg` archive format.

---

## .fpkg Archive Format

A `.fpkg` file is a `tar.zst` archive with the following layout:

```
package.fpkg  (tar.zst)
├── META/
│   ├── manifest.toml        # package name, version, deps, flags
│   ├── checksums.blake3     # "<blake3-hex>  <rel-path>" per DATA/ file
│   ├── content_tree.txt     # single line: BLAKE3 Merkle root of DATA/
│   ├── signature.ed25519    # Ed25519 detached sig over manifest.toml (64 bytes raw)
│   └── scripts/
│       ├── pre-install.sh
│       └── post-install.sh
└── DATA/                    # installed files, mirrors filesystem root
    └── usr/bin/...
```

---

## M1 — Dependency Solver (`fpm-solver/`)

Rust library + CLI. Consumes `manifest.toml` and a package index, returns a `Vec<ResolvedPackage>` — the exact install set with resolved versions.

### Features

- Parses `manifest.toml` — string deps (`"libfoo >= 1.2.0"`) and table form
- `provides` virtual names (`libc` → `glibc` or `musl`)
- `conflicts` declarations with human-readable conflict reports (via `pubgrub`)
- Optional deps excluded from resolution unless explicitly requested

### Build & use

```sh
cd fpm-solver && cargo build --release

fpm-solver resolve --manifest ./manifest.toml --index ./repo/index
fpm-solver check   --manifest ./manifest.toml
```

---

## M2 — Fetcher (`fpm-fetcher/`)

Rust async library + CLI. Receives `Vec<ResolvedPackage>` from M1, downloads `.fpkg` files from configured mirrors in parallel, then calls **M3 Verifier via C FFI** on each downloaded archive.

### Features

| Feature | Detail |
|---------|--------|
| Parallel downloads | Bounded by `parallel_downloads` (default 4), tokio `Semaphore` |
| HTTP Range resume | `.part` files survive interruptions |
| Mirror failover | Mirrors ranked by latency; next mirror tried on error |
| ETag caching | 304 avoids re-download of unchanged packages |
| M3 FFI call | `fpm_verify_package()` called after each download |
| Progress events | `mpsc::Sender<ProgressEvent>` — JSON-serialisable |

### Build

```sh
cd ../fpm-verifier && cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build -j$(nproc)
cd ../fpm-fetcher  && cargo build --release
```

---

## M3 — Verifier (`fpm-verifier/`)

C++20 static library (`libfpm_verifier.a`) + CLI. Called by M2 via FFI after each download.

### Verification pipeline

1. **Ed25519** — `META/signature.ed25519` verified over `manifest.toml`
2. **Per-file checksums** — every `DATA/` file vs `META/checksums.blake3`
3. **Merkle root** — BLAKE3 tree rebuilt from `DATA/`
4. **PKI chain** (optional) — package pubkey vs repo root key

### Build

```sh
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)
```

### FFI error codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `FPM_OK` | Success |
| 1 | `FPM_ERR_SIGNATURE` | Ed25519 verification failed |
| 2 | `FPM_ERR_MERKLE` | Merkle root mismatch |
| 3 | `FPM_ERR_CHECKSUM` | Per-file checksum mismatch |
| 4 | `FPM_ERR_PKI` | PKI chain verification failed |
| 5 | `FPM_ERR_IO` | File I/O error |
| 6 | `FPM_ERR_INVALID_INPUT` | Wrong key/signature length |

---

## M4 — Transaction Manager (`fpm-core/`)

Rust library. Manages **atomic generation snapshots** so every install, remove, or upgrade can be rolled back.

### Generation model

```
/var/lib/fpm/
├── generations/
│   ├── 1/  meta.json  root/
│   ├── 2/  meta.json  root/
│   └── 3/  meta.json  root/   ← rollback to 1
├── current -> 3               ← symlink, updated atomically
├── pending/                   ← staging area (M5 writes here)
│   └── root/
└── db.sqlite
```

### API

```rust
let mgr = TransactionManager::new_system();
let mut trx = mgr.begin("install firefox 125.0.3")?;
trx.set_plan(plan);
// M5 installs into trx.root_dir()
let gen_id = trx.commit()?;
mgr.rollback(1)?;
mgr.prune(10)?;
```

---

## M5 — Installer (`fpm-installer/`)

Rust library + CLI. The final step before `trx.commit()`. Receives a `Transaction` handle from M4 and the `InstallPlan`, physically unpacks each `.fpkg` and wires everything together.

### Install pipeline per package

```
 1. run pre-install hook  (META/scripts/pre-install.sh → staged at var/lib/fpm/hooks/)
 2. extract_data()        (tar.zst DATA/ → trx.root_dir(), streaming BLAKE3 per file)
 3. conflict check        (file ownership map across all packages in the plan)
 4. run_layout_fixups()   (ldconfig conf, .desktop entries)
 5. PackageManifest.save()(var/lib/fpm/manifests/<name>-<ver>.json)
 6. run post-install hook
```

### File manifest

Every installed package leaves a manifest at:
```
<root>/var/lib/fpm/manifests/<name>-<version>.json
```
```json
[
  { "path": "usr/bin/firefox",  "blake3": "abc...", "size": 265144, "type": "file" },
  { "path": "usr/share/applications/firefox.desktop", "blake3": "...", "size": 812, "type": "file" }
]
```
This manifest is the source of truth for `fpm remove` and M8 Database.

### Security

- Path-traversal guard: rejects any `DATA/` entry containing `..` components
- Conflict detection: two packages claiming the same file → `InstallerError::FileConflict`
- Hooks run in a plain child process with 60 s timeout; M7 will sandbox them via bwrap

### API

```rust
use fpm_installer::installer::Installer;

let installer = Installer::new();              // hooks + conflict check enabled
let result = installer.install_plan(&trx, &plan)?;
// result.total_files(), result.total_bytes()
let gen_id = trx.commit()?;                   // atomically promote pending/
```

### Remove

```rust
use fpm_installer::remove::Remover;

let remover = Remover::new_system();
remover.remove("firefox", "125.0.3")?;        // reads manifest, deletes files
```

### CLI

```sh
fpm-installer extract  <fpkg> <dest>                     # extract DATA/ only
fpm-installer remove   --name firefox --version 125.0.3  # remove package
fpm-installer list     --root /                          # list installed
fpm-installer manifest --name firefox --version 125.0.3  # show file manifest
```

### Build & test

```sh
cd fpm-installer && cargo test
```

---

## fpkg — Package Inspector CLI

Python 3.11+ tool for inspecting, verifying, and creating `.fpkg` files.

```sh
pip install blake3 tomli-w

./fpkg info     package.fpkg
./fpkg verify   package.fpkg
./fpkg inspect  package.fpkg
./fpkg manifest package.fpkg
./fpkg extract  package.fpkg --dest .
./fpkg create   --name myapp --version 1.0.0 --data ./dist --output myapp.fpkg
```

---

## Repository layout

```
fpkg/
├── fpkg                   # Python package inspector + creator CLI
├── fpkg-build             # Python package builder
├── fpm-solver/            # M1 — Rust, dependency resolution
├── fpm-verifier/          # M3 — C++20, cryptographic verification
├── fpm-fetcher/           # M2 — Rust async, package downloader
├── fpm-core/              # M4 — Rust, transaction manager
│   └── src/{lib,error,paths,generation,plan,trx}.rs
└── fpm-installer/         # M5 — Rust, package installer + remover
    ├── src/
    │   ├── lib.rs             # public API
    │   ├── error.rs           # InstallerError
    │   ├── extract.rs         # tar.zst DATA/ extractor, path-traversal guard
    │   ├── layout.rs          # ldconfig, .desktop fixups
    │   ├── manifest.rs        # PackageManifest (file ownership record)
    │   ├── hooks.rs           # pre/post-install script runner
    │   ├── installer.rs       # Installer orchestrator, conflict detection
    │   ├── remove.rs          # Remover (reads manifest, deletes files)
    │   └── main.rs            # CLI
    ├── tests/installer_tests.rs   # 9 integration tests
    └── Cargo.toml
```

## Building everything

```sh
# 1. M3 Verifier (C++)
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)

# 2. Rust crates (order matters for dep links)
cd ../fpm-solver    && cargo build --release
cd ../fpm-fetcher   && cargo build --release
cd ../fpm-core      && cargo build --release
cd ../fpm-installer && cargo build --release

# 3. Tests
cd ../fpm-verifier  && ctest --test-dir build --output-on-failure
cd ../fpm-solver    && cargo test
cd ../fpm-fetcher   && cargo test
cd ../fpm-core      && cargo test
cd ../fpm-installer && cargo test
```
