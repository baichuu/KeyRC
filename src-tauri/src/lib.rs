use rdev::{listen, Event, EventType, Key};
use serde::Serialize;
use std::sync::mpsc::sync_channel;
use std::thread;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

#[cfg(target_os = "linux")]
use gtk::prelude::GtkWindowExt;

#[cfg(target_os = "linux")]
fn make_window_sticky(window: &tauri::WebviewWindow) {
    if let Ok(gtk_window) = window.gtk_window() {
        gtk_window.stick();
    }
}

#[derive(Clone, Serialize)]
struct KeyEvent {
    key: String,
    modifiers: Vec<String>,
}

fn key_to_string(key: Key) -> &'static str {
    match key {
        Key::Alt => "Alt",
        Key::AltGr => "AltGr",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::ControlLeft | Key::ControlRight => "Ctrl",
        Key::Delete | Key::KpDelete => "Delete",
        Key::DownArrow => "↓",
        Key::End => "End",
        Key::Escape => "Escape",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Home => "Home",
        Key::Insert => "Insert",
        Key::LeftArrow => "←",
        Key::MetaLeft | Key::MetaRight => "Super",
        Key::PageDown => "PageDown",
        Key::PageUp => "PageUp",
        Key::Return | Key::KpReturn => "Enter",
        Key::RightArrow => "→",
        Key::ShiftLeft | Key::ShiftRight => "Shift",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "↑",
        Key::PrintScreen => "PrintScreen",
        Key::ScrollLock => "ScrollLock",
        Key::Pause => "Pause",
        Key::NumLock => "NumLock",
        Key::BackQuote => "`",
        Key::Num1 | Key::Kp1 => "1",
        Key::Num2 | Key::Kp2 => "2",
        Key::Num3 | Key::Kp3 => "3",
        Key::Num4 | Key::Kp4 => "4",
        Key::Num5 | Key::Kp5 => "5",
        Key::Num6 | Key::Kp6 => "6",
        Key::Num7 | Key::Kp7 => "7",
        Key::Num8 | Key::Kp8 => "8",
        Key::Num9 | Key::Kp9 => "9",
        Key::Num0 | Key::Kp0 => "0",
        Key::Minus | Key::KpMinus => "-",
        Key::Equal => "=",
        Key::KpPlus => "+",
        Key::KpMultiply => "*",
        Key::KpDivide | Key::Slash => "/",
        Key::KeyQ => "Q",
        Key::KeyW => "W",
        Key::KeyE => "E",
        Key::KeyR => "R",
        Key::KeyT => "T",
        Key::KeyY => "Y",
        Key::KeyU => "U",
        Key::KeyI => "I",
        Key::KeyO => "O",
        Key::KeyP => "P",
        Key::LeftBracket => "[",
        Key::RightBracket => "]",
        Key::KeyA => "A",
        Key::KeyS => "S",
        Key::KeyD => "D",
        Key::KeyF => "F",
        Key::KeyG => "G",
        Key::KeyH => "H",
        Key::KeyJ => "J",
        Key::KeyK => "K",
        Key::KeyL => "L",
        Key::SemiColon => ";",
        Key::Quote => "'",
        Key::BackSlash | Key::IntlBackslash => "\\",
        Key::KeyZ => "Z",
        Key::KeyX => "X",
        Key::KeyC => "C",
        Key::KeyV => "V",
        Key::KeyB => "B",
        Key::KeyN => "N",
        Key::KeyM => "M",
        Key::Comma => ",",
        Key::Dot => ".",
        Key::Unknown(_) | Key::Function => "",
    }
}

fn is_modifier(key: &Key) -> bool {
    matches!(
        key,
        Key::Alt
            | Key::AltGr
            | Key::ControlLeft
            | Key::ControlRight
            | Key::MetaLeft
            | Key::MetaRight
            | Key::ShiftLeft
            | Key::ShiftRight
    )
}

fn start_keyboard_listener(app_handle: AppHandle) {
    thread::spawn(move || {
        let (tx, rx) = sync_channel::<Event>(128);

        thread::spawn(move || {
            listen(move |event| {
                let _ = tx.try_send(event);
            })
            .expect("Could not listen to events");
        });

        let mut modifiers: Vec<String> = Vec::with_capacity(4);

        while let Ok(event) = rx.recv() {
            match event.event_type {
                EventType::KeyPress(key) => {
                    if is_modifier(&key) {
                        let s = key_to_string(key);
                        if !modifiers.iter().any(|m| m == s) {
                            modifiers.push(s.into());
                        }
                    } else {
                        let s = key_to_string(key);
                        if s.is_empty() {
                            continue;
                        }
                        if s == "F10" {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let visible = window.is_visible().unwrap_or(false);
                                if visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_always_on_top(true);
                                    let _ = window.set_focus();
                                    #[cfg(target_os = "linux")]
                                    make_window_sticky(&window);
                                }
                                let _ = app_handle.emit("visibility-changed", !visible);
                            }
                            continue;
                        }
                        let _ = app_handle.emit("key-event", KeyEvent {
                            key: s.into(),
                            modifiers: modifiers.clone(),
                        });
                    }
                }
                EventType::KeyRelease(key) => {
                    if is_modifier(&key) {
                        let s = key_to_string(key);
                        modifiers.retain(|m| m != s);
                    }
                }
                _ => {}
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let handle = app.handle().clone();
            start_keyboard_listener(handle);

            #[cfg(target_os = "linux")]
            if let Some(main_window) = app.get_webview_window("main") {
                make_window_sticky(&main_window);
            }

            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("KeyRC")
                .menu(&menu)
                .on_menu_event(|app_handle, event| {
                    if event.id.as_ref() == "quit" {
                        app_handle.exit(0);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
