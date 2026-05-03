# Contributing to fpkg

## Commit conventions

All commits must follow the format:

```
type(scope): short description

Longer explanation if needed. Wrap at 72 characters.
```

### Types

| Type | When to use |
|---|---|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `refactor` | Code restructuring without behaviour change |
| `docs` | Documentation only |
| `build` | Build system, Cargo, CI |
| `chore` | Maintenance, dependency bumps, formatting |

### Scope

Scope is the component being changed. Examples: `core`, `builder`, `manifest`, `cli`, `compat`, `root`.

### Rules

- Subject line: imperative mood, no capital, no trailing period, max 72 chars
- Blank line between subject and body
- Body explains *what* and *why*, not *how*
- One logical change per commit — do not batch unrelated changes
- Reference issues in the footer: `Closes #12`, `Fixes #34`

### Examples

```
feat(builder): add source integrity verification via blake3

Verify downloaded source archives against the sha256 field in
PKGBUILD.toml before extracting. Abort build on mismatch.
```

```
fix(manifest): reject unknown install.mode values at parse time
```

```
docs(root): add CONTRIBUTING.md
```

---

## Changelog conventions

The `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

### Version header

```
## [X.Y.Z] - YYYY-MM-DD HH:MM:SS

[type] One-line summary of the release

### Added / Changed / Fixed / Removed
```

### Section headers

Use `### Added`, `### Changed`, `### Fixed`, `### Removed` as needed. Group entries under subsections by path when multiple components are touched:

```
### Added

#### fpkg_lib/

- Add `fpkg_lib/manifest.py`: ...

#### root

- Add `fpkg`: CLI with subcommands ...
```

### Entry format

```
- Add `path/to/file.ext`: what it does and why it matters
- Change `component`: what changed and what it affects
- Fix `component`: what was broken and how it is now correct
- Remove `old-thing`: why it was removed
```

### Rules

- Every merged change that affects users or developers must have a changelog entry
- Entries go under the **Unreleased** block until a version is tagged
- When releasing, rename **Unreleased** to the version + timestamp and open a new **Unreleased** block above it
- Do not describe implementation details — describe observable behaviour
- Do not copy commit messages verbatim — changelog entries are for readers, not for git history
