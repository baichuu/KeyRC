mod evdev;
mod x11;

use crate::model::{KeyMessage, Keymap, Modifiers, UiMessage};
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
    keymap: Arc<RwLock<Keymap>>,
) {
    match backend {
        Backend::X11 => x11::start(sender, keymap),
        Backend::Wayland => evdev::start(sender, keymap),
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
    keymap: Arc<RwLock<Keymap>>,
    toggle_latched: bool,
    held: u8,
    chord_complete: bool,
}

impl Processor {
    pub(crate) fn new(sender: glib::Sender<UiMessage>, keymap: Arc<RwLock<Keymap>>) -> Self {
        Self {
            sender,
            keymap,
            toggle_latched: false,
            held: 0,
            chord_complete: false,
        }
    }

    pub(crate) fn press_modifier(&mut self, key: &'static str, held: u8) {
        if self.chord_complete {
            self.send_held_modifiers(self.held);
            self.chord_complete = false;
        }
        self.held = held;
        let _ = self.sender.send(UiMessage::Modifiers(modifiers(held)));
        self.send_key(key);
    }

    pub(crate) fn release_modifier(&mut self, held: u8) {
        let modifiers = modifiers(held);
        self.held = held;
        if held == 0 {
            self.chord_complete = false;
        }
        if self
            .keymap
            .read()
            .is_ok_and(|active| !active.toggle_mode.modifier_held(modifiers))
        {
            self.toggle_latched = false;
        }
        let _ = self.sender.send(UiMessage::Modifiers(modifiers));
    }

    pub(crate) fn press_key(&mut self, key: &'static str, held: u8) {
        if self.chord_complete && held != 0 {
            self.send_held_modifiers(held);
        }
        self.held = held;
        let (toggle, quit) = self.keymap.read().map_or((false, false), |active| {
            (
                active.toggle_mode.matches(key, modifiers(held)),
                active.quit.matches(key, modifiers(held)),
            )
        });
        self.send_key(key);
        self.chord_complete = held != 0;
        if quit {
            let _ = self.sender.send(UiMessage::Quit);
            return;
        }
        if toggle && !self.toggle_latched {
            self.toggle_latched = true;
            if let Err(error) = theme::toggle_mode() {
                eprintln!("Could not toggle KeyRC mode: {error}");
            }
        }
    }

    pub(crate) fn release_key(&mut self, key: &str) {
        if self.keymap.read().is_ok_and(|active| {
            !active.toggle_mode.has_modifiers() && active.toggle_mode.key.eq_ignore_ascii_case(key)
        }) {
            self.toggle_latched = false;
        }
    }

    fn send_held_modifiers(&self, held: u8) {
        for (mask, key) in [
            (SHIFT_LEFT | SHIFT_RIGHT, "Shift"),
            (CTRL_LEFT | CTRL_RIGHT, "Ctrl"),
            (ALT_LEFT | ALT_RIGHT, "Alt"),
            (SUPER_LEFT | SUPER_RIGHT, "Super"),
        ] {
            if held & mask != 0 {
                self.send_key(key);
            }
        }
    }

    fn send_key(&self, key: &'static str) {
        let _ = self.sender.send(UiMessage::Key(KeyMessage { key }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::glib::ControlFlow;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn recorded_events(run: impl FnOnce(&mut Processor)) -> Vec<&'static str> {
        let context = glib::MainContext::new();
        let _guard = context.acquire().expect("test main context is available");
        #[allow(deprecated)]
        let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
        let keys = Rc::new(RefCell::new(Vec::new()));
        let received = Rc::clone(&keys);
        receiver.attach(Some(&context), move |message| {
            match message {
                UiMessage::Key(message) => received.borrow_mut().push(message.key),
                UiMessage::Quit => received.borrow_mut().push("Quit"),
                UiMessage::Modifiers(_) | UiMessage::Theme(_) => {}
            }
            ControlFlow::Continue
        });

        let mut processor = Processor::new(sender, Arc::new(RwLock::new(Keymap::default())));
        run(&mut processor);
        while context.pending() {
            context.iteration(false);
        }
        let recorded = keys.borrow().clone();
        recorded
    }

    #[test]
    fn initial_combination_does_not_duplicate_modifiers() {
        let keys = recorded_events(|processor| {
            processor.press_modifier("Ctrl", CTRL_LEFT);
            processor.press_modifier("Alt", CTRL_LEFT | ALT_LEFT);
            processor.press_key("X", CTRL_LEFT | ALT_LEFT);
        });
        assert_eq!(keys, ["Ctrl", "Alt", "X"]);
    }

    #[test]
    fn held_modifiers_are_repeated_for_the_next_combination() {
        let keys = recorded_events(|processor| {
            processor.press_modifier("Super", SUPER_LEFT);
            processor.press_key("Tab", SUPER_LEFT);
            processor.release_key("Tab");
            processor.press_key("1", SUPER_LEFT);
        });
        assert_eq!(keys, ["Super", "Tab", "Super", "1"]);
    }

    #[test]
    fn quit_shortcut_sends_quit_message() {
        let events = recorded_events(|processor| {
            processor.press_modifier("Ctrl", CTRL_LEFT);
            processor.press_modifier("Alt", CTRL_LEFT | ALT_LEFT);
            processor.press_key("Q", CTRL_LEFT | ALT_LEFT);
        });
        assert_eq!(events, ["Ctrl", "Alt", "Q", "Quit"]);
    }
}
