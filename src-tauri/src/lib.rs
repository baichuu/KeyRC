use rdev::{listen, Event, EventType, Key};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use std::io::Cursor;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use rand::Rng;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

const KEYBOARD_SOUND: &[u8] = include_bytes!("../../src/assets/sounds/test2.mp3");
const SOUND_DURATION_MS: u64 = 100; // Duration per key click

struct SoundState {
    enabled: Arc<Mutex<bool>>,
    stream_handle: Arc<Mutex<Option<OutputStreamHandle>>>,
}

#[tauri::command]
fn set_sound_enabled(enabled: bool, state: tauri::State<SoundState>) {
    if let Ok(mut guard) = state.enabled.lock() {
        *guard = enabled;
    }
}

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
        Key::Escape => "Escape".to_string(),
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
        Key::Unknown(_) => String::new(),
    }
}

fn should_skip_key(key: &Key) -> bool {
    matches!(key, Key::Unknown(_) | Key::Function)
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

fn play_sound(enabled: &Arc<Mutex<bool>>, stream_handle: &Arc<Mutex<Option<OutputStreamHandle>>>) {
    let is_enabled = enabled.lock().map(|g| *g).unwrap_or(false);
    if !is_enabled {
        return;
    }
    let handle_clone = {
        if let Ok(guard) = stream_handle.lock() {
            guard.clone()
        } else {
            None
        }
    };
    if let Some(handle) = handle_clone {
        thread::spawn(move || {
            let mut rng = rand::rng();
            
            // ASMR-like variations for natural keyboard sound
            
            // Volume: wider range 0.6-1.0 for dynamic feel
            let volume: f32 = rng.random_range(0.6..1.0);
            
            // Speed/pitch: 0.92-1.08 for more noticeable tonal variation
            let speed: f32 = rng.random_range(0.92..1.08);
            
            // Random start offset within first 20ms for slight timing variation
            let start_offset_ms: u64 = rng.random_range(0..20);
            
            // Slightly vary duration for more organic feel (90-120ms)
            let duration_ms: u64 = rng.random_range(90..120);
            
            if let Ok(sink) = Sink::try_new(&handle) {
                if let Ok(source) = Decoder::new(Cursor::new(KEYBOARD_SOUND)) {
                    // Skip a tiny random amount at start for variation
                    let source = source.skip_duration(Duration::from_millis(start_offset_ms));
                    let source = source.speed(speed);
                    sink.set_volume(volume);
                    sink.append(source);
                    thread::sleep(Duration::from_millis(duration_ms));
                    sink.stop();
                }
            }
        });
    }
}

fn start_keyboard_listener(app_handle: AppHandle, sound_enabled: Arc<Mutex<bool>>, stream_handle: Arc<Mutex<Option<OutputStreamHandle>>>) {
    thread::spawn(move || {
        let (tx, rx) = sync_channel::<Event>(32);

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
                    if should_skip_key(&key) {
                        continue;
                    }
                    
                    // Play sound on key press
                    play_sound(&sound_enabled, &stream_handle);
                    
                    if is_modifier(&key) {
                        let key_str = key_to_string(key);
                        if !modifiers.contains(&key_str) {
                            modifiers.push(key_str);
                        }
                    } else {
                        let key_str = key_to_string(key);
                        
                        // F10 to toggle showkey (main window)
                        if key_str == "F10" {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let visible = window.is_visible().unwrap_or(false);
                                if visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_always_on_top(true);
                                    let _ = window.set_focus();
                                }
                                let _ = app_handle.emit("visibility-changed", !visible);
                            }
                            continue;
                        }
                        
                        // F9 to open chat
                        if key_str == "F9" {
                            if let Some(window) = app_handle.get_webview_window("chat") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else if let Ok(chat_window) = WebviewWindowBuilder::new(
                                    &app_handle,
                                    "chat",
                                    WebviewUrl::App("/chat".into()),
                                )
                                .title("Chat")
                                .decorations(false)
                                .always_on_top(true)
                                .skip_taskbar(true)
                                .transparent(true)
                                .build() {
                                #[cfg(target_os = "linux")]
                                make_window_sticky(&chat_window);
                            }
                            continue;
                        }
                        
                        let key_event = KeyEvent {
                            key: key_str,
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
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![set_sound_enabled])
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Setup audio output
            let sound_enabled: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
            let stream_handle: Arc<Mutex<Option<OutputStreamHandle>>> = Arc::new(Mutex::new(None));
            let stream_handle_clone = stream_handle.clone();
            
            // Store state for the command
            app.manage(SoundState {
                enabled: sound_enabled.clone(),
                stream_handle: stream_handle.clone(),
            });
            
            thread::spawn(move || {
                if let Ok((_stream, handle)) = OutputStream::try_default() {
                    if let Ok(mut guard) = stream_handle_clone.lock() {
                        *guard = Some(handle);
                    }
                    // Keep the stream alive
                    loop {
                        thread::sleep(std::time::Duration::from_secs(3600));
                    }
                }
            });
            
            // Give audio thread time to initialize
            thread::sleep(std::time::Duration::from_millis(100));
            
            start_keyboard_listener(handle, sound_enabled, stream_handle);

            // Make main window sticky (visible on all workspaces) on Linux
            #[cfg(target_os = "linux")]
            if let Some(main_window) = app.get_webview_window("main") {
                make_window_sticky(&main_window);
            }

            // Make chat window sticky on Linux
            #[cfg(target_os = "linux")]
            if let Some(chat_window) = app.get_webview_window("chat") {
                make_window_sticky(&chat_window);
            }

            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("KeyRC")
                .menu(&menu)
                .on_menu_event(|app_handle, event| {
                    match event.id.as_ref() {
                        "settings" => {
                            if let Some(window) = app_handle.get_webview_window("settings") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                let _ = WebviewWindowBuilder::new(
                                    app_handle,
                                    "settings",
                                    WebviewUrl::App("/settings".into()),
                                )
                                .title("Settings")
                                .inner_size(500.0, 620.0)
                                .resizable(false)
                                .center()
                                .build();
                            }
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
