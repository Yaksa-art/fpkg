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
  M4 fpm-core/trx   ── Atomic generation snapshot (CoW), rollback
       ▼
  M5 fpm-installer  ── (planned) Unpack DATA/, run scripts, layout files
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
| **M5 fpm-installer** | Rust | planned | File layout, ldconfig, desktop entries, M8 record |
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

Rust library + CLI. Consumes `manifest.toml` and a package index, returns a `Vec<ResolvedPackage>` — the exact install set with resolved versions. This list is passed directly to M2 Fetcher.

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

Rust async library + CLI. Receives `Vec<ResolvedPackage>` from M1, downloads `.fpkg` files from configured mirrors in parallel, then calls **M3 Verifier via C FFI** on each downloaded archive before returning paths to the caller (M4).

### Features

| Feature | Detail |
|---------|--------|
| Parallel downloads | Bounded by `parallel_downloads` (default 4), tokio `Semaphore` |
| HTTP Range resume | `.part` files survive interruptions; `Range: bytes=N-` on reconnect |
| Mirror failover | Mirrors probed with HEAD, ranked by latency; next mirror tried on error |
| ETag caching | `If-None-Match` avoids re-download of unchanged packages (304 hit) |
| BLAKE3 pre-check | Optional `blake3` field on `ResolvedPackage` checked before M3 call |
| M3 FFI call | `fpm_verify_package()` called after extraction; bad packages removed from cache |
| Progress events | `mpsc::Sender<ProgressEvent>` — `Started`, `Chunk`, `Downloaded`, `Verified`, `Done`, `Error` |

### Build

```sh
# Build M3 first so FFI link works
cd ../fpm-verifier && cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build -j$(nproc)
cd ../fpm-fetcher  && cargo build --release
```

> If `fpm-verifier/build/` is absent, the crate builds fine but skips cryptographic
> verification (a warning is printed). This allows development without a C++ toolchain.

### CLI

```sh
# Download a single package (uses /etc/fpm/fpm.conf)
fpm-fetcher fetch firefox 125.0.3 --pubkey /etc/fpm/keys/repo.pub

# Rank configured mirrors by latency
fpm-fetcher probe-mirrors
```

Progress events are printed as JSON lines:

```json
{"type":"started","package":"firefox","version":"125.0.3","total_bytes":82000000}
{"type":"chunk","package":"firefox","received_bytes":4096000,"total_bytes":82000000}
{"type":"downloaded","package":"firefox"}
{"type":"verified","package":"firefox","ok":true,"reason":null}
{"type":"done","package":"firefox","path":"/var/cache/fpm/firefox-125.0.3.fpkg"}
```

---

## M3 — Verifier (`fpm-verifier/`)

C++20 static library (`libfpm_verifier.a`) + CLI. Called by M2 Fetcher via FFI after each download. Also available standalone for offline verification.

### Verification pipeline

1. **Ed25519** — `META/signature.ed25519` verified over `manifest.toml` with package public key
2. **Per-file checksums** — every `DATA/` file verified against `META/checksums.blake3`
3. **Merkle root** — BLAKE3 tree rebuilt from `DATA/`, compared to `META/content_tree.txt`
4. **PKI chain** (optional) — package pubkey verified as signed by repo root key

### Build

```sh
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)
# → build/libfpm_verifier.a  (linked by fpm-fetcher)
# → build/fpm-verifier        (standalone CLI)
```

**Dependencies:** CMake 3.20+, GCC 12+ / Clang 15+, `libsodium-dev`, BLAKE3 vendored in `vendor/blake3/`.
See [`fpm-verifier/BUILD.md`](fpm-verifier/BUILD.md) for vendoring instructions.

### CLI

```sh
fpm-verifier package  <extracted-dir> <pubkey>        # full pipeline
fpm-verifier merkle   <data-dir> <expected-root-hex>  # merkle only
fpm-verifier checksum <data-dir> <checksums.blake3>   # checksums only
fpm-verifier pki      <root-pub> <pkg-pub> <sig>      # PKI chain only
fpm-verifier hash     <file>                          # BLAKE3 of any file
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

Rust library. Receives `Vec<FetchResult>` from M2 and `Vec<ResolvedPackage>` from M1, builds an `InstallPlan`, then manages **atomic generation snapshots** so every install, remove, or upgrade can be rolled back without data loss.

### Generation model

Every operation (install / remove / upgrade / rollback) creates a new numbered **generation**. The `current` symlink always points to the active generation. Rolling back simply creates a new generation record pointing at a past state and moves the symlink atomically.

```
/var/lib/fpm/
├── generations/
│   ├── 1/  meta.json  root/   ← initial install
│   ├── 2/  meta.json  root/   ← installed firefox
│   └── 3/  meta.json  root/   ← rollback to 1
├── current -> 3               ← symlink, updated atomically
├── pending/                   ← staging area (exists only during active trx)
│   └── root/                  ← M5 Installer writes files here
└── db.sqlite                  ← M8 Database
```

### Transaction lifecycle

```
TransactionManager::begin(description)
       │  creates pending/, returns Transaction handle
       ▼
