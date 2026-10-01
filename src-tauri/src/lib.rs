use gtk::cairo::{Context, Operator};
use gtk::gdk;
use gtk::gdk::prelude::GdkContextExt;
use gtk::gdk_pixbuf::{prelude::PixbufLoaderExt, Pixbuf, PixbufLoader};
use gtk::glib::translate::{from_glib_full, ToGlibPtr};
use gtk::glib::{self, ControlFlow};
use gtk::pango::{FontDescription, Layout};
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use rdev::{listen, EventType, Key};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::f64::consts::{FRAC_PI_2, PI};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const WIDTH: i32 = 290;
const KEY_HEIGHT: i32 = 70;
const FULL_HEIGHT: i32 = 114;
const MODIFIER_Y: f64 = 72.0;
const MODIFIER_HEIGHT: f64 = 42.0;
const CORNER_RADIUS: f64 = 24.0;
const FONT: &str = "Iosevka Nerd Font Mono";
const MAX_HISTORY: usize = 6;
const MAX_DISPLAY_UNITS: usize = 6;

#[link(name = "pangocairo-1.0")]
unsafe extern "C" {
    fn pango_cairo_create_layout(
        context: *mut gtk::cairo::ffi::cairo_t,
    ) -> *mut gtk::pango::ffi::PangoLayout;
    fn pango_cairo_show_layout(
        context: *mut gtk::cairo::ffi::cairo_t,
        layout: *mut gtk::pango::ffi::PangoLayout,
    );
}

#[derive(Clone, Copy, Default, PartialEq)]
enum DisplayMode {
    #[default]
    Full,
    KeysOnly,
}

