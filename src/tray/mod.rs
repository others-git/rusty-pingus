#![cfg(windows)]

use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

/// Decode the embedded app icon (a 32×32 PNG rendered from assets/favicon.svg)
/// into the raw RGBA the tray API wants, so the tray matches the favicon and
/// the Unraid icon without shipping a separate file.
fn make_icon() -> tray_icon::Icon {
    static ICON_PNG: &[u8] = include_bytes!("../../assets/tray-icon.png");
    let mut decoder = png::Decoder::new(ICON_PNG);
    // Normalize to 8-bit RGBA regardless of how the encoder wrote the PNG.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().expect("read tray icon png");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("decode tray icon png");
    buf.truncate(info.buffer_size());
    tray_icon::Icon::from_rgba(buf, info.width, info.height).expect("build tray icon")
}

/// Re-launch this executable with the same command-line arguments, then signal
/// shutdown so the current process releases its resources — notably the web
/// server's bound port. Used by the tray "Reload Config" action to pick up
/// changes to config.toml (including a new `[web].bind` port) without the user
/// having to manually stop and restart the app.
///
/// Order matters: we cancel first so the running web server gracefully shuts
/// down and frees its port *before* the new process tries to bind it.
fn reload_config(cancel: &CancellationToken) -> ! {
    use std::process::Command;

    let exe = std::env::current_exe();
    let args: Vec<String> = std::env::args().skip(1).collect();

    cancel.cancel();
    // Give the async runtime a moment to drain and release the bound port.
    std::thread::sleep(Duration::from_millis(1000));

    match exe {
        Ok(exe) => {
            if let Err(e) = Command::new(&exe).args(&args).spawn() {
                tracing::error!(error = %e, "Failed to relaunch for config reload");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Could not determine current executable for config reload");
        }
    }

    std::process::exit(0);
}

/// Create the system tray icon and run a Win32 message loop on the calling
/// (main) thread. This function never returns.
pub fn run_event_loop(dashboard_url: String, cancel: CancellationToken) -> ! {
    let open_item = MenuItem::new("Open Dashboard", true, None);
    let reload_item = MenuItem::new("Reload Config", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let menu = Menu::new();
    menu.append(&open_item).expect("menu append");
    menu.append(&reload_item).expect("menu append");
    menu.append(&quit_item).expect("menu append");

    let open_id = open_item.id().clone();
    let reload_id = reload_item.id().clone();
    let quit_id = quit_item.id().clone();

    // TrayIcon must be created on the thread that runs the message loop.
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Rusty Pingus")
        .with_icon(make_icon())
        .build()
        .expect("failed to create tray icon");

    // Pump Win32 messages so the tray icon processes shell notifications.
    // PeekMessage is non-blocking, allowing us to also poll tray/menu events.
    loop {
        // Drain the Win32 message queue
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Handle tray icon click (left-click opens dashboard)
        if let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
            use tray_icon::{MouseButton, MouseButtonState};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = tray_event
            {
                let _ = webbrowser::open(&dashboard_url);
            }
        }

        // Handle menu item selection
        if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            if menu_event.id == open_id {
                let _ = webbrowser::open(&dashboard_url);
            } else if menu_event.id == reload_id {
                reload_config(&cancel);
            } else if menu_event.id == quit_id {
                cancel.cancel();
                // Brief pause to let the async runtime start shutting down.
                std::thread::sleep(Duration::from_millis(800));
                std::process::exit(0);
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
