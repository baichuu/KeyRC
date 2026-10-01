use rdev::{listen, EventType, Key};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{
    menu::MenuBuilder, tray::TrayIconBuilder, AppHandle, Emitter, LogicalSize, LogicalUnit,
    Manager, PhysicalPosition, WebviewWindowBuilder, WindowEvent, WindowSizeConstraints,
};

#[cfg(target_os = "linux")]
use gtk::prelude::{GtkWindowExt, WidgetExt};

#[cfg(target_os = "linux")]
fn make_window_sticky(window: &tauri::WebviewWindow) {
    if let Ok(gtk_window) = window.gtk_window() {
        gtk_window.stick();
    }
}

#[cfg(target_os = "linux")]
fn lock_overlay_window(window: &tauri::WebviewWindow) {
    if let Ok(gtk_window) = window.gtk_window() {
        // Apply these after the native window is mapped. GTK/Tao may otherwise
        // restore accept_focus while processing the first draw event.
        gtk_window.set_accept_focus(false);
        gtk_window.set_focus_on_map(false);
        gtk_window.set_deletable(false);
    }
}

#[derive(Clone, Serialize)]
struct KeyEvent {
    key: String,
    modifiers: Vec<String>,
}

struct WindowState {
    ready: AtomicBool,
    position_path: PathBuf,
}

fn parse_position(contents: &str) -> Option<PhysicalPosition<i32>> {
    let mut lines = contents.lines();
    let x = lines.next()?.trim().parse().ok()?;
    let y = lines.next()?.trim().parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some(PhysicalPosition::new(x, y))
}

fn read_position(path: &Path) -> Option<PhysicalPosition<i32>> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| parse_position(&contents))
}

fn save_position(path: &Path, position: PhysicalPosition<i32>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, format!("{}\n{}\n", position.x, position.y))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

#[derive(Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DisplayMode {
    #[default]
    Full,
    KeysOnly,
}

impl DisplayMode {
    fn height(self) -> i32 {
        match self {
            Self::Full => 114,
            Self::KeysOnly => 70,
        }
    }

    fn config_value(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::KeysOnly => "keys_only",
        }
    }

    fn toggled(self) -> Self {
        match self {
            Self::Full => Self::KeysOnly,
            Self::KeysOnly => Self::Full,
        }
    }
}

#[derive(Clone, PartialEq, Serialize)]
struct Theme {
    mode: DisplayMode,
    active_bg: String,
    active_fg: String,
    key_text: String,
    background: String,
    border: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            mode: DisplayMode::Full,
            active_bg: "#000000".into(),
            active_fg: "#ffffff".into(),
            key_text: "#ffffff".into(),
            background: "#000000".into(),
            border: "#000000".into(),
        }
    }
}

fn theme_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/keyrc/config.toml"))
}

fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn palette_color<'a>(table: Option<&'a toml::Table>, key: &str) -> Option<&'a str> {
    table
        .and_then(|values| values.get(key))
        .and_then(toml::Value::as_str)
        .filter(|value| valid_color(value))
}

fn parse_theme(contents: &str) -> Theme {
    let mut theme = Theme::default();
    let Ok(config) = toml::from_str::<toml::Table>(contents) else {
        return theme;
    };

    if config.get("mode").and_then(toml::Value::as_str) == Some("keys_only") {
        theme.mode = DisplayMode::KeysOnly;
    }
    let colors = config.get("colors").and_then(toml::Value::as_table);
    for (key, target) in [
        ("active_bg", &mut theme.active_bg),
        ("active_fg", &mut theme.active_fg),
        ("key_text", &mut theme.key_text),
        ("background", &mut theme.background),
        ("border", &mut theme.border),
    ] {
        if let Some(value) = palette_color(colors, key) {
            *target = value.to_owned();
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

fn write_mode(path: &Path, mode: DisplayMode) -> Result<(), String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    toml::from_str::<toml::Table>(&contents).map_err(|error| error.to_string())?;
    let replacement = format!("mode = \"{}\"", mode.config_value());
    let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
    let first_table = lines
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .unwrap_or(lines.len());
    if let Some(index) = lines[..first_table].iter().position(|line| {
        line.split_once('=')
            .is_some_and(|(key, _)| key.trim() == "mode")
    }) {
        lines[index] = replacement;
    } else {
        lines.insert(0, replacement);
        lines.insert(1, String::new());
    }
    let updated = lines.join("\n") + "\n";
    toml::from_str::<toml::Table>(&updated).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, updated).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        error.to_string()
    })
}

fn toggle_mode() -> Result<(), String> {
    let path = theme_path().ok_or("HOME is unavailable")?;
    let current = read_theme_at(Some(&path));
    write_mode(&path, current.mode.toggled())
}

fn theme_revision(path: Option<&Path>) -> Option<(SystemTime, u64)> {
    let metadata = path.and_then(|path| fs::metadata(path).ok())?;
    Some((metadata.modified().ok()?, metadata.len()))
}

#[tauri::command]
fn get_theme() -> Theme {
    read_theme()
}