impl DisplayMode {
    fn height(self) -> i32 {
        match self {
            Self::Full => FULL_HEIGHT,
            Self::KeysOnly => KEY_HEIGHT,
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

#[derive(Clone, PartialEq)]
struct Theme {
    mode: DisplayMode,
    active_bg: Color,
    active_fg: Color,
    key_text: Color,
    background: Color,
    border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            mode: DisplayMode::Full,
            active_bg: Color::rgb(0, 0, 0),
            active_fg: Color::rgb(255, 255, 255),
            key_text: Color::rgb(255, 255, 255),
            background: Color::rgb(0, 0, 0),
            border: Color::rgb(0, 0, 0),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Color {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

impl Color {
    const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red: red as f64 / 255.0,
            green: green as f64 / 255.0,
            blue: blue as f64 / 255.0,
            alpha: 1.0,
        }
    }

    fn parse(value: &str) -> Option<Self> {
        if !matches!(value.len(), 7 | 9) || !value.starts_with('#') {
            return None;
        }
        let byte = |offset| u8::from_str_radix(&value[offset..offset + 2], 16).ok();
        Some(Self {
            red: f64::from(byte(1)?) / 255.0,
            green: f64::from(byte(3)?) / 255.0,
            blue: f64::from(byte(5)?) / 255.0,
            alpha: f64::from(if value.len() == 9 { byte(7)? } else { 255 }) / 255.0,
        })
    }

    fn with_alpha(self, alpha: f64) -> Self {
        Self {
            alpha: self.alpha * alpha,
            ..self
        }
    }

    fn set(self, context: &Context) {
        context.set_source_rgba(self.red, self.green, self.blue, self.alpha);
    }
}

#[derive(Clone, Copy, Default)]
struct Modifiers {
    shift: bool,
    ctrl: bool,
    alt: bool,
    super_key: bool,
}

impl Modifiers {
    fn units(self) -> usize {
        usize::from(self.shift)
            + usize::from(self.ctrl)
            + usize::from(self.alt)
            + usize::from(self.super_key)
    }
}

#[derive(Clone, Copy)]
struct KeyMessage {
    key: &'static str,
    modifiers: Modifiers,
}

#[derive(Clone)]
struct StoredKey {
    key: &'static str,
    modifiers: Modifiers,
    created: Instant,
}

struct AppState {
    theme: Theme,
    history: VecDeque<StoredKey>,
    active_modifiers: Modifiers,
    caps_lock: bool,
}

struct Icons {
    backspace: Pixbuf,
    enter: Pixbuf,
    tab: Pixbuf,
    space: Pixbuf,
    caps_lock: Pixbuf,
    backspace_large: Pixbuf,
    enter_large: Pixbuf,
    tab_large: Pixbuf,
    space_large: Pixbuf,
    caps_lock_large: Pixbuf,
    shift: Pixbuf,
    ctrl: Pixbuf,
    alt: Pixbuf,
    super_key: Pixbuf,
}

impl Icons {
    fn new() -> Self {
        Self {
            backspace: load_svg(
                36,
                36,
                "0 0 24 24",
                r#"<path fill="currentColor" d="m11.4 16 2.6-2.6 2.6 2.6 1.4-1.4-2.6-2.6L18 9.4 16.6 8 14 10.6 11.4 8 10 9.4l2.6 2.6-2.6 2.6zM9 20q-.475 0-.9-.213t-.7-.587L2 12l5.4-7.2q.275-.375.7-.587T9 4h11q.825 0 1.413.587T22 6v12q0 .825-.587 1.413T20 20zm-4.5-8L9 18h11V6H9zm10 0"/>"#,
            ),
            enter: load_svg(
                36,
                36,
                "0 0 24 24",
                r#"<path fill="none" stroke="currentColor" stroke-linecap="square" stroke-width="2" d="M5.75 16H16a3 3 0 0 0 3-3V5M8 12.5 4.5 16 8 19.5"/>"#,
            ),
            tab: load_svg(
                36,
                36,
                "0 0 16 16",
                r#"<path fill="currentColor" d="m10.78 8.53-3.75 3.75a.749.749 0 1 1-1.06-1.06l2.469-2.47H1.75a.75.75 0 0 1 0-1.5h6.689L5.97 4.78a.749.749 0 1 1 1.06-1.06l3.75 3.75a.75.75 0 0 1 0 1.06M13 12.25v-8.5a.75.75 0 0 1 1.5 0v8.5a.75.75 0 0 1-1.5 0"/>"#,
            ),
            space: load_svg(
                36,
                36,
                "0 0 24 24",
                r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>"#,
            ),
            caps_lock: load_svg(
                36,
                36,
                "0 0 56 56",
                r#"<path fill="currentColor" d="M20.781 37.621h14.461c3.281 0 5.016-1.922 5.016-5.016v-4.148h8.882c1.946 0 3.493-1.148 3.493-2.953 0-1.102-.563-1.969-1.617-2.883L30.906 4.88c-.96-.844-1.851-1.406-2.906-1.406-1.031 0-1.922.562-2.883 1.406L4.984 22.645c-1.101.96-1.617 1.757-1.617 2.859 0 1.805 1.547 2.953 3.516 2.953h8.86v4.148c0 3.094 1.757 5.016 5.038 5.016m.375-3.539c-.89 0-1.5-.586-1.5-1.477v-6.89c0-.563-.21-.797-.773-.797H8.664c-.164 0-.234-.07-.234-.187a.33.33 0 0 1 .14-.282L27.508 7.996c.21-.187.328-.258.492-.258s.305.07.492.258L47.453 24.45a.33.33 0 0 1 .14.281c0 .118-.093.188-.257.188H37.14c-.563 0-.774.234-.774.797v6.89c0 .868-.656 1.477-1.5 1.477Zm-1.383 18.445h16.29c2.695 0 4.242-1.5 4.242-4.218v-3.375c0-2.72-1.547-4.266-4.243-4.266H19.773c-2.718 0-4.265 1.57-4.265 4.266v3.375c0 2.695 1.547 4.218 4.265 4.218m.54-3.304c-.82 0-1.266-.422-1.266-1.242v-2.72c0-.82.445-1.288 1.265-1.288h15.211c.797 0 1.242.468 1.242 1.289v2.718c0 .82-.445 1.243-1.242 1.243Z"/>"#,
            ),
            backspace_large: load_svg(
                44,
                44,
                "0 0 24 24",
                r#"<path fill="currentColor" d="m11.4 16 2.6-2.6 2.6 2.6 1.4-1.4-2.6-2.6L18 9.4 16.6 8 14 10.6 11.4 8 10 9.4l2.6 2.6-2.6 2.6zM9 20q-.475 0-.9-.213t-.7-.587L2 12l5.4-7.2q.275-.375.7-.587T9 4h11q.825 0 1.413.587T22 6v12q0 .825-.587 1.413T20 20zm-4.5-8L9 18h11V6H9zm10 0"/>"#,
            ),
            enter_large: load_svg(
                44,
                44,
                "0 0 24 24",
                r#"<path fill="none" stroke="currentColor" stroke-linecap="square" stroke-width="2" d="M5.75 16H16a3 3 0 0 0 3-3V5M8 12.5 4.5 16 8 19.5"/>"#,
            ),
            tab_large: load_svg(
                44,
                44,
                "0 0 16 16",
                r#"<path fill="currentColor" d="m10.78 8.53-3.75 3.75a.749.749 0 1 1-1.06-1.06l2.469-2.47H1.75a.75.75 0 0 1 0-1.5h6.689L5.97 4.78a.749.749 0 1 1 1.06-1.06l3.75 3.75a.75.75 0 0 1 0 1.06M13 12.25v-8.5a.75.75 0 0 1 1.5 0v8.5a.75.75 0 0 1-1.5 0"/>"#,
            ),
            space_large: load_svg(
                44,
                44,
                "0 0 24 24",
                r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>"#,
            ),
            caps_lock_large: load_svg(
                44,
                44,
                "0 0 56 56",
                r#"<path fill="currentColor" d="M20.781 37.621h14.461c3.281 0 5.016-1.922 5.016-5.016v-4.148h8.882c1.946 0 3.493-1.148 3.493-2.953 0-1.102-.563-1.969-1.617-2.883L30.906 4.88c-.96-.844-1.851-1.406-2.906-1.406-1.031 0-1.922.562-2.883 1.406L4.984 22.645c-1.101.96-1.617 1.757-1.617 2.859 0 1.805 1.547 2.953 3.516 2.953h8.86v4.148c0 3.094 1.757 5.016 5.038 5.016m.375-3.539c-.89 0-1.5-.586-1.5-1.477v-6.89c0-.563-.21-.797-.773-.797H8.664c-.164 0-.234-.07-.234-.187a.33.33 0 0 1 .14-.282L27.508 7.996c.21-.187.328-.258.492-.258s.305.07.492.258L47.453 24.45a.33.33 0 0 1 .14.281c0 .118-.093.188-.257.188H37.14c-.563 0-.774.234-.774.797v6.89c0 .868-.656 1.477-1.5 1.477Zm-1.383 18.445h16.29c2.695 0 4.242-1.5 4.242-4.218v-3.375c0-2.72-1.547-4.266-4.243-4.266H19.773c-2.718 0-4.265 1.57-4.265 4.266v3.375c0 2.695 1.547 4.218 4.265 4.218m.54-3.304c-.82 0-1.266-.422-1.266-1.242v-2.72c0-.82.445-1.288 1.265-1.288h15.211c.797 0 1.242.468 1.242 1.289v2.718c0 .82-.445 1.243-1.242 1.243Z"/>"#,
            ),
            shift: load_svg(
                26,
                22,
                "0 0 16 16",
                r#"<path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5 8 2.731 1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/>"#,
            ),
            ctrl: load_svg(
                29,
                29,
                "0 0 16 16",
                r#"<path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/>"#,
            ),
            alt: load_svg(
                26,
                22,
                "0 0 24 24",
                r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/>"#,
            ),
            super_key: load_svg(
                26,
                22,
                "0 0 24 24",
                r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13 1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14 1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/>"#,
            ),
        }
    }
}

fn load_svg(width: i32, height: i32, view_box: &str, body: &str) -> Pixbuf {
    let loader = PixbufLoader::with_type("svg").expect("SVG loader is unavailable");
    loader.set_size(width, height);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{view_box}" color="white">{body}</svg>"#
    );
    loader
        .write(svg.as_bytes())
        .expect("Could not render KeyRC icon");
    loader.close().expect("Could not finish KeyRC icon");
    loader.pixbuf().expect("Rendered KeyRC icon is empty")
}

fn draw_icon(context: &Context, icon: &Pixbuf, x: f64, y: f64, color: Color) {
    context.push_group();
    context.set_source_pixbuf(icon, x, y);
    let _ = context.paint();
    if let Ok(mask) = context.pop_group() {
        color.set(context);
        let _ = context.mask(mask);
    }
}

impl AppState {
    fn new(theme: Theme) -> Self {
        Self {
            theme,
            history: VecDeque::with_capacity(MAX_HISTORY),
            active_modifiers: Modifiers::default(),
            caps_lock: false,
        }
    }

    fn push_key(&mut self, message: KeyMessage) {
        if message.key == "CapsLock" {
            self.caps_lock = !self.caps_lock;
        }
        self.active_modifiers = message.modifiers;
        let reset = is_alias(message.key)
            || self
                .history
                .front()
                .is_some_and(|previous| is_alias(previous.key));
        if reset {
            self.history.clear();
        }
        self.history.push_front(StoredKey {
            key: message.key,
            modifiers: message.modifiers,
            created: Instant::now(),
        });
        self.history.truncate(MAX_HISTORY);
    }
}

enum UiMessage {
    Key(KeyMessage),
    Theme(Theme),
}

fn config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/keyrc/config.toml"))
}

fn cache_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".cache"))
        })
        .map(|cache| cache.join("keyrc/position"))
}

