## ADDED Requirements

### Requirement: Release workflow triggered by version tags
The system SHALL provide a GitHub Actions workflow file at `.github/workflows/release.yml` that triggers on pushes of tags matching `v*` (e.g., `v0.1.0`, `v1.2.3`).

#### Scenario: Tag push triggers release workflow
- **WHEN** a git tag matching `v*` is pushed to the repository
- **THEN** the release workflow starts automatically

#### Scenario: Non-tag push does not trigger release
- **WHEN** a commit is pushed to a branch without a matching tag
- **THEN** the release workflow does NOT run

### Requirement: Linux binary build
The release workflow SHALL build a release-mode binary for `x86_64-unknown-linux-gnu` on an `ubuntu-latest` runner and name the output `rusty-pingus-linux-x86_64`.

#### Scenario: Linux binary produced
- **WHEN** the release workflow runs its Linux job
- **THEN** `cargo build --release --target x86_64-unknown-linux-gnu` completes and produces a binary at `target/x86_64-unknown-linux-gnu/release/rusty-pingus`

### Requirement: Windows binary build
The release workflow SHALL build a release-mode binary for `x86_64-pc-windows-msvc` on a `windows-latest` runner and name the output `rusty-pingus-windows-x86_64.exe`.

#### Scenario: Windows binary produced
- **WHEN** the release workflow runs its Windows job
- **THEN** `cargo build --release --target x86_64-pc-windows-msvc` completes and produces a binary at `target\x86_64-pc-windows-msvc\release\rusty-pingus.exe`

### Requirement: Binaries uploaded as GitHub Release assets
The release workflow SHALL create a GitHub Release for the triggering tag (if one does not exist) and upload both platform binaries as release assets using the default `GITHUB_TOKEN`.

#### Scenario: Release created and assets attached
- **WHEN** both platform jobs complete successfully
- **THEN** a GitHub Release exists for the tag with `rusty-pingus-linux-x86_64` and `rusty-pingus-windows-x86_64.exe` as downloadable assets

#### Scenario: Release uses tag name as version
- **WHEN** the tag `v0.2.0` is pushed
- **THEN** the GitHub Release is titled `v0.2.0` and the assets are attached to that release

### Requirement: Jobs run in parallel
The Linux and Windows build jobs SHALL run concurrently, not sequentially, to minimize total release wall-clock time.

#### Scenario: Parallel execution
- **WHEN** the release workflow starts
- **THEN** both the Linux and Windows build jobs start simultaneously as independent jobs in the same workflow

### Requirement: Cargo build cache on release jobs
Each release build job SHALL use `Swatinem/rust-cache` to cache dependencies and avoid redundant downloads across repeated release builds.

#### Scenario: Cache populated on first run
- **WHEN** the release workflow runs for the first time
- **THEN** Cargo dependencies are downloaded, compiled, and cached for future runs
