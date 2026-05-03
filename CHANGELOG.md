# Changelog

All notable changes to fpkg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