fn palette_color(table: Option<&toml::Table>, key: &str) -> Option<Color> {
    table
        .and_then(|values| values.get(key))
        .and_then(toml::Value::as_str)
        .and_then(Color::parse)
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
        if let Some(color) = palette_color(colors, key) {
            *target = color;
        }
    }
    theme
}

fn read_theme_at(path: Option<&Path>) -> Theme {
    path.and_then(|path| fs::read_to_string(path).ok())
        .map(|contents| parse_theme(&contents))
        .unwrap_or_default()
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
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, updated).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        error.to_string()
    })
}

fn toggle_mode() -> Result<(), String> {
    let path = config_path().ok_or("HOME is unavailable")?;
    let current = read_theme_at(Some(&path));
    write_mode(&path, current.mode.toggled())
}

fn theme_revision(path: Option<&Path>) -> Option<(SystemTime, u64)> {
    let metadata = path.and_then(|path| fs::metadata(path).ok())?;
    Some((metadata.modified().ok()?, metadata.len()))
}

fn start_theme_listener(sender: glib::Sender<UiMessage>) {
    thread::spawn(move || {
        let path = config_path();
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
                current = next.clone();
                if sender.send(UiMessage::Theme(next)).is_err() {
                    return;
                }
            }
        }
    });
}

fn parse_position(contents: &str) -> Option<(i32, i32)> {
    let mut lines = contents.lines();
    let x = lines.next()?.trim().parse().ok()?;
    let y = lines.next()?.trim().parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some((x, y))
}

