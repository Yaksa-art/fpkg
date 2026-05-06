# Changelog

All notable changes to fpkg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.3] - 2026-05-06 19:30:00

[feat] Implement M1 Dependency Solver — PubGrub-based dependency resolution with virtual packages and conflict detection

### Added

#### fpm-solver/ (Rust crate)

- Add `fpm-solver/Cargo.toml`: library + binary crate, depends on `pubgrub` (PubGrub algorithm), `serde`, `clap`, `toml`, `anyhow`
- Add `src/types.rs`: `Package`, `Version`, `VersionReq`, `Op`, `Dep` — typed primitives with manual semver parsing and comparison; `VersionReq::matches` for constraint evaluation
- Add `src/manifest.rs`: `Manifest` parser for `manifest.toml` — reads `[package]`, `[dependencies.requires]`, `provides`, `conflicts`; supports both simple string deps and full `{name, version, optional, reason}` form
- Add `src/index.rs`: `PackageIndex` — in-memory package registry; `add`, `versions_of`, `get`, `providers_of`, `resolve_name` (resolves virtual package names to real providers via `provides`), `satisfying_versions`, `has_conflict`
- Add `src/solver.rs`: `resolve(index, root, version) -> Resolution` — wraps the `pubgrub` crate's `DependencyProvider` trait; maps `VersionReq` constraints to `Range<SemVerVersion>`; skips optional deps from resolution; inserts conflict ranges as `Range::empty()`; returns `HashMap<String, Version>` on success or a human-readable conflict report on failure
- Add `src/lib.rs`: public library re-exports — `PackageIndex`, `resolve`, `Resolution`, `Dep`, `Package`, `VersionReq`
- Add `src/main.rs`: `fpm-solver` CLI — `resolve --manifest <path> --index <dir>` (loads all `.toml` files from the index dir, runs resolution, prints sorted package list); `check --manifest <path>` (parses and pretty-prints deps, provides, conflicts without resolution)
- Add `src/tests.rs`: unit tests — `test_simple_dep_resolution`, `test_transitive_deps`, `test_version_conflict_detection`, `test_provides_virtual_package`, `test_optional_deps_excluded`, `test_conflict_error_message`

## [0.1.2] - 2026-05-03 15:40:00

[feat] Implement M8 Local Database — SQLite-backed package tracking with generations and rollback

### Added

#### fpkg-db/ (Rust crate)

- Add `fpkg-db/Cargo.toml`: library + binary crate, depends on `rusqlite` (bundled), `serde`, `clap`, `chrono`
- Add `src/schema.rs`: schema version tracking via `PRAGMA user_version`; WAL mode; creates five tables: `packages`, `files`, `generations`, `repos`, `hold`
- Add `src/models.rs`: typed structs for all five tables with `serde` derive — `Package`, `NewPackage`, `PackageFile`, `NewFile`, `Generation`, `GenerationEntry`, `Repo`, `NewRepo`, `Hold`
- Add `src/db.rs`: `Database` handle — `open(path)`, `open_system()` (`/var/lib/fpm/db.sqlite`), `open_user()` (`~/.local/share/fpm/db.sqlite`), `open_default(user_mode)`, `stats()` — `FPM_DB` env var overrides path
- Add `src/packages.rs`: `insert`, `update`, `upsert`, `remove`, `get_by_name`, `list_all` (with optional mode filter), `search` (name + summary LIKE), `is_held`
- Add `src/files.rs`: `insert_batch`, `delete_for_package`, `list_for_package`, `owner_of` (path → package name + version + mode), `count_for_package`
- Add `src/generations.rs`: `record`, `mark_rolled_back`, `list` (DESC, limit), `get`, `latest_id`, `purge_old` — generations store JSON-serialized `Vec<GenerationEntry>`
- Add `src/repos.rs`: `add`, `remove`, `enable`, `update_sync` (stamps `last_sync`), `list`, `get_by_name` — supports `fpkg`, `apt`, `apk`, `rpm` types; priority ordering
- Add `src/hold.rs`: `add`, `remove`, `list`, `is_held` — holds a package name at an optional pinned version
- Add `src/lib.rs`: public library re-exports
- Add `src/main.rs`: `fpkg-db` CLI — `init`, `stats`, `list`, `info`, `search`, `owns`, `files`, `generations`, `gen-record`, `repos`, `repo-add`, `repo-remove`, `holds`, `hold-add`, `hold-remove`, `register`, `unregister`; `--db <path>` and `--user` flags

## [0.1.1] - 2026-05-03 15:35:00

[refactor] Rewrite M10 Builder in Rust; replace Python fpkg-build with native binary

### Changed

#### fpkg-build/ (Rust crate)

- Rewrite M10 Builder as a Rust binary crate (`fpkg-build/`)
- `src/main.rs`: clap CLI, --dry-run/--output-dir/--verbose
- `src/pkgbuild.rs`: serde PKGBUILD.toml parser
- `src/manifest.rs`: Manifest struct + TOML serialization
- `src/package.rs`: FpkgWriter, BLAKE3 checksums, Merkle root
- `src/builder.rs`: full build pipeline, /bin/sh script runner
- `src/checksums.rs`: BLAKE3 helpers
- Remove `fpkg_lib/builder.py` (superseded)
- Add `CONTRIBUTING.md`: commit and changelog conventions

## [0.1.0] - 2026-05-03 15:30:00

[feat] Implement M10 Builder and fpkg core — manifest spec, .fpkg format, CLI tooling

### Added

#### fpkg_lib/

- Add `fpkg_lib/__init__.py`: package entry point, version `0.1.0`
- Add `fpkg_lib/manifest.py`: full `manifest.toml` spec as typed dataclasses — `[package]`, `[package.size]`, `[package.flags]`, `[verification]`, `[dependencies]`, `[install]`, `[repository]`, `[compat]`; TOML serialization and deserialization via `tomllib` / `tomli-w`
- Add `fpkg_lib/package.py`: `.fpkg` archive reader (`FpkgReader`) and writer (`FpkgWriter`); BLAKE3 content hashing; Merkle root computation over `DATA/`; checksum verification
- Add `fpkg_lib/builder.py`: M10 Builder — `PkgBuild` (parses `PKGBUILD.toml`), `Builder` (fetches source, runs build and install scripts in isolated temp dir, stages into `DATA/`, computes checksums, writes `.fpkg`)

#### root

- Add `fpkg`: CLI — `info`, `verify`, `inspect`, `manifest`, `extract`, `create` subcommands
- Add `fpkg-build`: M10 Builder CLI — reads `PKGBUILD.toml`, produces `.fpkg`; supports `--dry-run`, `--output-dir`, `--verbose`
- Add `README.md`: usage instructions for `fpkg` and `fpkg-build`, `.fpkg` format reference, `manifest.toml` field table, build environment variables
- Add `example/PKGBUILD.toml`: minimal working build descriptor

## [0.0.1] - 2026-05-03 15:00:00

[docs] Add LICENSE.md documentation

### Added

#### root

- Add `LICENSE.md`: project license file added to repository root
