## ADDED Requirements

### Requirement: PR and main-branch CI workflow
The system SHALL provide a GitHub Actions workflow file at `.github/workflows/ci.yml` that runs on every pull request targeting `main` and on every push to `main`.

#### Scenario: Workflow triggers on PR
- **WHEN** a pull request is opened or updated against `main`
- **THEN** the CI workflow runs automatically

#### Scenario: Workflow triggers on push to main
- **WHEN** a commit is pushed directly to `main`
- **THEN** the CI workflow runs automatically

### Requirement: Build check on Linux
The CI workflow SHALL run `cargo build` on `ubuntu-latest` with the stable Rust toolchain and fail the workflow if the build fails.

#### Scenario: Build succeeds
- **WHEN** the Rust source compiles without errors on `ubuntu-latest`
- **THEN** the build step passes and the workflow continues

#### Scenario: Build fails
- **WHEN** a compilation error exists in the source
- **THEN** the CI workflow reports a failed status check on the PR

### Requirement: Test run on Linux
The CI workflow SHALL run `cargo test` on `ubuntu-latest` and fail the workflow if any test fails.

#### Scenario: All tests pass
- **WHEN** `cargo test` completes with zero failures
- **THEN** the test step passes

#### Scenario: Test failure blocks merge
- **WHEN** one or more tests fail
- **THEN** the CI workflow reports a failed status check, preventing merge if branch protection is enabled

### Requirement: Cargo build cache
The CI workflow SHALL cache Cargo registry and build artifacts using `Swatinem/rust-cache` to reduce subsequent run times.

#### Scenario: Cache hit reduces build time
- **WHEN** the workflow runs and a valid cache exists for the current `Cargo.lock`
- **THEN** dependency compilation is skipped and the build completes significantly faster than a cold run