fn read_position(path: Option<&Path>) -> Option<(i32, i32)> {
    path.and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| parse_position(&contents))
}

fn save_position(path: &Path, position: (i32, i32)) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, format!("{}\n{}\n", position.0, position.1))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn start_position_writer(path: PathBuf) -> Sender<(i32, i32)> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(mut position) = receiver.recv() {
            while let Ok(next) = receiver.recv_timeout(Duration::from_millis(150)) {
                position = next;
            }
            if let Err(error) = save_position(&path, position) {
                eprintln!("Could not save KeyRC position: {error}");
            }
        }
    });
    sender
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

fn start_keyboard_listener(sender: glib::Sender<UiMessage>) {
    thread::spawn(move || {
        let mut held = 0_u8;
        listen(move |event| match event.event_type {
            EventType::KeyPress(key) => match key {
                Key::ShiftLeft => held |= 1,
                Key::ShiftRight => held |= 2,
                Key::ControlLeft => held |= 4,
                Key::ControlRight => held |= 8,
                Key::Alt => held |= 16,
                Key::AltGr => held |= 32,
                Key::MetaLeft => held |= 64,
                Key::MetaRight => held |= 128,
                key => {
                    let key = key_to_string(key);
                    if key.is_empty() {
                        return;
                    }
                    let modifiers = Modifiers {
                        shift: held & 3 != 0,
                        ctrl: held & 12 != 0,
                        alt: held & 48 != 0,
                        super_key: held & 192 != 0,
                    };
                    let _ = sender.send(UiMessage::Key(KeyMessage { key, modifiers }));
                }
            },
            EventType::KeyRelease(key) => match key {
                Key::ShiftLeft => held &= !1,
                Key::ShiftRight => held &= !2,
                Key::ControlLeft => held &= !4,
                Key::ControlRight => held &= !8,
                Key::Alt => held &= !16,
                Key::AltGr => held &= !32,
                Key::MetaLeft => held &= !64,
                Key::MetaRight => held &= !128,
                _ => {}
            },
            _ => {}
        })
        .expect("Could not listen to events");
    });
}

