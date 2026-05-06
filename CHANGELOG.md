# Changelog

All notable changes to fpkg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.9] - 2026-05-06 22:44

[feat] fpm-db — add connection pool, repo store, generation manager

### Added

#### fpm-db/ (Rust crate)

- Add `src/pool.rs`: `DbPool` (`r2d2` + `r2d2_sqlite`), `open_pool()`, `open_pool_in_memory()`, schema migration on open
- Add `src/repos.rs`: `DbRepo`, `RepoStore` — `upsert`, `list`, `list_enabled`, `get`, `set_enabled`, `mark_synced`, `remove`
- Add `src/generation_mgr.rs`: `snapshot_generation()`, `generation_snapshot()`, `prune_generations(keep)`, `latest_generation()`, `generation_chain()` (recursive SQL CTE)
- Add `Cargo.toml` deps: `r2d2 = "0.8"`, `r2d2_sqlite = "0.22"`
- Update `src/error.rs`: add `Pool(String)`, `Json(String)` variants
- Update `src/tests.rs`: 8 new tests including `test_concurrent_read_100_threads`

## [0.1.8] - 2026-05-06 22:39

[feat] Implement M8 Database (Rust + SQLite via rusqlite)

### Added

#### fpm-db/ (Rust crate)

- Add `fpm-db/Cargo.toml`: library + binary crate, depends on `rusqlite` (bundled), `serde`, `clap`, `chrono`
- Add `src/schema.rs`: schema version tracking via `PRAGMA user_version`; WAL mode; five tables: `packages`, `files`, `generations`, `repos`, `hold`
- Add `src/models.rs`: typed structs for all five tables with `serde` derive
- Add `src/db.rs`: `Database` handle — `open(path)`, `open_system()` (`/var/lib/fpm/db.sqlite`), `open_user()` (`~/.local/share/fpm/db.sqlite`), `open_default(user_mode)`, `FPM_DB` env var override
- Add `src/packages.rs`: `insert`, `update`, `upsert`, `remove`, `get_by_name`, `list_all`, `search`
- Add `src/files.rs`: `insert_batch`, `delete_for_package`, `list_for_package`, `owner_of`, `count_for_package`
- Add `src/generations.rs`: `record`, `mark_rolled_back`, `list`, `get`, `latest_id`, `purge_old`
- Add `src/repos.rs`: `add`, `remove`, `enable`, `update_sync`, `list`, `get_by_name`
- Add `src/hold.rs`: `add`, `remove`, `list`, `is_held`
- Add `src/main.rs`: `fpm-db` CLI — `init`, `stats`, `list`, `info`, `search`, `owns`, `files`, `generations`, `repos`, `holds` and related subcommands; `--db <path>` and `--user` flags

## [0.1.7] - 2026-05-06 22:33

[feat] Implement M5 Installer in Rust

### Added

#### fpm-installer/ (Rust crate)

- Add `fpm-installer/Cargo.toml`: library + binary crate
- Add `src/error.rs`: `InstallerError` — I/O, path-traversal, conflict, hook timeout variants
- Add `src/extract.rs`: streaming `tar.zst` extractor of `DATA/` into `trx.root_dir()`; path-traversal guard rejecting `..` components; per-file BLAKE3 verification
- Add `src/manifest.rs`: `PackageManifest` — writes `<root>/var/lib/fpm/manifests/<name>-<ver>.json` listing each installed file with path, blake3, size, type
- Add `src/hooks.rs`: pre/post-install script runner from `META/scripts/`; 60 s timeout; plain child process (M7 will sandbox via bwrap)
- Add `src/layout.rs`: ldconfig conf generation, `.desktop` entry registration stubs
- Add `src/installer.rs`: `Installer` orchestrator — runs pipeline: pre-hook → extract → conflict-check → layout-fixups → manifest-save → post-hook
- Add `src/remove.rs`: `Remover` — reads `PackageManifest`, deletes tracked files, prunes empty dirs
- Add `src/main.rs`: `fpm-installer` CLI — `extract`, `remove`, `list`, `manifest`
- Add `tests/installer_tests.rs`: 9 integration tests

## [0.1.6] - 2026-05-06 22:28

[feat] Implement M4 Transaction Manager in Rust

### Added

#### fpm-core/ (Rust crate)

