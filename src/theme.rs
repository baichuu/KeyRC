use crate::model::{Color, DisplayMode, Theme, UiMessage};
use gtk::glib;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

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

fn revision(path: Option<&Path>) -> Option<(SystemTime, u64)> {
    let metadata = path.and_then(|path| fs::metadata(path).ok())?;
    Some((metadata.modified().ok()?, metadata.len()))
}

pub(crate) fn start_listener(sender: glib::Sender<UiMessage>) {
    thread::spawn(move || {
        let path = path();
        let mut revision = revision(path.as_deref());
        let mut current = read(path.as_deref());
        loop {
            thread::sleep(Duration::from_millis(250));
            let next_revision = self::revision(path.as_deref());
            if next_revision == revision {
                continue;
            }
            revision = next_revision;
            let next = read(path.as_deref());
            if next != current {
                current = next.clone();
                if sender.send(UiMessage::Theme(next)).is_err() {
                    return;
                }
            }
        }
    });
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