fn rounded_panel(context: &Context, x: f64, y: f64, width: f64, height: f64, corners: u8) {
    let right = x + width;
    let bottom = y + height;
    let radius = CORNER_RADIUS.min(width / 2.0).min(height / 2.0);
    context.new_sub_path();
    context.move_to(x + if corners & 1 != 0 { radius } else { 0.0 }, y);
    context.line_to(right - if corners & 2 != 0 { radius } else { 0.0 }, y);
    if corners & 2 != 0 {
        context.arc(right - radius, y + radius, radius, -FRAC_PI_2, 0.0);
    }
    context.line_to(right, bottom - if corners & 4 != 0 { radius } else { 0.0 });
    if corners & 4 != 0 {
        context.arc(right - radius, bottom - radius, radius, 0.0, FRAC_PI_2);
    }
    context.line_to(x + if corners & 8 != 0 { radius } else { 0.0 }, bottom);
    if corners & 8 != 0 {
        context.arc(x + radius, bottom - radius, radius, FRAC_PI_2, PI);
    }
    context.line_to(x, y + if corners & 1 != 0 { radius } else { 0.0 });
    if corners & 1 != 0 {
        context.arc(x + radius, y + radius, radius, PI, PI + FRAC_PI_2);
    }
    context.close_path();
}

fn fill_panel(
    context: &Context,
    theme: &Theme,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corners: u8,
) {
    rounded_panel(context, x, y, width, height, corners);
    theme.background.set(context);
    let _ = context.fill_preserve();
    theme.border.set(context);
    context.set_line_width(1.0);
    let _ = context.stroke();
}

fn text_layout(context: &Context, text: &str, size: f64) -> Layout {
    let layout: Layout =
        unsafe { from_glib_full(pango_cairo_create_layout(context.to_raw_none())) };
    let mut description = FontDescription::from_string(FONT);
    // CSS font sizes are pixels. Absolute Pango sizing preserves that mapping
    // instead of interpreting these values as desktop-scaled points.
    description.set_absolute_size(size * f64::from(gtk::pango::SCALE));
    layout.set_font_description(Some(&description));
    layout.set_text(text);
    layout
}

fn text_width(context: &Context, text: &str, size: f64) -> f64 {
    f64::from(text_layout(context, text, size).pixel_size().0)
}

fn draw_centered_text(context: &Context, text: &str, center_x: f64, center_y: f64, size: f64) {
    let layout = text_layout(context, text, size);
    let (width, height) = layout.pixel_size();
    // Center the shared font line box so every key has one baseline. Centering
    // each glyph's ink bounds makes lowercase letters visibly jump around.
    context.move_to(
        center_x - f64::from(width) / 2.0,
        center_y - f64::from(height) / 2.0,
    );
    unsafe {
        pango_cairo_show_layout(context.to_raw_none(), layout.to_glib_none().0);
    }
}

fn alias(key: &str) -> &str {
    match key {
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "Insert" => "Ins",
        "PrintScreen" => "PrtSc",
        "ScrollLock" => "ScrLk",
        "NumLock" => "Num",
        "Escape" => "Esc",
        "Delete" => "Del",
        _ => key,
    }
}

fn is_alias(key: &str) -> bool {
    alias(key) != key
}
fn is_arrow(key: &str) -> bool {
    matches!(key, "" | "" | "" | "")
}
fn is_svg_key(key: &str) -> bool {
    matches!(key, "Backspace" | "Enter" | "Tab" | "Space" | "CapsLock")
}

fn display_text(key: &str, caps_lock: bool) -> String {
    let key = alias(key);
    if key.len() == 1 && key.as_bytes()[0].is_ascii_uppercase() && !caps_lock {
        key.to_ascii_lowercase()
    } else {
        key.to_owned()
    }
}

fn key_width(context: &Context, key: &StoredKey, mode: DisplayMode, caps_lock: bool) -> f64 {
    let mut width = 0.0;
    if mode == DisplayMode::KeysOnly {
        let count = key.modifiers.units();
        if count > 0 {
            width += count as f64 * 36.0 + count as f64 * 4.0;
        }
    }
    width
        + if is_svg_key(key.key) {
            if mode == DisplayMode::KeysOnly {
                44.0
            } else {
                36.0
            }
        } else {
            let size = if is_arrow(key.key) { 44.0 } else { 36.0 };
            text_width(context, &display_text(key.key, caps_lock), size)
        }
}

