## Context

Rusty-Pingus is a Rust binary with no existing CI. The project lives on GitHub. We need two workflows: a fast PR check (CI) and a release publisher that produces downloadable binaries for Linux and Windows.

The only complication worth designing around is the `sqlx` offline mode requirement: `sqlx` compile-time query macros need either a live `DATABASE_URL` or a pre-generated `.sqlx/` query cache. We use runtime queries (no `!` macros), so this is not an issue — the build just needs the standard Rust toolchain.

## Goals / Non-Goals

**Goals:**
- PR CI: `cargo build` + `cargo test` on Linux, fast, required to pass before merge
- Release: build Linux and Windows binaries natively on GitHub-hosted runners
- Upload binaries as GitHub Release assets on `v*` tags
- Zero secrets required beyond the default `GITHUB_TOKEN`

**Non-Goals:**
- macOS builds (can be added later trivially)
- Linux ARM / musl builds in v1
- Docker image publishing
- Code signing (Windows binaries unsigned — document this)
- Cross-compilation (using native runners for each platform avoids cross-compile complexity)

## Decisions

### 1. Native runners, not cross-compilation
Use `ubuntu-latest` for Linux and `windows-latest` for Windows rather than cross-compiling from one host. This avoids `cross` tool setup, `cargo-zigbuild`, or MSVC SDK emulation. GitHub provides both runners for free on public repos. Trade-off: slightly longer total wall-clock (two jobs run in parallel), but simpler and more reliable.

### 2. Rust toolchain via `dtolnay/rust-toolchain`
`dtolnay/rust-toolchain@stable` is the community standard for setting up Rust in GitHub Actions. It handles caching of the toolchain itself and supports target specification. Alternative: `actions-rs/toolchain` — deprecated and unmaintained.

### 3. Cargo build cache via `Swatinem/rust-cache`
`Swatinem/rust-cache` caches `~/.cargo/registry`, `~/.cargo/git`, and `target/` keyed on `Cargo.lock`. Dramatically reduces CI time after the first run (from ~5min to ~30s on cache hit). Alternative: manual `actions/cache` — more setup, no `Cargo.lock`-aware keying.

### 4. Release trigger: `push` to tags matching `v*`
Standard Rust project convention. The release workflow triggers on `v*` tags (e.g., `v0.1.0`). The `ci.yml` triggers on PRs and pushes to `main`. The two workflows are separate files so they can be managed and iterated independently.

### 5. Binary naming
- Linux: `rusty-pingus-linux-x86_64`
- Windows: `rusty-pingus-windows-x86_64.exe`

Named with platform suffix so both can coexist as release assets.

## Risks / Trade-offs

- **Windows binary is unsigned** → Browsers and Windows Defender may warn on download. Document this; code signing requires a certificate purchase and is out of scope for v1.
- **GitHub Actions minutes** → Free tier has limits on private repos; public repos get unlimited minutes. Not a concern if the repo is public.
- **Cache invalidation** → `Swatinem/rust-cache` may not cache correctly if `Cargo.lock` is not committed. Ensure `Cargo.lock` is committed (it should be for a binary crate).

## Open Questions

- Should the CI workflow also run `cargo clippy` and `cargo fmt --check`? → Yes, add as optional steps; lint failures are warnings, not blocking, in v1.
