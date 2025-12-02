use rdev::{listen, Event, EventType, Key};
use serde::Serialize;
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
struct KeyEvent {
    key: String,
    modifiers: Vec<String>,
    event_type: String,
}

fn key_to_string(key: Key) -> String {
    match key {
        Key::Alt => "Alt".to_string(),
        Key::AltGr => "AltGr".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::CapsLock => "CapsLock".to_string(),
        Key::ControlLeft => "Ctrl".to_string(),
        Key::ControlRight => "Ctrl".to_string(),
        Key::Delete => "Delete".to_string(),
        Key::DownArrow => "↓".to_string(),
        Key::End => "End".to_string(),
        Key::Escape => "Esc".to_string(),
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(),
        Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(),
        Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(),
        Key::F12 => "F12".to_string(),
        Key::Home => "Home".to_string(),
        Key::LeftArrow => "←".to_string(),
        Key::MetaLeft => "Super".to_string(),
        Key::MetaRight => "Super".to_string(),
        Key::PageDown => "PageDown".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::Return => "Enter".to_string(),
        Key::RightArrow => "→".to_string(),
        Key::ShiftLeft => "Shift".to_string(),
        Key::ShiftRight => "Shift".to_string(),
        Key::Space => "Space".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::UpArrow => "↑".to_string(),
        Key::PrintScreen => "PrintScreen".to_string(),
        Key::ScrollLock => "ScrollLock".to_string(),
        Key::Pause => "Pause".to_string(),
        Key::NumLock => "NumLock".to_string(),
        Key::BackQuote => "`".to_string(),
        Key::Num1 => "1".to_string(),
        Key::Num2 => "2".to_string(),
        Key::Num3 => "3".to_string(),
        Key::Num4 => "4".to_string(),
        Key::Num5 => "5".to_string(),
        Key::Num6 => "6".to_string(),
        Key::Num7 => "7".to_string(),
        Key::Num8 => "8".to_string(),
        Key::Num9 => "9".to_string(),
        Key::Num0 => "0".to_string(),
        Key::Minus => "-".to_string(),
        Key::Equal => "=".to_string(),
        Key::KeyQ => "Q".to_string(),
        Key::KeyW => "W".to_string(),
        Key::KeyE => "E".to_string(),
        Key::KeyR => "R".to_string(),
        Key::KeyT => "T".to_string(),
        Key::KeyY => "Y".to_string(),
        Key::KeyU => "U".to_string(),
        Key::KeyI => "I".to_string(),
        Key::KeyO => "O".to_string(),
        Key::KeyP => "P".to_string(),
        Key::LeftBracket => "[".to_string(),
        Key::RightBracket => "]".to_string(),
        Key::KeyA => "A".to_string(),
        Key::KeyS => "S".to_string(),
        Key::KeyD => "D".to_string(),
        Key::KeyF => "F".to_string(),
        Key::KeyG => "G".to_string(),
        Key::KeyH => "H".to_string(),
        Key::KeyJ => "J".to_string(),
        Key::KeyK => "K".to_string(),
        Key::KeyL => "L".to_string(),
        Key::SemiColon => ";".to_string(),
        Key::Quote => "'".to_string(),
        Key::BackSlash => "\\".to_string(),
        Key::IntlBackslash => "\\".to_string(),
        Key::KeyZ => "Z".to_string(),
        Key::KeyX => "X".to_string(),
        Key::KeyC => "C".to_string(),
        Key::KeyV => "V".to_string(),
        Key::KeyB => "B".to_string(),
        Key::KeyN => "N".to_string(),
        Key::KeyM => "M".to_string(),
        Key::Comma => ",".to_string(),
        Key::Dot => ".".to_string(),
        Key::Slash => "/".to_string(),
        Key::Insert => "Insert".to_string(),
        Key::KpReturn => "Enter".to_string(),
        Key::KpMinus => "-".to_string(),
        Key::KpPlus => "+".to_string(),
        Key::KpMultiply => "*".to_string(),
        Key::KpDivide => "/".to_string(),
        Key::Kp0 => "0".to_string(),
        Key::Kp1 => "1".to_string(),
        Key::Kp2 => "2".to_string(),
        Key::Kp3 => "3".to_string(),
        Key::Kp4 => "4".to_string(),
        Key::Kp5 => "5".to_string(),
        Key::Kp6 => "6".to_string(),
        Key::Kp7 => "7".to_string(),
        Key::Kp8 => "8".to_string(),
        Key::Kp9 => "9".to_string(),
        Key::KpDelete => "Delete".to_string(),
        Key::Function => "Fn".to_string(),
        Key::Unknown(code) => format!("Key{}", code),
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
        let (tx, rx) = mpsc::channel::<Event>();

        thread::spawn(move || {
            listen(move |event| {
                let _ = tx.send(event);
            })
            .expect("Could not listen to events");
        });

        let mut modifiers: Vec<String> = Vec::new();

        loop {
            if let Ok(event) = rx.recv() {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        if is_modifier(&key) {
                            let key_str = key_to_string(key);
                            if !modifiers.contains(&key_str) {
                                modifiers.push(key_str);
                            }
                        } else {
                            let key_event = KeyEvent {
                                key: key_to_string(key),
                                modifiers: modifiers.clone(),
                                event_type: "press".to_string(),
                            };
                            let _ = app_handle.emit("key-event", key_event);
                        }
                    }
                    EventType::KeyRelease(key) => {
                        if is_modifier(&key) {
                            let key_str = key_to_string(key);
                            modifiers.retain(|m| m != &key_str);
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            start_keyboard_listener(handle);



            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
