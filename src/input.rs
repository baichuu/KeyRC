use crate::model::{KeyMessage, Modifiers, UiMessage};
use gtk::glib;
use rdev::{listen, EventType, Key};
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

pub(crate) fn start_listener(sender: glib::Sender<UiMessage>) {
    thread::Builder::new()
        .name("keyrc-input".into())
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut held = 0_u8;
            listen(move |event| match event.event_type {
                EventType::KeyPress(key) => {
                    if let Some((bit, key)) = modifier_key(key) {
                        if held & bit == 0 {
                            let chord = modifiers(held);
                            held |= bit;
                            let _ = sender.send(UiMessage::Modifiers(modifiers(held)));
                            let _ = sender.send(UiMessage::Key(KeyMessage {
                                key,
                                modifiers: chord,
                            }));
                        }
                    } else {
                        let key = key_name(key);
                        if !key.is_empty() {
                            let _ = sender.send(UiMessage::Key(KeyMessage {
                                key,
                                modifiers: modifiers(held),
                            }));
                        }
                    }
                }
                EventType::KeyRelease(key) => {
                    if let Some((bit, _)) = modifier_key(key) {
                        held &= !bit;
                        let _ = sender.send(UiMessage::Modifiers(modifiers(held)));
                    }
                }
                _ => {}
            })
            .expect("Could not listen to events");
        })
        .expect("Could not start input listener");
}
