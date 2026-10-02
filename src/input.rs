use crate::model::{KeyMessage, Keymap, Modifiers, UiMessage};
use crate::theme;
use gtk::glib;
use rdev::{listen, EventType, Key};
use std::sync::{Arc, RwLock};
use std::thread;

const SHIFT_LEFT: u8 = 1 << 0;
const SHIFT_RIGHT: u8 = 1 << 1;
const CTRL_LEFT: u8 = 1 << 2;
const CTRL_RIGHT: u8 = 1 << 3;
const ALT_LEFT: u8 = 1 << 4;
const ALT_RIGHT: u8 = 1 << 5;
const SUPER_LEFT: u8 = 1 << 6;
const SUPER_RIGHT: u8 = 1 << 7;

fn key_name(key: Key) -> &'static str {
    match key {
        Key::Alt => "Alt",
        Key::AltGr => "AltGr",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::ControlLeft | Key::ControlRight => "Ctrl",
        Key::Delete | Key::KpDelete => "Delete",
        Key::DownArrow => "Down",
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
        Key::LeftArrow => "Left",
        Key::MetaLeft | Key::MetaRight => "Super",
        Key::PageDown => "PageDown",
        Key::PageUp => "PageUp",
        Key::Return | Key::KpReturn => "Enter",
        Key::RightArrow => "Right",
        Key::ShiftLeft | Key::ShiftRight => "Shift",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "Up",
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

fn modifiers(held: u8) -> Modifiers {
    Modifiers {
        shift: held & (SHIFT_LEFT | SHIFT_RIGHT) != 0,
        ctrl: held & (CTRL_LEFT | CTRL_RIGHT) != 0,
        alt: held & (ALT_LEFT | ALT_RIGHT) != 0,
        super_key: held & (SUPER_LEFT | SUPER_RIGHT) != 0,
    }
}

fn modifier_key(key: Key) -> Option<(u8, &'static str)> {
    match key {
        Key::ShiftLeft => Some((SHIFT_LEFT, "Shift")),
        Key::ShiftRight => Some((SHIFT_RIGHT, "Shift")),
        Key::ControlLeft => Some((CTRL_LEFT, "Ctrl")),
        Key::ControlRight => Some((CTRL_RIGHT, "Ctrl")),
        Key::Alt => Some((ALT_LEFT, "Alt")),
        Key::AltGr => Some((ALT_RIGHT, "Alt")),
        Key::MetaLeft => Some((SUPER_LEFT, "Super")),
        Key::MetaRight => Some((SUPER_RIGHT, "Super")),
        _ => None,
    }
}

struct Processor {
    sender: glib::Sender<UiMessage>,
    keymap: Arc<RwLock<Keymap>>,
    toggle_latched: bool,
    held: u8,
    chord_complete: bool,
}

impl Processor {
    fn new(sender: glib::Sender<UiMessage>, keymap: Arc<RwLock<Keymap>>) -> Self {
        Self {
            sender,
            keymap,
            toggle_latched: false,
            held: 0,
            chord_complete: false,
        }
    }

    fn press_modifier(&mut self, key: &'static str, held: u8) {
        if self.chord_complete {
            self.send_held_modifiers(self.held);
            self.chord_complete = false;
        }
        self.held = held;
        let _ = self.sender.send(UiMessage::Modifiers(modifiers(held)));
        self.send_key(key);
    }

    fn release_modifier(&mut self, held: u8) {
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

    fn press_key(&mut self, key: &'static str, held: u8) {
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

    fn release_key(&mut self, key: &str) {
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

pub(crate) fn start_listener(sender: glib::Sender<UiMessage>, keymap: Arc<RwLock<Keymap>>) {
    thread::Builder::new()
        .name("keyrc-input".into())
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut held = 0_u8;
            let mut processor = Processor::new(sender, keymap);
            listen(move |event| match event.event_type {
                EventType::KeyPress(key) => {
                    if let Some((bit, key)) = modifier_key(key) {
                        if held & bit == 0 {
                            held |= bit;
                            processor.press_modifier(key, held);
                        }
                    } else {
                        let key = key_name(key);
                        if !key.is_empty() {
                            processor.press_key(key, held);
                        }
                    }
                }
                EventType::KeyRelease(key) => {
                    if let Some((bit, _)) = modifier_key(key) {
                        held &= !bit;
                        processor.release_modifier(held);
                    } else {
                        processor.release_key(key_name(key));
                    }
                }
                _ => {}
            })
            .expect("Could not listen to events");
        })
        .expect("Could not start input listener");
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
