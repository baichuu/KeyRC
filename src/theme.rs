use crate::model::{Color, DisplayMode, Keymap, Shortcut, Theme, UiMessage};
use gtk::gio::{self, prelude::*};
use gtk::glib;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default, PartialEq)]
pub(crate) struct Settings {
    pub(crate) theme: Theme,
    pub(crate) keymap: Keymap,
}

pub(crate) fn path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/keyrc/config.toml"))
}

fn palette_color(table: Option<&toml::Table>, key: &str) -> Option<Color> {
    table
        .and_then(|values| values.get(key))
        .and_then(toml::Value::as_str)
        .and_then(Color::parse)
}

fn opacity(table: Option<&toml::Table>) -> Option<f64> {
    let value = table?.get("opacity")?;
    let value = value
        .as_float()
        .or_else(|| value.as_integer().map(|value| value as f64))?;
    (0.0..=1.0).contains(&value).then_some(value)
}

fn parse(contents: &str) -> Settings {
    let mut settings = Settings::default();
    let Ok(config) = toml::from_str::<toml::Table>(contents) else {
        return settings;
    };
    let general = config.get("general").and_then(toml::Value::as_table);
    let mode = general
        .and_then(|general| general.get("mode"))
        .or_else(|| config.get("mode"))
        .and_then(toml::Value::as_str);
    if mode == Some("keys_only") {
        settings.theme.mode = DisplayMode::KeysOnly;
    }
    if let Some(opacity) = opacity(general) {
        settings.theme.opacity = opacity;
    }
    let colors = config.get("colors").and_then(toml::Value::as_table);
    for (key, target) in [
        ("active_fg", &mut settings.theme.active_fg),
        ("key_text", &mut settings.theme.key_text),
        ("background", &mut settings.theme.background),
        ("border", &mut settings.theme.border),
    ] {
        if let Some(color) = palette_color(colors, key) {
            *target = color;
        }
    }
    let keymap = config.get("keymap").and_then(toml::Value::as_table);
    settings.keymap.toggle_mode = keymap
        .and_then(|values| values.get("toggle_mode"))
        .and_then(toml::Value::as_str)
        .and_then(Shortcut::parse)
        .unwrap_or_default();
    settings.keymap.quit = keymap
        .and_then(|values| values.get("quit"))
        .and_then(toml::Value::as_str)
        .and_then(Shortcut::parse)
        .unwrap_or_else(|| Keymap::default().quit);
    settings
}

pub(crate) fn read(path: Option<&Path>) -> Settings {
    path.and_then(|path| fs::read_to_string(path).ok())
        .map(|contents| parse(&contents))
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
        lines.remove(index);
    }
    let general = lines.iter().position(|line| line.trim() == "[general]");
    if let Some(start) = general {
        let end = lines[start + 1..]
            .iter()
            .position(|line| line.trim_start().starts_with('['))
            .map_or(lines.len(), |offset| start + 1 + offset);
        if let Some(index) = lines[start + 1..end].iter().position(|line| {
            line.split_once('=')
                .is_some_and(|(key, _)| key.trim() == "mode")
        }) {
            lines[start + 1 + index] = replacement;
        } else {
            lines.insert(start + 1, replacement);
        }
    } else {
        lines.insert(0, "[general]".into());
        lines.insert(1, replacement);
        lines.insert(2, String::new());
    }
    let updated = lines.join("\n") + "\n";
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temporary, updated).map_err(|error| error.to_string())?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        error.to_string()
    })
}

pub(crate) fn toggle_mode() -> Result<(), String> {
    let path = path().ok_or("HOME is unavailable")?;
    let current = read(Some(&path));
    write_mode(&path, current.theme.mode.toggled())
}

pub(crate) fn start_listener(
    sender: glib::Sender<UiMessage>,
    keymap: Arc<RwLock<Keymap>>,
) -> Option<gio::FileMonitor> {
    let path = path()?;
    let directory = path.parent()?;
    fs::create_dir_all(directory).ok()?;
    let monitor = gio::File::for_path(directory)
        .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        .ok()?;
    monitor.set_rate_limit(50);

    let current = Rc::new(RefCell::new(read(Some(&path))));
    monitor.connect_changed(move |_, changed, other, _| {
        let is_config = |file: &gio::File| file.path().as_deref() == Some(path.as_path());
        if !is_config(changed) && !other.is_some_and(is_config) {
            return;
        }
        let next = read(Some(&path));
        if next != *current.borrow() {
            if let Ok(mut active) = keymap.write() {
                active.clone_from(&next.keymap);
            }
            if next.theme != current.borrow().theme {
                let _ = sender.send(UiMessage::Theme(next.theme.clone()));
            }
            *current.borrow_mut() = next.clone();
        }
    });
    Some(monitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_and_mode() {
        let settings = parse(
            r##"[general]
mode = "keys_only"
opacity = 0.8

[keymap]
toggle_mode = "Super+Shift+K"
quit = "Ctrl+Escape"
[colors]
active_fg = "#aabbccdd"
key_text = "invalid"
background = "#010203"
border = "#445566"
"##,
        );
        let theme = settings.theme;
        assert!(theme.mode == DisplayMode::KeysOnly);
        assert_eq!(theme.opacity, 0.8);
        assert_eq!(theme.active_fg.alpha, 0xdd as f64 / 255.0);
        assert!(theme.key_text == Color::rgb(255, 255, 255));
        assert!(settings.keymap.toggle_mode.key == "K");
        assert!(settings.keymap.toggle_mode.modifiers.shift);
        assert!(settings.keymap.toggle_mode.modifiers.super_key);
        assert!(settings.keymap.quit.key == "Escape");
        assert!(settings.keymap.quit.modifiers.ctrl);
    }

    #[test]
    fn uses_default_quit_shortcut_when_missing() {
        let settings = parse("");
        assert_eq!(settings.theme.opacity, 1.0);
        assert_eq!(settings.keymap.quit.key, "Q");
        assert!(settings.keymap.quit.modifiers.ctrl);
        assert!(settings.keymap.quit.modifiers.alt);
    }

    #[test]
    fn rejects_opacity_outside_zero_to_one() {
        for value in ["-0.1", "1.1", "\"0.5\""] {
            let settings = parse(&format!("[general]\nopacity = {value}\n"));
            assert_eq!(settings.theme.opacity, 1.0);
        }
    }
}
