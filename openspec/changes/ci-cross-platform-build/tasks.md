## 1. Repository Preparation

- [x] 1.1 Create `.github/workflows/` directory
- [x] 1.2 Ensure `Cargo.lock` is committed to the repository (required for `Swatinem/rust-cache` to key correctly; binary crates should always commit the lockfile)

## 2. CI Workflow

- [x] 2.1 Create `.github/workflows/ci.yml` with triggers: `push` to `main` and `pull_request` targeting `main`
- [x] 2.2 Add a single `build` job on `ubuntu-latest` using `dtolnay/rust-toolchain@stable`
- [x] 2.3 Add `Swatinem/rust-cache` step to the CI job for dependency caching
- [x] 2.4 Add `cargo build` step (debug profile is sufficient for PR checks)
- [x] 2.5 Add `cargo test` step
- [x] 2.6 Add optional `cargo clippy -- -D warnings` step (lint check)

## 3. Release Workflow

- [x] 3.1 Create `.github/workflows/release.yml` with trigger: `push` to tags matching `v*`
- [x] 3.2 Define a `build-linux` job on `ubuntu-latest`: set up stable Rust, add `x86_64-unknown-linux-gnu` target, run `cargo build --release --target x86_64-unknown-linux-gnu`
- [x] 3.3 In `build-linux`, rename the output binary to `rusty-pingus-linux-x86_64` and upload it as a workflow artifact
- [x] 3.4 Define a `build-windows` job on `windows-latest`: set up stable Rust, add `x86_64-pc-windows-msvc` target, run `cargo build --release --target x86_64-pc-windows-msvc`
- [x] 3.5 In `build-windows`, rename the output binary to `rusty-pingus-windows-x86_64.exe` and upload it as a workflow artifact
- [x] 3.6 Add `Swatinem/rust-cache` to both `build-linux` and `build-windows` jobs
- [x] 3.7 Define a `release` job that depends on both build jobs (`needs: [build-linux, build-windows]`), downloads both artifacts, and uses `softprops/action-gh-release` to create/update the GitHub Release for the tag and attach both binaries
- [x] 3.8 Verify the release workflow YAML is valid (check indentation, `permissions: contents: write` set on the release job for `GITHUB_TOKEN` to create releases)
