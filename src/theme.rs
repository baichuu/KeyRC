use crate::model::{Color, DisplayMode, Theme, UiMessage};
use gtk::gio::{self, prelude::*};
use gtk::glib;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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

fn parse(contents: &str) -> Theme {
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

pub(crate) fn read(path: Option<&Path>) -> Theme {
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

pub(crate) fn toggle_mode() -> Result<(), String> {
    let path = path().ok_or("HOME is unavailable")?;
    let current = read(Some(&path));
    write_mode(&path, current.mode.toggled())
}

pub(crate) fn start_listener(sender: glib::Sender<UiMessage>) -> Option<gio::FileMonitor> {
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
            *current.borrow_mut() = next.clone();
            let _ = sender.send(UiMessage::Theme(next));
        }
    });
    Some(monitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_and_mode() {
        let theme = parse(
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
}