- Add `fpm-core/Cargo.toml`: library crate, `trx` module
- Add `src/error.rs`: `TrxError` enum
- Add `src/paths.rs`: system/user path constants (`/var/lib/fpm/generations/`, `pending/`, `current` symlink)
- Add `src/generation.rs`: `GenerationMeta` — serialises to `meta.json`; `GenerationStatus` (Committed / RolledBack)
- Add `src/plan.rs`: `InstallPlan` — list of `PlannedPackage` with source path and action (Install / Remove / Upgrade)
- Add `src/trx.rs`: `TransactionManager` — `new_system()`, `new_user()`; `begin(description)` → `Transaction`; `rollback(gen_id)` (symlink swap); `prune(keep)` (deletes old generation dirs)
- `Transaction`: `root_dir()`, `set_plan()`, `commit()` (renames `pending/` → `generations/<n>/`, updates `current` symlink atomically), `abort()` (deletes `pending/`)
- Add `src/lib.rs`: public re-exports
- Add `tests/trx_tests.rs`: 7 tests including rollback and prune

## [0.1.5] - 2026-05-06 19:43

[feat] Implement M3 Verifier in C++20 — Ed25519, BLAKE3 Merkle, checksums, PKI chain

### Added

#### fpm-verifier/ (C++20 static library + CLI)

- Add `CMakeLists.txt`: static lib `fpm_verifier`, binary `fpm-verifier`, GoogleTest suite; links libsodium + BLAKE3
- Add `include/fpm_verifier.h`: C API (`fpm_verify_package`, `fpm_verify_merkle`, `fpm_hash_file`) for FFI from Rust
- Add `include/fpm_verifier.hpp`: C++ API — `VerifyError`, `VerifyResult`, all verifier functions
- Add `src/blake3_hasher.cpp`: BLAKE3 hash over file path or raw bytes; hex encode/decode helpers
- Add `src/merkle.cpp`: sort `DATA/` entries, build BLAKE3 binary tree, return hex root
- Add `src/ed25519.cpp`: `verify_ed25519(pubkey_path, sig_path, message_path)` via libsodium
- Add `src/checksum_file.cpp`: parse `META/checksums.blake3`, verify each `DATA/` file hash
- Add `src/pki.cpp`: walk PKI chain — repo root pubkey signs package pubkey, verify via libsodium
- Add `src/verifier.cpp`: `fpm_verify_package()` orchestrator — Ed25519 → checksums → Merkle → PKI
- Add `src/c_api.cpp`: C ABI wrappers over C++ functions
- Add `src/main.cpp`: CLI — `package`, `merkle`, `checksum`, `pki`, `hash` subcommands
- Add `tests/`: 13 GoogleTest tests
- Add `BUILD.md`: build instructions, BLAKE3 vendoring, Rust FFI linking guide

## [0.1.4] - 2026-05-06 19:34

[feat] Implement M2 Fetcher — async parallel downloader with resume, BLAKE3 verify, and cache

### Added

#### fpm-fetcher/ (Rust crate)

- Add `fpm-fetcher/Cargo.toml`: library + binary crate; depends on `reqwest` (rustls-tls, stream), `tokio` (rt-multi-thread, fs, io-util), `blake3`, `futures`, `serde`, `clap`, `anyhow`
- Add `src/types.rs`: `PackageUrl` — download descriptor with `name`, `version`, `urls` (mirror list), `blake3` (optional expected hash), `size`; `FetchError` — typed errors for hash mismatch, download failure, I/O, no-mirrors
- Add `src/cache.rs`: `Cache` — `system()` (`/var/cache/fpm`), `user()` (`~/.cache/fpm`), `from_env(user_mode)` with `FPM_CACHE` override; `contains`, `path_for`, `partial_path_for`, `ensure_dir`, `remove`, `commit_partial` (atomic rename `.part` → `.fpkg`)
- Add `src/mirror.rs`: `Mirror` struct with URL and priority; `probe_mirrors(mirrors, timeout)` — async HEAD probe, returns list sorted by latency
- Add `src/progress.rs`: `Progress` — lock-free byte counter via `Arc<AtomicU64>`; `add`, `downloaded_bytes`, `fraction`
- Add `src/fetcher.rs`: `fetch_one(FetchRequest) -> FetchResult` — cache hit check, then mirror-ordered download with HTTP Range resume; BLAKE3 verify on completion; `fetch_all` — spawns each as independent `tokio::task`
- Add `src/lib.rs`: public re-exports
- Add `src/main.rs`: `fpm-fetcher` CLI — `download`, `probe`, `cached`, `purge`; `--user` flag
- Add `src/tests.rs`: 5 async tests

## [0.1.3] - 2026-05-06 19:28

[feat] Implement M1 Dependency Solver — PubGrub-based dependency resolution with virtual packages and conflict detection

### Added

