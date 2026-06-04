## Why

On Windows, rusty-pingus currently requires a terminal to stay open, has no way to reach the web UI without knowing the port, and crashes if `config.toml` is missing. Operators expect GUI tools to live quietly in the system tray, be launchable by double-clicking, and not require manual setup before first use.

## What Changes

- **System tray icon** (Windows-only, compiled out on other platforms): a tray icon appears when the binary starts, with a right-click context menu (Open Dashboard, Quit) and a left-click/double-click that opens `http://localhost:<port>` in the default browser
- **No console window on Windows**: the binary runs as a GUI subsystem process so no cmd window flashes or persists; logging redirects to a rolling file in the data directory instead
- **Default config generation**: if `config.toml` (or the path given via `--config`) does not exist at startup, the binary writes a commented default config to that path and logs/notifies the user, then proceeds with zero monitors configured (no crash)
- **Open-on-start**: on first launch (no existing DB), the dashboard is opened in the browser automatically after the server is ready

## Capabilities

### New Capabilities

- `tray-icon`: Windows system tray integration — icon, left-click opens dashboard, right-click menu with Open and Quit actions
- `default-config`: auto-generate a default `config.toml` if none exists, instead of exiting with an error

### Modified Capabilities

- `monitor-config`: the "missing config file exits with error" requirement changes — missing config now generates a default file instead

## Impact

- New dependency: `tray-icon` crate (cross-platform tray, but only wired on Windows)
- New dependency: `webbrowser` crate (open URL in default browser)
- New dependency: `tracing-appender` (rolling file log on Windows when no console)
- `Cargo.toml`: `[[bin]]` target gets `#[cfg(windows)]` subsystem handling via a `build.rs` or `windows` manifest
- `src/main.rs`: platform-specific startup path — Windows spawns tray event loop alongside tokio runtime
- `src/config/mod.rs`: `load()` changes to write default instead of erroring on missing file
- `config.toml` template embedded as a string constant for default generation
- `release.yml`: no changes needed — existing Windows build already produces the `.exe`
