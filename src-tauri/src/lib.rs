use rdev::{listen, EventType, Key};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, LogicalSize, Manager};

#[cfg(target_os = "linux")]
use gtk::prelude::{GtkWindowExt, WidgetExt};

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

#[derive(Clone, PartialEq, Serialize)]
struct Theme {
    is_light: bool,
    bg: String,
    fg: String,
    bg_dark: String,
    border: String,
    grey: String,
    green: String,
    blue: String,
    purple: String,
    cyan: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            is_light: false,
            bg: "#000000".into(),
            fg: "#ffffff".into(),
            bg_dark: "#000000".into(),
            border: "#000000".into(),
            grey: "#666666".into(),
            green: "#ffffff".into(),
            blue: "#ffffff".into(),
            purple: "#ffffff".into(),
            cyan: "#ffffff".into(),
        }
    }
}

fn theme_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share/nvim/theme"))
}

fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_theme(contents: &str) -> Theme {
    let values: HashMap<&str, &str> = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect();
    let mut theme = Theme::default();

    theme.is_light = values.get("is_light").is_some_and(|value| *value == "1");
    for (key, target) in [
        ("bg", &mut theme.bg),
        ("fg", &mut theme.fg),
        ("bg_dark", &mut theme.bg_dark),
        ("border", &mut theme.border),
        ("grey", &mut theme.grey),
        ("green", &mut theme.green),
        ("blue", &mut theme.blue),
        ("purple", &mut theme.purple),
        ("cyan", &mut theme.cyan),
    ] {
        if let Some(value) = values.get(key).filter(|value| valid_color(value)) {
            *target = (*value).to_owned();
        }
    }

    theme
}

fn read_theme_at(path: Option<&Path>) -> Theme {
    path.and_then(|path| fs::read_to_string(path).ok())
        .map(|contents| parse_theme(&contents))
        .unwrap_or_default()
}

fn read_theme() -> Theme {
    let path = theme_path();
    read_theme_at(path.as_deref())
}

fn theme_revision(path: Option<&Path>) -> Option<(SystemTime, u64)> {
    let metadata = path.and_then(|path| fs::metadata(path).ok())?;
    Some((metadata.modified().ok()?, metadata.len()))
}

#[tauri::command]
fn get_theme() -> Theme {
    read_theme()
}

fn start_theme_listener(app_handle: AppHandle) {
    thread::spawn(move || {
        let path = theme_path();
        let mut revision = theme_revision(path.as_deref());
        let mut current = read_theme_at(path.as_deref());

        loop {
            thread::sleep(Duration::from_millis(250));
            let next_revision = theme_revision(path.as_deref());
            if next_revision == revision {
                continue;
            }

            revision = next_revision;
            let next = read_theme_at(path.as_deref());
            if next != current {
                current = next;
                let _ = app_handle.emit("theme-changed", current.clone());
            }
        }
    });
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    #[test]
    fn parses_themesync_palette() {
        let theme = parse_theme(
            "is_light=1\nbg=#112233\nfg=#aabbcc\nbg_dark=#010203\n\
             border=#445566\ngrey=#778899\ngreen=#123456\nblue=#234567\n\
             purple=#345678\ncyan=#456789\n",
        );

        assert!(theme.is_light);
        assert_eq!(theme.bg, "#112233");
        assert_eq!(theme.fg, "#aabbcc");
        assert_eq!(theme.cyan, "#456789");
    }

    #[test]
    fn rejects_css_instead_of_treating_it_as_a_color() {
        let theme = parse_theme("bg=red; display: none\nfg=#abcdef\n");

        assert_eq!(theme.bg, "#000000");
        assert_eq!(theme.fg, "#abcdef");
    }
}

fn key_to_string(key: Key) -> &'static str {
    match key {
        Key::Alt => "Alt",
        Key::AltGr => "AltGr",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::ControlLeft | Key::ControlRight => "Ctrl",
        Key::Delete | Key::KpDelete => "Delete",
        Key::DownArrow => "",
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
        Key::LeftArrow => "",
        Key::MetaLeft | Key::MetaRight => "Super",
        Key::PageDown => "PageDown",
        Key::PageUp => "PageUp",
        Key::Return | Key::KpReturn => "Enter",
        Key::RightArrow => "",
        Key::ShiftLeft | Key::ShiftRight => "Shift",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "",
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
        let mut modifiers: Vec<String> = Vec::with_capacity(4);

        listen(move |event| match event.event_type {
            EventType::KeyPress(key) => {
                if is_modifier(&key) {
                    let s = key_to_string(key);
                    if !modifiers.iter().any(|m| m == s) {
                        modifiers.push(s.into());
                    }
                } else {
                    let s = key_to_string(key);
                    if s.is_empty() {
                        return;
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
                        return;
                    }
                    let _ = app_handle.emit(
                        "key-event",
                        KeyEvent {
                            key: s.into(),
                            modifiers: modifiers.clone(),
                        },
                    );
                }
            }
            EventType::KeyRelease(key) => {
                if is_modifier(&key) {
                    let s = key_to_string(key);
                    modifiers.retain(|m| m != s);
                }
            }
            _ => {}
        })
        .expect("Could not listen to events");
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![get_theme])
        .setup(|app| {
            let handle = app.handle().clone();
            start_keyboard_listener(handle.clone());
            start_theme_listener(handle);

            #[cfg(target_os = "linux")]
            if let Some(main_window) = app.get_webview_window("main") {
                // WebKitGTK reports a 200 px natural minimum while a fixed
                // window is being created. Wry also gives its webview a
                // 200x200 size request, so shrink the child before the window.
                let _ = main_window.with_webview(|webview| {
                    webview.inner().set_size_request(300, 100);
                });
                let _ = main_window.set_size(LogicalSize::new(300.0, 100.0));
                let _ = main_window.set_resizable(false);
                make_window_sticky(&main_window);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