#### fpm-solver/ (Rust crate)

- Add `fpm-solver/Cargo.toml`: library + binary crate, depends on `pubgrub`, `serde`, `clap`, `toml`, `anyhow`
- Add `src/types.rs`: `Package`, `Version`, `VersionReq`, `Op`, `Dep` — typed primitives with manual semver parsing; `VersionReq::matches` for constraint evaluation
- Add `src/manifest.rs`: `Manifest` parser for `manifest.toml` — `[package]`, `[dependencies.requires]`, `provides`, `conflicts`; supports string deps and full `{name, version, optional, reason}` form
- Add `src/index.rs`: `PackageIndex` — `add`, `versions_of`, `get`, `providers_of`, `resolve_name`, `satisfying_versions`, `has_conflict`
- Add `src/solver.rs`: `resolve(index, root, version) -> Resolution` — `DependencyProvider` over `pubgrub`; maps `VersionReq` to `Range<SemVerVersion>`; optional deps skipped; conflicts via `Range::empty()`
- Add `src/lib.rs`: public re-exports
- Add `src/main.rs`: `fpm-solver` CLI — `resolve`, `check`
- Add `src/tests.rs`: 6 unit tests

## [0.1.2] - 2026-05-03 20:06

[feat] Implement M8 Local Database — SQLite-backed package tracking with generations and rollback

### Added

#### fpkg-db/ (Rust crate)

- Add `fpkg-db/Cargo.toml`: library + binary crate, depends on `rusqlite` (bundled), `serde`, `clap`, `chrono`
- Add `src/schema.rs`: schema version via `PRAGMA user_version`; WAL mode; tables: `packages`, `files`, `generations`, `repos`, `hold`
- Add `src/models.rs`: typed structs with `serde` derive for all tables
- Add `src/db.rs`: `Database` handle; `FPM_DB` env var overrides path
- Add `src/packages.rs`, `files.rs`, `generations.rs`, `repos.rs`, `hold.rs`: full CRUD for each domain
- Add `src/main.rs`: `fpkg-db` CLI with `--db <path>` and `--user` flags

## [0.1.1] - 2026-05-03 19:53

[refactor] Rewrite M10 Builder in Rust; replace Python fpkg-build with native binary

### Changed

#### fpkg-build/ (Rust crate)

- Rewrite M10 Builder as a Rust binary crate (`fpkg-build/`)
- `src/main.rs`: clap CLI, `--dry-run`, `--output-dir`, `--verbose`
- `src/pkgbuild.rs`: serde `PKGBUILD.toml` parser
- `src/manifest.rs`: `Manifest` struct + TOML serialization
- `src/package.rs`: `FpkgWriter`, BLAKE3 checksums, Merkle root
- `src/builder.rs`: full build pipeline, `/bin/sh` script runner
- `src/checksums.rs`: BLAKE3 helpers
- Remove `fpkg_lib/builder.py` (superseded)
- Add `CONTRIBUTING.md`: commit and changelog conventions

## [0.1.0] - 2026-05-03 15:31

[feat] Implement M10 Builder and fpkg core — manifest spec, .fpkg format, CLI tooling

### Added

#### fpkg_lib/

- Add `fpkg_lib/__init__.py`: package entry point, version `0.1.0`
- Add `fpkg_lib/manifest.py`: full `manifest.toml` spec as typed dataclasses — `[package]`, `[package.size]`, `[package.flags]`, `[verification]`, `[dependencies]`, `[install]`, `[repository]`, `[compat]`; TOML serialization via `tomllib` / `tomli-w`
- Add `fpkg_lib/package.py`: `.fpkg` reader (`FpkgReader`) and writer (`FpkgWriter`); BLAKE3 content hashing; Merkle root over `DATA/`; checksum verification
- Add `fpkg_lib/builder.py`: M10 Builder — `PkgBuild` (parses `PKGBUILD.toml`), `Builder` (fetches source, runs build/install scripts, stages `DATA/`, writes `.fpkg`)

#### root

- Add `fpkg`: CLI — `info`, `verify`, `inspect`, `manifest`, `extract`, `create`
- Add `fpkg-build`: M10 Builder CLI — reads `PKGBUILD.toml`, produces `.fpkg`; `--dry-run`, `--output-dir`, `--verbose`
- Add `README.md`: usage, `.fpkg` format reference, `manifest.toml` field table
- Add `example/PKGBUILD.toml`: minimal working build descriptor

## [0.0.1] - 2026-05-03 15:00

[docs] Add LICENSE.md

### Added

- Add `LICENSE.md`: project license file