trx.set_plan(InstallPlan)   ← built from M1 + M2 results
       │
       ▼  M5 Installer (upcoming) writes files into trx.root_dir()
       │
trx.commit()  ── atomic rename pending/ → generations/<id>/
               ── update current symlink
               ── write meta.json with package list + BLAKE3 hashes
       OR
trx.abort()   ── remove pending/ entirely
```

### API

```rust
use fpm_core::{TransactionManager, InstallPlan, FpmPaths};

let mgr = TransactionManager::new_system();    // /var/lib/fpm
let mut trx = mgr.begin("install firefox 125.0.3")?;

let plan = InstallPlan::from_resolved(&resolved, &fetch_results, &installed, "install firefox");
trx.set_plan(plan);

// M5 Installer would now unpack files into trx.root_dir()

let gen_id = trx.commit()?;   // → 2

// Later:
mgr.rollback(1)?;              // creates gen 3, moves current → 3
mgr.prune(10)?;                // remove old generations, keep 10 most recent
```

### InstallPlan

`InstallPlan::from_resolved()` takes M1 + M2 output and the currently installed package map, and classifies each package as `Install` / `Upgrade { from_version }` / `AlreadyInstalled`. Only actionable entries (not `AlreadyInstalled`) are processed by M5.

### Build & test

```sh
# Requires M1 and M2 already built
cd fpm-core && cargo test
```

---

## fpkg — Package Inspector CLI

Python 3.11+ tool for inspecting, verifying, and creating `.fpkg` files.

```sh
pip install blake3 tomli-w

./fpkg info     package.fpkg           # show metadata
./fpkg verify   package.fpkg           # verify checksums
./fpkg inspect  package.fpkg           # list archive contents
./fpkg manifest package.fpkg           # print raw manifest.toml
./fpkg extract  package.fpkg --dest .  # extract DATA/
./fpkg create   --name myapp --version 1.0.0 --data ./dist --output myapp.fpkg
```

---

## Repository layout

```
fpkg/
├── fpkg                   # Python package inspector + creator CLI
├── fpkg-build             # Python package builder
├── fpm-solver/            # M1 — Rust, dependency resolution
│   ├── src/{types,manifest,index,solver,tests}.rs
│   └── Cargo.toml
├── fpm-verifier/          # M3 — C++20, cryptographic verification
│   ├── include/{fpm_verifier.h,fpm_verifier.hpp}
│   ├── src/{blake3_hasher,merkle,ed25519,checksum_file,pki,verifier,c_api,main}.cpp
│   ├── tests/
│   ├── vendor/blake3/
│   ├── CMakeLists.txt
│   └── BUILD.md
├── fpm-fetcher/           # M2 — Rust async, package downloader
│   ├── src/
│   │   ├── lib.rs             # public API, re-exports solver types
│   │   ├── config.rs          # FetcherConfig (loads fpm.conf)
│   │   ├── mirror.rs          # Mirror, probe_mirrors, rank_mirrors
│   │   ├── cache.rs           # PackageCache (.fpkg + .part + .etag)
│   │   ├── download.rs        # fetch_packages(), fetch_one(), extract_fpkg()
│   │   ├── progress.rs        # ProgressEvent, mpsc channel
│   │   ├── verifier_ffi.rs    # extern "C" bindings to libfpm_verifier.a
│   │   └── main.rs            # CLI: fetch / probe-mirrors
│   ├── tests/download_tests.rs
│   ├── build.rs               # links libfpm_verifier.a + libsodium
│   └── Cargo.toml
└── fpm-core/              # M4 — Rust, transaction manager
    ├── src/
    │   ├── lib.rs             # re-exports all public types
    │   ├── error.rs           # TrxError enum
    │   ├── paths.rs           # FpmPaths (system / user mode)
    │   ├── generation.rs      # Generation, GenerationMeta, list/load/save/prune
    │   ├── plan.rs            # InstallPlan, PlanEntry, PlanOp
    │   └── trx.rs             # Transaction, TransactionManager
    ├── tests/trx_tests.rs     # 8 integration tests
    └── Cargo.toml
```

## Building everything

```sh
# 1. M3 Verifier (C++)
cd fpm-verifier
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)

# 2. M1 Solver (Rust)
cd ../fpm-solver && cargo build --release

# 3. M2 Fetcher (Rust) — links against libfpm_verifier.a
cd ../fpm-fetcher && cargo build --release

# 4. M4 Transaction Manager (Rust)
cd ../fpm-core && cargo build --release

# 5. Tests
cd ../fpm-verifier && ctest --test-dir build --output-on-failure
cd ../fpm-solver   && cargo test
cd ../fpm-fetcher  && cargo test
cd ../fpm-core     && cargo test
```
