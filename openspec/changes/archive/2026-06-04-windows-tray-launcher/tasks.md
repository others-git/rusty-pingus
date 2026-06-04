## 1. Dependencies & Build Setup

- [x] 1.1 Add `tray-icon` crate to `Cargo.toml` under `[target.'cfg(windows)'.dependencies]` so it only compiles on Windows
- [x] 1.2 Add `webbrowser` crate to `[dependencies]` (cross-platform, used on all platforms for browser open)
- [x] 1.3 Add `tracing-appender` to `[dependencies]` for file-based log rolling
- [x] 1.4 Add `winresource` to `[build-dependencies]` (Windows only) for manifest embedding — implemented via `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` in main.rs instead; simpler and idiomatic Rust
- [x] 1.5 Create `build.rs`: on Windows release builds, embed a manifest — handled via cfg_attr attribute; no build.rs required

## 2. Default Config Generation

- [x] 2.1 Add a `DEFAULT_CONFIG` string constant to `src/config/mod.rs` containing the full default `config.toml` content with commented-out monitor examples
- [x] 2.2 Modify `config::load()`: if the file does not exist, write `DEFAULT_CONFIG` to the path, emit a `tracing::warn!` with the path, and return a default (empty monitors) `Config` instead of erroring
- [x] 2.3 Update the integration test for missing config to assert the new behaviour (generates default, returns empty config)

## 3. Windows Tray Module

- [x] 3.1 Create `src/tray/mod.rs` gated with `#[cfg(windows)]`; defines `run_event_loop(url, cancel)` which owns the main-thread tao event loop
- [x] 3.2 In `run_event_loop`: tray icon created via `TrayIconBuilder` with programmatic cyan circle RGBA (no PNG file needed — `Icon::from_rgba`)
- [x] 3.3 Add right-click menu items "Open Dashboard" and "Quit" using `tray-icon`'s `Menu` and `MenuItem` APIs
- [x] 3.4 Wire left-click event on the tray icon to call `webbrowser::open` directly
- [x] 3.5 Wire "Open Dashboard" menu item to call `webbrowser::open`
- [x] 3.6 Wire "Quit" menu item to cancel the token and set `ControlFlow::Exit`
- [x] 3.7 Icon generated programmatically at runtime — no PNG asset file required

## 4. Windows Logging (File Appender)

- [x] 4.1 In `src/main.rs`, add a platform-specific logging init: on Windows release builds (`#[cfg(all(windows, not(debug_assertions)))]`) use `tracing_appender::rolling::daily(log_dir, "rusty-pingus.log")` as the writer; otherwise keep the existing stdout subscriber
- [x] 4.2 Derive `log_dir` from the database path parent directory (e.g. if db is `./data/rusty-pingus.db`, logs go to `./data/logs/`)
- [x] 4.3 Ensure the log directory is created with `std::fs::create_dir_all` before initialising the appender

## 5. Main Startup Integration (Windows)

- [x] 5.1 In `src/main.rs`, detect first launch: check whether the DB file exists before calling `db::init()` and store the result as `is_first_launch: bool`
- [x] 5.2 On Windows, spawn the async runtime on a background `std::thread` instead of using `#[tokio::main]`; Tokio runtime created explicitly with `Runtime::new()`
- [x] 5.3 On Windows first launch, `webbrowser::open` called on main thread after 800ms startup delay; on non-Windows, called from a tokio task
- [x] 5.4 Tray Quit handler calls `cancel.cancel()` + `process::exit(0)` after a brief drain pause; OpenBrowser calls `webbrowser::open` directly from the message loop
- [x] 5.5 Main thread calls `tray::run_event_loop(url, cancel)` which runs the Win32 message pump — never returns
- [x] 5.6 Non-Windows path uses explicit `tokio::runtime::Runtime::new()?.block_on(async_main(...))` — equivalent to `#[tokio::main]`

## 6. Verification

- [x] 6.1 Build for Windows target (`cargo build --target x86_64-pc-windows-msvc`) and confirm it compiles cleanly
- [x] 6.2 Verify `cargo clippy -- -D warnings` passes on the Windows target
- [x] 6.3 Smoke test on a real Windows machine: double-click produces tray icon, no console window, left-click opens browser, Quit shuts down cleanly
- [x] 6.4 Smoke test missing config: covered by `missing_config_generates_default_and_returns_empty` integration test
- [x] 6.5 Update `README.md`: added Windows section documenting tray icon usage, first-launch behaviour, and log file location