fn draw_special_key(
    context: &Context,
    icons: &Icons,
    key: &str,
    x: f64,
    center_y: f64,
    large: bool,
    color: Color,
) {
    let icon = match (key, large) {
        ("Backspace", false) => &icons.backspace,
        ("Enter", false) => &icons.enter,
        ("Tab", false) => &icons.tab,
        ("Space", false) => &icons.space,
        ("CapsLock", false) => &icons.caps_lock,
        ("Backspace", true) => &icons.backspace_large,
        ("Enter", true) => &icons.enter_large,
        ("Tab", true) => &icons.tab_large,
        ("Space", true) => &icons.space_large,
        ("CapsLock", true) => &icons.caps_lock_large,
        _ => return,
    };
    draw_icon(
        context,
        icon,
        x,
        center_y - f64::from(icon.height()) / 2.0,
        color,
    );
}

fn draw_modifier(context: &Context, icons: &Icons, modifier: &str, x: f64, y: f64, color: Color) {
    let icon = match modifier {
        "Shift" => &icons.shift,
        "Ctrl" => &icons.ctrl,
        "Alt" => &icons.alt,
        "Super" => &icons.super_key,
        _ => return,
    };
    draw_icon(
        context,
        icon,
        x - f64::from(icon.width()) / 2.0,
        y - f64::from(icon.height()) / 2.0,
        color,
    );
}

fn draw_key_token(
    context: &Context,
    icons: &Icons,
    key: &StoredKey,
    state: &AppState,
    x: f64,
    width: f64,
) {
    let progress = (key.created.elapsed().as_secs_f64() / 0.14).min(1.0);
    let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
    let center_x = x + width / 2.0;
    let center_y = f64::from(KEY_HEIGHT) / 2.0 + 4.0 * (1.0 - eased);
    context.save().ok();
    context.translate(center_x, center_y);
    context.scale(0.92 + 0.08 * eased, 0.92 + 0.08 * eased);
    context.translate(-center_x, -center_y);
    state.theme.key_text.with_alpha(eased).set(context);
    let mut cursor = x;
    if state.theme.mode == DisplayMode::KeysOnly {
        for (active, name) in [
            (key.modifiers.super_key, "Super"),
            (key.modifiers.ctrl, "Ctrl"),
            (key.modifiers.alt, "Alt"),
            (key.modifiers.shift, "Shift"),
        ] {
            if active {
                draw_modifier(
                    context,
                    icons,
                    name,
                    cursor + 18.0,
                    center_y,
                    state.theme.key_text.with_alpha(eased),
                );
                cursor += 40.0;
            }
        }
    }
    if is_svg_key(key.key) {
        draw_special_key(
            context,
            icons,
            key.key,
            cursor,
            center_y,
            state.theme.mode == DisplayMode::KeysOnly,
            state.theme.key_text.with_alpha(eased),
        );
    } else {
        let size = if is_arrow(key.key) { 44.0 } else { 36.0 };
        let text = display_text(key.key, state.caps_lock);
        draw_centered_text(
            context,
            &text,
            cursor + text_width(context, &text, size) / 2.0,
            center_y,
            size,
        );
    }
    context.restore().ok();
}

fn draw_modifier_row(context: &Context, icons: &Icons, state: &AppState) {
    let panels = [
        ("Shift", state.active_modifiers.shift),
        ("Ctrl", state.active_modifiers.ctrl),
        ("Alt", state.active_modifiers.alt),
        ("Super", state.active_modifiers.super_key),
    ];
    for (index, (name, active)) in panels.into_iter().enumerate() {
        let x = index as f64 * 73.0;
        let corners = match index {
            0 => 8,
            3 => 4,
            _ => 0,
        };
        // Keep the one-pixel stroke fully inside the native surface. A path on
        // the exact bottom/right edge loses half of its border to clipping.
        rounded_panel(
            context,
            x + 0.5,
            MODIFIER_Y + 0.5,
            70.0,
            MODIFIER_HEIGHT - 1.0,
            corners,
        );
        if active {
            state.theme.active_bg.set(context);
        } else {
            state.theme.background.set(context);
        }
        let _ = context.fill_preserve();
        state.theme.border.set(context);
        context.set_line_width(1.0);
        let _ = context.stroke();
        let color = if active {
            state.theme.active_fg
        } else {
            state.theme.key_text.with_alpha(0.35)
        };
        draw_modifier(
            context,
            icons,
            name,
            x + 35.5,
            MODIFIER_Y + MODIFIER_HEIGHT / 2.0,
            color,
        );
    }
}

