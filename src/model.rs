use gtk::cairo::Context;
use std::collections::VecDeque;
use std::time::Instant;

pub(crate) const WIDTH: i32 = 290;
pub(crate) const KEY_HEIGHT: i32 = 70;
pub(crate) const FULL_HEIGHT: i32 = 114;
pub(crate) const MODIFIER_Y: f64 = 72.0;
pub(crate) const MODIFIER_HEIGHT: f64 = 42.0;
pub(crate) const MAX_HISTORY: usize = 6;
pub(crate) const MAX_DISPLAY_UNITS: usize = 6;

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) enum DisplayMode {
    #[default]
    Full,
    KeysOnly,
}

impl DisplayMode {
    pub(crate) fn height(self) -> i32 {
        match self {
            Self::Full => FULL_HEIGHT,
            Self::KeysOnly => KEY_HEIGHT,
        }
    }

    pub(crate) fn config_value(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::KeysOnly => "keys_only",
        }
    }

    pub(crate) fn toggled(self) -> Self {
        match self {
            Self::Full => Self::KeysOnly,
            Self::KeysOnly => Self::Full,
        }
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct Theme {
    pub(crate) mode: DisplayMode,
    pub(crate) active_bg: Color,
    pub(crate) active_fg: Color,
    pub(crate) key_text: Color,
    pub(crate) background: Color,
    pub(crate) border: Color,
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
pub(crate) struct Color {
    pub(crate) red: f64,
    pub(crate) green: f64,
    pub(crate) blue: f64,
    pub(crate) alpha: f64,
}

impl Color {
    pub(crate) const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red: red as f64 / 255.0,
            green: green as f64 / 255.0,
            blue: blue as f64 / 255.0,
            alpha: 1.0,
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
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

    pub(crate) fn with_alpha(self, alpha: f64) -> Self {
        Self {
            alpha: self.alpha * alpha,
            ..self
        }
    }

    pub(crate) fn set(self, context: &Context) {
        context.set_source_rgba(self.red, self.green, self.blue, self.alpha);
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Modifiers {
    pub(crate) shift: bool,
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) super_key: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Shortcut {
    pub(crate) modifiers: Modifiers,
    pub(crate) key: String,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Keymap {
    pub(crate) toggle_mode: Shortcut,
    pub(crate) quit: Shortcut,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            toggle_mode: Shortcut::default(),
            quit: Shortcut {
                modifiers: Modifiers {
                    ctrl: true,
                    alt: true,
                    ..Modifiers::default()
                },
                key: "Q".into(),
            },
        }
    }
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            modifiers: Modifiers {
                ctrl: true,
                alt: true,
                ..Modifiers::default()
            },
            key: "M".into(),
        }
    }
}

impl Shortcut {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let mut modifiers = Modifiers::default();
        let mut key = None;
        for part in value.split('+') {
            let part = part.trim();
            if part.eq_ignore_ascii_case("shift") {
                modifiers.shift = true;
            } else if part.eq_ignore_ascii_case("ctrl") || part.eq_ignore_ascii_case("control") {
                modifiers.ctrl = true;
            } else if part.eq_ignore_ascii_case("alt") {
                modifiers.alt = true;
            } else if part.eq_ignore_ascii_case("super")
                || part.eq_ignore_ascii_case("meta")
                || part.eq_ignore_ascii_case("win")
            {
                modifiers.super_key = true;
            } else if part.is_empty() || key.is_some() {
                return None;
            } else {
                key = Some(part.to_owned());
            }
        }
        Some(Self {
            modifiers,
            key: key?,
        })
    }

    pub(crate) fn matches(&self, key: &str, modifiers: Modifiers) -> bool {
        self.key.eq_ignore_ascii_case(key) && self.modifiers == modifiers
    }

    pub(crate) fn modifier_held(&self, modifiers: Modifiers) -> bool {
        (!self.modifiers.shift || modifiers.shift)
            && (!self.modifiers.ctrl || modifiers.ctrl)
            && (!self.modifiers.alt || modifiers.alt)
            && (!self.modifiers.super_key || modifiers.super_key)
    }

    pub(crate) fn has_modifiers(&self) -> bool {
        self.modifiers.units() != 0
    }
}

impl Modifiers {
    pub(crate) fn units(self) -> usize {
        usize::from(self.shift)
            + usize::from(self.ctrl)
            + usize::from(self.alt)
            + usize::from(self.super_key)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct KeyMessage {
    pub(crate) key: &'static str,
}

#[derive(Clone)]
pub(crate) struct StoredKey {
    pub(crate) key: &'static str,
    pub(crate) created: Instant,
}

pub(crate) struct AppState {
    pub(crate) theme: Theme,
    pub(crate) history: VecDeque<StoredKey>,
    pub(crate) active_modifiers: Modifiers,
    pub(crate) caps_lock: bool,
}

impl AppState {
    pub(crate) fn new(theme: Theme) -> Self {
        Self {
            theme,
            history: VecDeque::with_capacity(MAX_HISTORY),
            active_modifiers: Modifiers::default(),
            caps_lock: false,
        }
    }

    pub(crate) fn push_key(&mut self, message: KeyMessage) {
        if message.key == "CapsLock" {
            self.caps_lock = !self.caps_lock;
        }
        self.history.push_front(StoredKey {
            key: message.key,
            created: Instant::now(),
        });
        self.history.truncate(MAX_HISTORY);
    }
}

pub(crate) enum UiMessage {
    Key(KeyMessage),
    Modifiers(Modifiers),
    Theme(Theme),
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_history_does_not_expand_combination_modifiers() {
        for mode in [DisplayMode::Full, DisplayMode::KeysOnly] {
            let mut state = AppState::new(Theme {
                mode,
                ..Theme::default()
            });
            for key in ["Ctrl", "Alt", "M"] {
                state.push_key(KeyMessage { key });
            }
            let history: Vec<_> = state.history.iter().rev().map(|key| key.key).collect();
            assert_eq!(history, ["Ctrl", "Alt", "M"]);
        }
    }

    #[test]
    fn special_keys_keep_their_combination_modifiers() {
        let mut state = AppState::new(Theme::default());
        state.push_key(KeyMessage { key: "Super" });
        state.push_key(KeyMessage { key: "Escape" });
        let history: Vec<_> = state.history.iter().rev().map(|key| key.key).collect();
        assert_eq!(history, ["Super", "Escape"]);
    }
}
