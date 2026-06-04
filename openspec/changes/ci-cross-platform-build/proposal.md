## Why

Rusty-Pingus needs to ship pre-built binaries for both Linux (x86_64) and Windows (x86_64) so operators can download and run the tool without a Rust toolchain. GitHub Actions is the natural home for this since the repo is already on GitHub.

## What Changes

- Add `.github/workflows/release.yml`: a GitHub Actions workflow that builds `rusty-pingus` for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) on every push to `main` and on tagged releases
- On tagged releases (`v*`), upload both binaries as GitHub Release assets
- Add `.github/workflows/ci.yml`: a lightweight CI workflow that runs `cargo build` and `cargo test` on pull requests (Linux only, fast feedback)

## Capabilities

### New Capabilities

- `ci-pipeline`: A GitHub Actions CI workflow that validates builds and tests on pull requests
- `release-pipeline`: A GitHub Actions release workflow that builds cross-platform binaries and attaches them to GitHub Releases on version tags

### Modified Capabilities

## Impact

- New files: `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- No changes to Rust source code, `Cargo.toml`, or existing project structure
- Requires the GitHub repo to be public or have Actions enabled
- Windows build uses `x86_64-pc-windows-msvc` (native, no cross-compilation toolchain needed — GitHub-hosted `windows-latest` runner)
- Linux build uses `x86_64-unknown-linux-gnu` on `ubuntu-latest`