fn draw_ui(context: &Context, icons: &Icons, state: &AppState) {
    context.set_operator(Operator::Source);
    context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    let _ = context.paint();
    context.set_operator(Operator::Over);
    fill_panel(
        context,
        &state.theme,
        0.5,
        0.5,
        f64::from(WIDTH) - 1.0,
        f64::from(KEY_HEIGHT) - 1.0,
        if state.theme.mode == DisplayMode::KeysOnly {
            15
        } else {
            3
        },
    );
    let mut shown = Vec::new();
    let mut units = 0;
    for key in &state.history {
        shown.push(key);
        units += 1;
        if state.theme.mode == DisplayMode::KeysOnly {
            units += key.modifiers.units();
        }
        if units >= MAX_DISPLAY_UNITS {
            break;
        }
    }
    shown.reverse();
    let widths: Vec<f64> = shown
        .iter()
        .map(|key| key_width(context, key, state.theme.mode, state.caps_lock))
        .collect();
    let total = widths.iter().sum::<f64>() + 4.0 * widths.len().saturating_sub(1) as f64;
    let mut x = (f64::from(WIDTH) - total) / 2.0;
    for (key, width) in shown.into_iter().zip(widths) {
        draw_key_token(context, icons, key, state, x, width);
        x += width + 4.0;
    }
    if state.theme.mode == DisplayMode::Full {
        draw_modifier_row(context, icons, state);
    }
}

fn setup_tray() -> AppIndicator {
    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Toggle mode");
    toggle.connect_activate(|_| {
        if let Err(error) = toggle_mode() {
            eprintln!("Could not toggle KeyRC mode: {error}");
        }
    });
    menu.append(&toggle);
    menu.append(&gtk::SeparatorMenuItem::new());
    let quit = gtk::MenuItem::with_label("Quit");
    quit.connect_activate(|_| gtk::main_quit());
    menu.append(&quit);
    menu.show_all();
    let mut indicator = AppIndicator::new("keyrc", "input-keyboard");
    indicator.set_title("KeyRC");
    indicator.set_menu(&mut menu);
    indicator.set_status(AppIndicatorStatus::Active);
    indicator
}

fn configure_window(window: &gtk::Window, height: i32) {
    window.set_title("keyrc");
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_keep_above(true);
    window.stick();
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_accept_focus(false);
    window.set_focus_on_map(false);
    window.set_deletable(false);
    window.set_app_paintable(true);
    window.set_default_size(WIDTH, height);
    window.set_size_request(WIDTH, height);
    if let Some(screen) = WidgetExt::screen(window) {
        if let Some(visual) = screen.rgba_visual() {
            window.set_visual(Some(&visual));
        }
    }
}

