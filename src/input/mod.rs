mod evdev;
mod x11;

use crate::model::{KeyMessage, Modifiers, Shortcut, UiMessage};
use crate::platform::Backend;
use crate::theme;
use gtk::glib;
use std::sync::{Arc, RwLock};

pub(crate) const SHIFT_LEFT: u8 = 1 << 0;
pub(crate) const SHIFT_RIGHT: u8 = 1 << 1;
pub(crate) const CTRL_LEFT: u8 = 1 << 2;
pub(crate) const CTRL_RIGHT: u8 = 1 << 3;
pub(crate) const ALT_LEFT: u8 = 1 << 4;
pub(crate) const ALT_RIGHT: u8 = 1 << 5;
pub(crate) const SUPER_LEFT: u8 = 1 << 6;
pub(crate) const SUPER_RIGHT: u8 = 1 << 7;

pub(crate) fn start_listener(
    backend: Backend,
    sender: glib::Sender<UiMessage>,
    shortcut: Arc<RwLock<Shortcut>>,
) {
    match backend {
        Backend::X11 => x11::start(sender, shortcut),
        Backend::Wayland => evdev::start(sender, shortcut),
    }
}

pub(crate) fn modifiers(held: u8) -> Modifiers {
    Modifiers {
        shift: held & (SHIFT_LEFT | SHIFT_RIGHT) != 0,
        ctrl: held & (CTRL_LEFT | CTRL_RIGHT) != 0,
        alt: held & (ALT_LEFT | ALT_RIGHT) != 0,
        super_key: held & (SUPER_LEFT | SUPER_RIGHT) != 0,
    }
}

pub(crate) struct Processor {
    sender: glib::Sender<UiMessage>,
    shortcut: Arc<RwLock<Shortcut>>,
    toggle_latched: bool,
}

impl Processor {
    pub(crate) fn new(sender: glib::Sender<UiMessage>, shortcut: Arc<RwLock<Shortcut>>) -> Self {
        Self {
            sender,
            shortcut,
            toggle_latched: false,
        }
    }

    pub(crate) fn press_modifier(&mut self, key: &'static str, held: u8) {
        let _ = self.sender.send(UiMessage::Modifiers(modifiers(held)));
        let _ = self.sender.send(UiMessage::Key(KeyMessage { key }));
    }

    pub(crate) fn release_modifier(&mut self, held: u8) {
        let modifiers = modifiers(held);
        if self
            .shortcut
            .read()
            .is_ok_and(|active| !active.modifier_held(modifiers))
        {
            self.toggle_latched = false;
        }
        let _ = self.sender.send(UiMessage::Modifiers(modifiers));
    }

    pub(crate) fn press_key(&mut self, key: &'static str, held: u8) {
        let toggle = self
            .shortcut
            .read()
            .is_ok_and(|active| active.matches(key, modifiers(held)));
        if toggle && !self.toggle_latched {
            self.toggle_latched = true;
            if let Err(error) = theme::toggle_mode() {
                eprintln!("Could not toggle KeyRC mode: {error}");
            }
        }
        let _ = self.sender.send(UiMessage::Key(KeyMessage { key }));
    }

    pub(crate) fn release_key(&mut self, key: &str) {
        if self
            .shortcut
            .read()
            .is_ok_and(|active| !active.has_modifiers() && active.key.eq_ignore_ascii_case(key))
        {
            self.toggle_latched = false;
        }
    }
}