fn apply_display_mode(app_handle: &AppHandle, mode: DisplayMode) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let height = mode.height();
        #[cfg(target_os = "linux")]
        let _ = window.with_webview(move |webview| {
            webview.inner().set_size_request(290, height);
        });
        // Fixed size constraints keep manual resizing disabled while allowing
        // GTK to shrink the window when the display mode changes.
        let _ = window.set_size_constraints(WindowSizeConstraints {
            min_width: Some(LogicalUnit::new(290.0).into()),
            max_width: Some(LogicalUnit::new(290.0).into()),
            min_height: Some(LogicalUnit::new(f64::from(height)).into()),
            max_height: Some(LogicalUnit::new(f64::from(height)).into()),
        });
        let _ = window.set_size(LogicalSize::new(290.0, f64::from(height)));
    }
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
                if next.mode != current.mode {
                    apply_display_mode(&app_handle, next.mode);
                }
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
    fn parses_theme_config() {
        let theme = parse_theme(
            r##"[colors]
background = "#112233"
key_text = "#aabbcc"
active_bg = "#010203"
border = "#445566"
active_fg = "#234567"
"##,
        );

        assert_eq!(theme.background, "#112233");
        assert_eq!(theme.key_text, "#aabbcc");
        assert_eq!(theme.active_bg, "#010203");
        assert_eq!(theme.active_fg, "#234567");
        assert_eq!(theme.border, "#445566");
    }

    #[test]
    fn rejects_css_instead_of_treating_it_as_a_color() {
        let theme = parse_theme(
            r##"[colors]
background = "red; display: none"
key_text = "#abcdef"
"##,
        );

        assert_eq!(theme.background, "#000000");
        assert_eq!(theme.key_text, "#abcdef");
    }

    #[test]
    fn reads_keys_only_mode_and_five_colors() {
        let theme = parse_theme(
            r##"mode = "keys_only"

[colors]
key_text = "#aabbcc"
active_bg = "#223344"
active_fg = "invalid"
"##,
        );

        assert!(theme.mode == DisplayMode::KeysOnly);
        assert_eq!(theme.key_text, "#aabbcc");
        assert_eq!(theme.active_bg, "#223344");
        assert_eq!(theme.active_fg, "#ffffff");
    }

    #[test]
    fn defaults_to_full_mode_for_missing_or_unknown_mode() {
        for contents in ["", "mode = \"unknown\"", "mode = 1"] {
            assert!(parse_theme(contents).mode == DisplayMode::Full);
        }
    }

    #[test]
    fn writes_mode_without_changing_other_settings() {
        let path = std::env::temp_dir().join(format!(
            "keyrc-mode-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            "# keep this comment\nmode = \"full\"\n\n[colors]\nbackground = \"#112233\"\n",
        )
        .unwrap();
        write_mode(&path, DisplayMode::KeysOnly).unwrap();
        let updated = fs::read_to_string(&path).unwrap();
        assert!(updated.contains("# keep this comment"));
        assert!(updated.contains("mode = \"keys_only\""));
        assert!(updated.contains("background = \"#112233\""));
        fs::remove_file(path).unwrap();
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
            let position_path = app.path().cache_dir()?.join("keyrc/position");
            let saved_position = read_position(&position_path);
            let initial_theme = read_theme();
            let initial_height = f64::from(initial_theme.mode.height());
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .ok_or("Main window configuration is unavailable")?;
            let mut builder = WebviewWindowBuilder::from_config(app, config)?
                .visible(false)
                .focusable(false)
                .closable(false)
                // GTK and WebKit must start with the final geometry. Resizing
                // after creation lets the window manager reposition the popup.
                .inner_size(290.0, initial_height)
                .min_inner_size(290.0, initial_height)
                .max_inner_size(290.0, initial_height)
                .initialization_script(format!(
                    "window.__KEYRC_INITIAL_THEME__ = {};",
                    serde_json::to_string(&initial_theme)?
                ));
            if let Some(position) = saved_position {
                builder = builder.position(f64::from(position.x), f64::from(position.y));
            }
            let main_window = builder.build()?;
            main_window.set_focusable(false)?;
            if let Some(position) = saved_position {
                // The cache contains physical pixels. Set the exact position
                // while hidden, including on displays with non-default scaling.
                main_window.set_position(position)?;
            }
            #[cfg(target_os = "linux")]
            make_window_sticky(&main_window);
            app.manage(WindowState {
                ready: AtomicBool::new(false),
                position_path,
            });
            let window_handle = handle.clone();
            main_window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                if matches!(
                    event,
                    WindowEvent::Moved(_) | WindowEvent::CloseRequested { .. }
                ) {
                    let state = window_handle.state::<WindowState>();
                    if state.ready.load(Ordering::Acquire) {
                        if let Some(window) = window_handle.get_webview_window("main") {
                            // Read the current position so delayed startup events
                            // cannot overwrite the cache with an old coordinate.
                            if let Ok(position) = window.outer_position() {
                                if let Err(error) = save_position(&state.position_path, position) {
                                    eprintln!("Could not save KeyRC position: {error}");
                                }
                            }
                        }
                    }
                }
            });
            // Showing from Rust avoids waiting on JavaScript effects that
            // WebKit can defer while the native window is still unmapped.
            // Geometry and the frontend's initial theme are already prepared.
            main_window.show()?;
            #[cfg(target_os = "linux")]
            lock_overlay_window(&main_window);

            let tray_menu = MenuBuilder::new(app)
                .text("toggle_mode", "Toggle mode")
                .separator()
                .text("quit", "Quit")
                .build()?;
            let mut tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("KeyRC")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle_mode" => {
                        if let Err(error) = toggle_mode() {
                            eprintln!("Could not toggle KeyRC mode: {error}");
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            app.state::<WindowState>()
                .ready
                .store(true, Ordering::Release);
            start_keyboard_listener(handle.clone());
            start_theme_listener(handle);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