pub fn run() {
    gtk::init().expect("Could not initialize GTK");
    let initial_theme = read_theme_at(config_path().as_deref());
    #[cfg(debug_assertions)]
    let initial_theme = {
        let mut theme = initial_theme;
        match std::env::var("KEYRC_PREVIEW_MODE").as_deref() {
            Ok("keys_only") => theme.mode = DisplayMode::KeysOnly,
            Ok("full") => theme.mode = DisplayMode::Full,
            _ => {}
        }
        theme
    };
    let icons = Rc::new(Icons::new());
    let initial_state = AppState::new(initial_theme.clone());
    #[cfg(debug_assertions)]
    let initial_state = {
        let mut state = initial_state;
        if let Ok(keys) = std::env::var("KEYRC_PREVIEW_KEYS") {
            for specification in keys.split(',') {
                let mut parts = specification.split('+').collect::<Vec<_>>();
                let raw_key = parts.pop().unwrap_or_default();
                let key = match raw_key {
                    "Right" => "",
                    "Left" => "",
                    "Up" => "",
                    "Down" => "",
                    "Enter" => "Enter",
                    other => Box::leak(other.to_owned().into_boxed_str()),
                };
                let modifiers = Modifiers {
                    shift: parts.contains(&"Shift"),
                    ctrl: parts.contains(&"Ctrl"),
                    alt: parts.contains(&"Alt"),
                    super_key: parts.contains(&"Super"),
                };
                state.push_key(KeyMessage { key, modifiers });
            }
            for key in &mut state.history {
                key.created = Instant::now() - Duration::from_secs(1);
            }
        }
        state
    };
    let state = Rc::new(RefCell::new(initial_state));
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    configure_window(&window, initial_theme.mode.height());
    let position_path = cache_path();
    if let Some((x, y)) = read_position(position_path.as_deref()) {
        window.move_(x, y);
    }
    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_size_request(WIDTH, initial_theme.mode.height());
    drawing_area.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    window.add(&drawing_area);
    let draw_state = Rc::clone(&state);
    let draw_icons = Rc::clone(&icons);
    drawing_area.connect_draw(move |_, context| {
        draw_ui(context, &draw_icons, &draw_state.borrow());
        glib::Propagation::Stop
    });
    let drag_window = window.clone();
    drawing_area.connect_button_press_event(move |_, event| {
        if event.button() == 1 {
            let (x, y) = event.root();
            drag_window.begin_move_drag(1, x as i32, y as i32, event.time());
        }
        glib::Propagation::Stop
    });
    window.connect_delete_event(|_, _| glib::Propagation::Stop);
    let position_ready = Rc::new(Cell::new(false));
    if let Some(path) = position_path {
        let position_sender = start_position_writer(path);
        let position_ready = Rc::clone(&position_ready);
        window.connect_configure_event(move |_, event| {
            if position_ready.get() {
                let _ = position_sender.send(event.position());
            }
            false
        });
    }
    #[allow(deprecated)]
    let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
    start_keyboard_listener(sender.clone());
    start_theme_listener(sender);
    let animation_running = Rc::new(Cell::new(false));
    let receiver_state = Rc::clone(&state);
    let receiver_area = drawing_area.clone();
    let receiver_window = window.clone();
    receiver.attach(None, move |message| {
        match message {
            UiMessage::Key(message) => {
                receiver_state.borrow_mut().push_key(message);
                receiver_area.queue_draw();
                if !animation_running.replace(true) {
                    let area = receiver_area.clone();
                    let animation_running = Rc::clone(&animation_running);
                    let state = Rc::clone(&receiver_state);
                    glib::timeout_add_local(Duration::from_millis(16), move || {
                        area.queue_draw();
                        let animating = state
                            .borrow()
                            .history
                            .iter()
                            .any(|key| key.created.elapsed() < Duration::from_millis(140));
                        if !animating {
                            animation_running.set(false);
                        }
                        ControlFlow::from(animating)
                    });
                }
            }
            UiMessage::Theme(theme) => {
                let old_mode = receiver_state.borrow().theme.mode;
                let new_mode = theme.mode;
                receiver_state.borrow_mut().theme = theme;
                if new_mode != old_mode {
                    let height = new_mode.height();
                    receiver_area.set_size_request(WIDTH, height);
                    receiver_window.set_size_request(WIDTH, height);
                    receiver_window.resize(WIDTH, height);
                }
                receiver_area.queue_draw();
            }
        }
        ControlFlow::Continue
    });
    let _tray = setup_tray();
    window.show_all();
    // Apply this again after mapping because some window managers ignore the
    // initial sticky hint. KeyRC should remain visible on every workspace.
    window.stick();
    window.set_accept_focus(false);
    if let Some(native) = window.window() {
        native.stick();
        native.set_accept_focus(false);
    }
    // Ignore startup configure events, which may report (0, 0) before the
    // window manager applies the cached position.
    glib::timeout_add_local_once(Duration::from_millis(500), move || {
        position_ready.set(true);
    });
    gtk::main();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_and_mode() {
        let theme = parse_theme(
            r##"mode = "keys_only"
[colors]
active_bg = "#112233"
active_fg = "#aabbccdd"
key_text = "invalid"
background = "#010203"
border = "#445566"
"##,
        );
        assert!(theme.mode == DisplayMode::KeysOnly);
        assert!(theme.active_bg == Color::rgb(0x11, 0x22, 0x33));
        assert_eq!(theme.active_fg.alpha, 0xdd as f64 / 255.0);
        assert!(theme.key_text == Color::rgb(255, 255, 255));
    }

    #[test]
    fn parses_position_strictly() {
        assert_eq!(parse_position("12\n-8\n"), Some((12, -8)));
        assert_eq!(parse_position("12\n-8\nextra\n"), None);
    }

    #[test]
    fn caps_lock_changes_letters_without_shift() {
        assert_eq!(display_text("A", false), "a");
        assert_eq!(display_text("A", true), "A");
    }
}
