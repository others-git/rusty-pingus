## Purpose

Defines the Windows system tray icon behavior, including startup visibility, click interactions, context menu, console window suppression, file logging for GUI builds, and first-launch browser auto-open.

## Requirements

### Requirement: Tray icon appears on Windows startup
On Windows, the system SHALL display a tray icon in the notification area immediately after the web server is ready. The icon SHALL be visible for the lifetime of the process.

#### Scenario: Icon visible after launch
- **WHEN** the binary is launched on Windows (double-click or terminal)
- **THEN** a tray icon appears in the Windows notification area within 2 seconds of the web server binding its port

#### Scenario: Icon absent on non-Windows platforms
- **WHEN** the binary runs on Linux or macOS
- **THEN** no tray icon is created and no tray-related code executes

### Requirement: Left-click opens the dashboard
The system SHALL open the configured web dashboard URL in the default browser when the user left-clicks or double-clicks the tray icon.

#### Scenario: Single or double click opens browser
- **WHEN** the user clicks the tray icon on Windows
- **THEN** the default browser opens `http://localhost:<configured-port>`

### Requirement: Right-click context menu
The system SHALL display a context menu on right-click with two items: "Open Dashboard" and "Quit".

#### Scenario: Open Dashboard menu item
- **WHEN** the user right-clicks the tray icon and selects "Open Dashboard"
- **THEN** the default browser opens the dashboard URL

#### Scenario: Quit menu item shuts down cleanly
- **WHEN** the user right-clicks the tray icon and selects "Quit"
- **THEN** the application performs a graceful shutdown (identical to Ctrl+C) and the process exits

### Requirement: No console window on double-click launch
The Windows release binary SHALL NOT show a console (cmd) window when launched from Explorer or by double-clicking. Debug builds SHALL retain the console window for developer convenience.

#### Scenario: Release build — no console
- **WHEN** `rusty-pingus.exe` (release build) is launched from Windows Explorer
- **THEN** no console window appears; the process runs silently in the background with the tray icon as the only UI

#### Scenario: Debug build — console present
- **WHEN** `rusty-pingus.exe` (debug build) is launched from a terminal
- **THEN** a console window is present and log output is written to stdout

### Requirement: File logging on Windows release builds
When running as a GUI subsystem process (no console), the system SHALL write log output to rolling daily files in the data directory at `<db-dir>/logs/rusty-pingus.log`.

#### Scenario: Log file created on start
- **WHEN** the Windows release binary starts
- **THEN** a log file is created or appended to at `<data_dir>/logs/rusty-pingus.YYYY-MM-DD.log`

### Requirement: Auto-open browser on first launch
On Windows, if no existing database file is found at startup (first launch), the system SHALL automatically open the dashboard in the default browser once the web server is ready.

#### Scenario: First launch opens browser
- **WHEN** the binary starts on Windows and the database file does not yet exist
- **THEN** after the web server is ready, the default browser opens the dashboard URL automatically

#### Scenario: Subsequent launches do not auto-open
- **WHEN** the binary starts on Windows and the database file already exists
- **THEN** the browser is NOT opened automatically
