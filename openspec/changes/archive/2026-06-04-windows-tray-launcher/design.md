## Context

Rusty-pingus is a Tokio async binary. The challenge of adding a system tray is that tray icon libraries require their event loop to run on the **main thread**, while Tokio wants to own the main thread for its runtime. These two demands must be reconciled. On non-Windows platforms, all tray code is compiled out entirely — no runtime check, no stub thread.

The default-config feature is comparatively simple: a string constant holding the default TOML is embedded in the binary, written to disk on first run.

## Goals / Non-Goals

**Goals:**
- Windows tray icon with left-click (open dashboard) and right-click menu (Open Dashboard, Quit)
- No console window on double-click launch from Explorer
- Auto-generate `config.toml` if missing rather than exiting with an error
- Auto-open browser on first launch (no existing DB)
- File-based logging on Windows when running without a console

**Non-Goals:**
- macOS menu bar icon (separate change if ever needed)
- Linux tray / AppIndicator
- Tray balloon notifications / toasts for monitor state changes (v2 feature)
- Custom tray icon artwork in v1 (use a coloured circle or embedded PNG)
- Installer / NSIS / MSI packaging

## Decisions

### 1. Threading model: tray on main thread, Tokio on a dedicated thread
`tray-icon` (the most actively maintained cross-platform tray crate) requires its event loop on the main OS thread on Windows. Solution: on Windows, `main()` spawns a background `std::thread` that creates the Tokio runtime and runs `async_main()`, then the main thread enters the tray event loop. The tray sends commands (OpenBrowser, Quit) via a `std::sync::mpsc` channel read by the async side. Alternative: `systray` crate — unmaintained. `tray-item` — simpler API but less flexible menu support.

### 2. Windows subsystem: `build.rs` manifest embedding
Setting `#![windows_subsystem = "windows"]` in source silences the console globally, but breaks `cargo run` in a terminal (no stdout). Better: embed a Windows application manifest via `build.rs` using the `winresource` crate, setting the subsystem to `windows` only in release builds. Debug builds keep the console. Alternative: a separate `rusty-pingus-tray.exe` wrapper — rejected, complicates distribution.

### 3. Logging on Windows (no console): `tracing-appender`
When running as a GUI subsystem process, stdout is unavailable. Use `tracing-appender` to write rolling daily log files to `<data_dir>/logs/`. On non-Windows or debug builds, keep the existing stdout subscriber. The log path is reported in the tray tooltip.

### 4. Default config: embedded string constant
The default `config.toml` content is stored as a `const &str` in `src/config/mod.rs`. On missing file, it is written to disk and the user is notified via tray tooltip (Windows) or stderr (other platforms). The binary then proceeds with zero monitors. Alternative: generate programmatically from structs — more complex, harder to keep readable.

### 5. Browser open: `webbrowser` crate
`webbrowser::open(url)` is a one-liner that works on Windows, macOS, and Linux. Used for both left-click and auto-open on first launch.

### 6. First-launch detection: DB file existence
If the SQLite file does not exist before `db::init()` creates it, it is the first launch. On Windows, this triggers an automatic browser open after a short delay (giving the web server time to start).

## Risks / Trade-offs

- **`tray-icon` requires a Win32 event loop** → Must use `EventLoopBuilder` from `tao` or `winit`. `tray-icon` 0.x bundles `tao` as a dependency; using it correctly requires processing `tao` events even if we don't render a window. Mitigation: follow `tray-icon` examples; use `EventLoop::run()` with a minimal handler that only processes tray menu events.
- **Release build loses console** → Developers debugging a release build can set `RUST_LOG` and redirect to a file, or use a debug build. Document this.
- **`winresource` adds a Windows build-time dependency** → Only active when cross-compiling or building on Windows. The CI `windows-latest` runner has the MSVC toolchain; `winresource` uses `rc.exe` from the Windows SDK which is present there.
- **Auto-open on first launch may be surprising** → Mitigated by only doing it once (first launch = no DB file). Subsequent starts do not auto-open.

## Open Questions

- Should the tray icon colour reflect overall monitor health (green/red)? → Defer to v2; v1 uses a static icon.
- Should "Quit" in the tray menu require confirmation? → No, keep it simple.
