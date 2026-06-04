## 1. Dependencies & Build Setup

- [ ] 1.1 Add `tray-icon` crate to `Cargo.toml` under `[target.'cfg(windows)'.dependencies]` so it only compiles on Windows
- [ ] 1.2 Add `webbrowser` crate to `[dependencies]` (cross-platform, used on all platforms for browser open)
- [ ] 1.3 Add `tracing-appender` to `[dependencies]` for file-based log rolling
- [ ] 1.4 Add `winresource` to `[build-dependencies]` (Windows only) for manifest embedding
- [ ] 1.5 Create `build.rs`: on Windows release builds, embed a manifest setting `<requestedExecutionLevel>` and `subsystem` to `windows` so no console window appears on double-click; no-op on debug builds and non-Windows

## 2. Default Config Generation

- [ ] 2.1 Add a `DEFAULT_CONFIG` string constant to `src/config/mod.rs` containing the full default `config.toml` content with commented-out monitor examples
- [ ] 2.2 Modify `config::load()`: if the file does not exist, write `DEFAULT_CONFIG` to the path, emit a `tracing::warn!` with the path, and return a default (empty monitors) `Config` instead of erroring
- [ ] 2.3 Update the integration test for missing config to assert the new behaviour (generates default, returns empty config)

## 3. Windows Tray Module

- [ ] 3.1 Create `src/tray/mod.rs` gated with `#[cfg(windows)]`; define a `TrayCommand` enum (`OpenBrowser(String)`, `Quit`) and a `spawn_tray(url: String, tx: std::sync::mpsc::Sender<TrayCommand>)` function
- [ ] 3.2 In `spawn_tray`: use `tray-icon` to create a tray icon with an embedded PNG icon (embed a small 16×16 or 32×32 status icon via `include_bytes!`)
- [ ] 3.3 Add right-click menu items "Open Dashboard" and "Quit" using `tray-icon`'s `Menu` and `MenuItem` APIs
- [ ] 3.4 Wire left-click / double-click event on the tray icon to send `TrayCommand::OpenBrowser` through the channel
- [ ] 3.5 Wire "Open Dashboard" menu item to send `TrayCommand::OpenBrowser`
- [ ] 3.6 Wire "Quit" menu item to send `TrayCommand::Quit`
- [ ] 3.7 Create a minimal icon PNG (`assets/icon.png`, 32×32, cyan circle on transparent background) and embed it with `include_bytes!`

## 4. Windows Logging (File Appender)

- [ ] 4.1 In `src/main.rs`, add a platform-specific logging init: on Windows release builds (`#[cfg(all(windows, not(debug_assertions)))]`) use `tracing_appender::rolling::daily(log_dir, "rusty-pingus.log")` as the writer; otherwise keep the existing stdout subscriber
- [ ] 4.2 Derive `log_dir` from the database path parent directory (e.g. if db is `./data/rusty-pingus.db`, logs go to `./data/logs/`)
- [ ] 4.3 Ensure the log directory is created with `std::fs::create_dir_all` before initialising the appender

## 5. Main Startup Integration (Windows)

- [ ] 5.1 In `src/main.rs`, detect first launch: check whether the DB file exists before calling `db::init()` and store the result as `is_first_launch: bool`
- [ ] 5.2 On Windows, spawn the async runtime on a background `std::thread` instead of using `#[tokio::main]`; pass a `std::sync::mpsc::Receiver<TrayCommand>` into the async side
- [ ] 5.3 In the async runtime thread, after the web server bind succeeds, if `is_first_launch` is true call `webbrowser::open(&dashboard_url)` to auto-open the browser
- [ ] 5.4 Poll `TrayCommand` receiver in the main async select loop: on `OpenBrowser` call `webbrowser::open(url)`, on `Quit` call `cancel.cancel()` to trigger graceful shutdown
- [ ] 5.5 After async startup is complete, call `spawn_tray(url, tx)` from the main thread and enter the tray event loop (blocking); shutdown the async thread when the loop exits
- [ ] 5.6 On non-Windows, keep the existing `#[tokio::main]` flow unchanged

## 6. Verification

- [ ] 6.1 Build for Windows target (`cargo build --target x86_64-pc-windows-msvc`) and confirm it compiles cleanly
- [ ] 6.2 Verify `cargo clippy -- -D warnings` passes on the Windows target
- [ ] 6.3 Smoke test on a real Windows machine: double-click produces tray icon, no console window, left-click opens browser, Quit shuts down cleanly
- [ ] 6.4 Smoke test missing config: delete `config.toml`, launch binary, confirm default file is generated and binary starts without error
- [ ] 6.5 Update `README.md`: add a Windows section documenting tray icon usage, first-launch behaviour, and log file location
