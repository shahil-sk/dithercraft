# DitherCraft (Rust) — Version Control & Release Governance (VERSIONING.md)

This document defines the versioning strategy, Git branch governance, and release management protocols for DitherCraft.

---

## 1. Semantic Versioning Specification (SemVer 2.0.0)

Version format: `MAJOR.MINOR.PATCH`

1. **`MAJOR`**: Incompatible architectural shifts, breaking public API changes in `dither-core`, or breaking format revisions in preset serialization.
2. **`MINOR`**: Backwards-compatible feature additions (new dithering algorithms, new CRT filters, UI improvements, format support).
3. **`PATCH`**: Backwards-compatible bug fixes, performance optimizations, and documentation updates.

### Version Synchronization
The version is declared once at the Cargo workspace root in [`Cargo.toml`](file:///mnt/5893accc-4b16-4de7-b2a7-6333530b2614/PROJECTS/21_Active/dithercraft/Cargo.toml) under `[workspace.package]`:
```toml
[workspace.package]
version = "0.9.0"
edition = "2024"
```
All sub-crates inherit this version:
```toml
[package]
name = "dither-core"
version.workspace = true
```

---

## 2. Git Branching Strategy

- **`main`**: Production-ready, deployable release branch. All commits must pass CI checks (cargo test, clippy, fmt, xtask ci).
- **Topic Branches (`feat/*`, `fix/*`, `perf/*`)**: Isolated work branches merged into `main` via pull requests or verified squashed commits.

---

## 3. Conventional Commits Standard (MANDATORY)

Every commit message MUST adhere to the Conventional Commits specification:

```
<type>(<scope>): <short imperative summary>

- <bullet describing what changed and why>
- <bullet for each significant change>
```

### Commit Types:
- `feat`: A new user-facing feature or algorithm.
- `fix`: A bug fix in algorithms, GUI, or CLI.
- `perf`: A code change that improves performance (e.g. SIMD vectorization, Rayon tuning).
- `refactor`: A code change that neither fixes a bug nor adds a feature.
- `test`: Adding missing tests or correcting existing tests.
- `style`: Changes that do not affect the meaning of the code (formatting, white-space).
- `chore`: Maintenance tasks, dependency updates, build configurations.
- `docs`: Documentation updates only.

### Allowed Scopes:
- `core`: Code inside `crates/dither-core`.
- `gui`: Code inside `crates/dither-gui`.
- `cli`: Code inside `crates/dither-cli`.
- `preset`: Palette or matrix JSON files.
- `ci`: GitHub Actions workflows.
- `docs`: Specifications, roadmaps, or markdown docs.
- `ai`: Agent skills, hooks, or MCP configurations.

---

## 4. Release Tagging & Changelog Automation

1. **Tag Format**: `vX.Y.Z` (e.g. `v6.0.0`).
2. Pushing a tag matching `v*` to GitHub triggers the automated multi-platform release builder (`.github/workflows/release.yml`).
3. Binary artifacts are built, checksummed (`SHA256SUMS.txt`), and attached to the GitHub Release draft.
