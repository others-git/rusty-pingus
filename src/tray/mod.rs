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

/// Generate a 32×32 cyan circle as raw RGBA — no PNG file needed.
fn make_icon() -> tray_icon::Icon {
    let size = 32u32;
    let cx = 15.5f32;
    let cy = 15.5f32;
    let r2 = 14.0f32 * 14.0f32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let i = ((y * size + x) * 4) as usize;
            if dx * dx + dy * dy <= r2 {
                rgba[i] = 34;    // #22d3ee cyan
                rgba[i + 1] = 211;
                rgba[i + 2] = 238;
                rgba[i + 3] = 255;
            }
        }
    }
    tray_icon::Icon::from_rgba(rgba, size, size).expect("build tray icon")
}

/// Create the system tray icon and run a Win32 message loop on the calling
/// (main) thread. This function never returns.
pub fn run_event_loop(dashboard_url: String, cancel: CancellationToken) -> ! {
    let open_item = MenuItem::new("Open Dashboard", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let menu = Menu::new();
    menu.append(&open_item).expect("menu append");
    menu.append(&quit_item).expect("menu append");

    let open_id = open_item.id().clone();
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
