use super::{
    Processor, ALT_LEFT, ALT_RIGHT, CTRL_LEFT, CTRL_RIGHT, SHIFT_LEFT, SHIFT_RIGHT, SUPER_LEFT,
    SUPER_RIGHT,
};
use crate::model::{Shortcut, UiMessage};
use evdev::{Device, EventSummary, KeyCode};
use gtk::glib;
use nix::poll::{poll, PollFd, PollFlags};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;

struct Keyboard {
    path: PathBuf,
    device: Device,
    held: u8,
}

fn key_name(key: KeyCode) -> &'static str {
    match key {
        KeyCode::KEY_BACKSPACE => "Backspace",
        KeyCode::KEY_CAPSLOCK => "CapsLock",
        KeyCode::KEY_DELETE | KeyCode::KEY_KPDOT => "Delete",
        KeyCode::KEY_DOWN => "Down",
        KeyCode::KEY_END => "End",
        KeyCode::KEY_ESC => "Escape",
        KeyCode::KEY_F1 => "F1",
        KeyCode::KEY_F2 => "F2",
        KeyCode::KEY_F3 => "F3",
        KeyCode::KEY_F4 => "F4",
        KeyCode::KEY_F5 => "F5",
        KeyCode::KEY_F6 => "F6",
        KeyCode::KEY_F7 => "F7",
        KeyCode::KEY_F8 => "F8",
        KeyCode::KEY_F9 => "F9",
        KeyCode::KEY_F10 => "F10",
        KeyCode::KEY_F11 => "F11",
        KeyCode::KEY_F12 => "F12",
        KeyCode::KEY_HOME => "Home",
        KeyCode::KEY_INSERT => "Insert",
        KeyCode::KEY_LEFT => "Left",
        KeyCode::KEY_PAGEDOWN => "PageDown",
        KeyCode::KEY_PAGEUP => "PageUp",
        KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER => "Enter",
        KeyCode::KEY_RIGHT => "Right",
        KeyCode::KEY_SPACE => "Space",
        KeyCode::KEY_TAB => "Tab",
        KeyCode::KEY_UP => "Up",
        KeyCode::KEY_SYSRQ => "PrintScreen",
        KeyCode::KEY_SCROLLLOCK => "ScrollLock",
        KeyCode::KEY_PAUSE => "Pause",
        KeyCode::KEY_NUMLOCK => "NumLock",
        KeyCode::KEY_GRAVE => "`",
        KeyCode::KEY_1 | KeyCode::KEY_KP1 => "1",
        KeyCode::KEY_2 | KeyCode::KEY_KP2 => "2",
        KeyCode::KEY_3 | KeyCode::KEY_KP3 => "3",
        KeyCode::KEY_4 | KeyCode::KEY_KP4 => "4",
        KeyCode::KEY_5 | KeyCode::KEY_KP5 => "5",
        KeyCode::KEY_6 | KeyCode::KEY_KP6 => "6",
        KeyCode::KEY_7 | KeyCode::KEY_KP7 => "7",
        KeyCode::KEY_8 | KeyCode::KEY_KP8 => "8",
        KeyCode::KEY_9 | KeyCode::KEY_KP9 => "9",
        KeyCode::KEY_0 | KeyCode::KEY_KP0 => "0",
        KeyCode::KEY_MINUS | KeyCode::KEY_KPMINUS => "-",
        KeyCode::KEY_EQUAL | KeyCode::KEY_KPEQUAL => "=",
        KeyCode::KEY_KPPLUS => "+",
        KeyCode::KEY_KPASTERISK => "*",
        KeyCode::KEY_KPSLASH | KeyCode::KEY_SLASH => "/",
        KeyCode::KEY_Q => "Q",
        KeyCode::KEY_W => "W",
        KeyCode::KEY_E => "E",
        KeyCode::KEY_R => "R",
        KeyCode::KEY_T => "T",
        KeyCode::KEY_Y => "Y",
        KeyCode::KEY_U => "U",
        KeyCode::KEY_I => "I",
        KeyCode::KEY_O => "O",
        KeyCode::KEY_P => "P",
        KeyCode::KEY_LEFTBRACE => "[",
        KeyCode::KEY_RIGHTBRACE => "]",
        KeyCode::KEY_A => "A",
        KeyCode::KEY_S => "S",
        KeyCode::KEY_D => "D",
        KeyCode::KEY_F => "F",
        KeyCode::KEY_G => "G",
        KeyCode::KEY_H => "H",
        KeyCode::KEY_J => "J",
        KeyCode::KEY_K => "K",
        KeyCode::KEY_L => "L",
        KeyCode::KEY_SEMICOLON => ";",
        KeyCode::KEY_APOSTROPHE => "'",
        KeyCode::KEY_BACKSLASH | KeyCode::KEY_102ND => "\\",
        KeyCode::KEY_Z => "Z",
        KeyCode::KEY_X => "X",
        KeyCode::KEY_C => "C",
        KeyCode::KEY_V => "V",
        KeyCode::KEY_B => "B",
        KeyCode::KEY_N => "N",
        KeyCode::KEY_M => "M",
        KeyCode::KEY_COMMA => ",",
        KeyCode::KEY_DOT => ".",
        _ => "",
    }
}

fn modifier_key(key: KeyCode) -> Option<(u8, &'static str)> {
    match key {
        KeyCode::KEY_LEFTSHIFT => Some((SHIFT_LEFT, "Shift")),
        KeyCode::KEY_RIGHTSHIFT => Some((SHIFT_RIGHT, "Shift")),
        KeyCode::KEY_LEFTCTRL => Some((CTRL_LEFT, "Ctrl")),
        KeyCode::KEY_RIGHTCTRL => Some((CTRL_RIGHT, "Ctrl")),
        KeyCode::KEY_LEFTALT => Some((ALT_LEFT, "Alt")),
        KeyCode::KEY_RIGHTALT => Some((ALT_RIGHT, "Alt")),
        KeyCode::KEY_LEFTMETA => Some((SUPER_LEFT, "Super")),
        KeyCode::KEY_RIGHTMETA => Some((SUPER_RIGHT, "Super")),
        _ => None,
    }
}

fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().is_some_and(|keys| {
        keys.contains(KeyCode::KEY_A)
            && keys.contains(KeyCode::KEY_Z)
            && keys.contains(KeyCode::KEY_ENTER)
            && keys.contains(KeyCode::KEY_SPACE)
    })
}

fn event_paths() -> impl Iterator<Item = PathBuf> {
    fs::read_dir("/dev/input")
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("event"))
        })
}

fn add_keyboards(keyboards: &mut Vec<Keyboard>) {
    let existing: HashSet<_> = keyboards.iter().map(|item| item.path.as_path()).collect();
    let additions: Vec<_> = event_paths()
        .filter(|path| !existing.contains(path.as_path()))
        .filter_map(|path| Device::open(&path).ok().map(|device| (path, device)))
        .filter(|(_, device)| is_keyboard(device))
        .filter_map(|(path, device)| {
            device.set_nonblocking(true).ok().map(|_| Keyboard {
                path,
                device,
                held: 0,
            })
        })
        .collect();
    keyboards.extend(additions);
}

fn combined_modifiers(keyboards: &[Keyboard]) -> u8 {
    keyboards.iter().fold(0, |held, device| held | device.held)
}

fn handle_key(
    keyboards: &mut [Keyboard],
    index: usize,
    key: KeyCode,
    value: i32,
    processor: &mut Processor,
) {
    if let Some((bit, name)) = modifier_key(key) {
        if value == 1 && keyboards[index].held & bit == 0 {
            keyboards[index].held |= bit;
            processor.press_modifier(name, combined_modifiers(keyboards));
        } else if value == 0 && keyboards[index].held & bit != 0 {
            keyboards[index].held &= !bit;
            processor.release_modifier(combined_modifiers(keyboards));
        }
        return;
    }
    let name = key_name(key);
    if name.is_empty() {
        return;
    }
    if matches!(value, 1 | 2) {
        processor.press_key(name, combined_modifiers(keyboards));
    } else if value == 0 {
        processor.release_key(name);
    }
}

fn listen(sender: glib::Sender<UiMessage>, shortcut: Arc<RwLock<Shortcut>>) {
    let mut keyboards = Vec::new();
    let mut processor = Processor::new(sender, shortcut);
    let mut warned_no_keyboard = false;
    loop {
        add_keyboards(&mut keyboards);
        if keyboards.is_empty() {
            if !warned_no_keyboard {
                eprintln!("No readable keyboard found under /dev/input; check input-group access");
                warned_no_keyboard = true;
            }
            thread::sleep(std::time::Duration::from_secs(2));
            continue;
        }
        warned_no_keyboard = false;

        let ready = {
            let mut descriptors: Vec<_> = keyboards
                .iter()
                .map(|keyboard| PollFd::new(keyboard.device.as_fd(), PollFlags::POLLIN))
                .collect();
            if poll(&mut descriptors, 2000_u16).is_err() {
                continue;
            }
            descriptors
                .iter()
                .map(|descriptor| {
                    descriptor.revents().is_some_and(|events| {
                        events
                            .intersects(PollFlags::POLLIN | PollFlags::POLLERR | PollFlags::POLLHUP)
                    })
                })
                .collect::<Vec<_>>()
        };

        let mut remove = Vec::new();
        for (index, ready) in ready.into_iter().enumerate() {
            if !ready {
                continue;
            }
            let events = match keyboards[index].device.fetch_events() {
                Ok(events) => events.collect::<Vec<_>>(),
                Err(error) if error.kind() == ErrorKind::WouldBlock => continue,
                Err(_) => {
                    remove.push(index);
                    continue;
                }
            };
            for event in events {
                if let EventSummary::Key(_, key, value) = event.destructure() {
                    handle_key(&mut keyboards, index, key, value, &mut processor);
                }
            }
        }
        let removed_modifier = remove.iter().any(|&index| keyboards[index].held != 0);
        for index in remove.into_iter().rev() {
            keyboards.remove(index);
        }
        if removed_modifier {
            processor.release_modifier(combined_modifiers(&keyboards));
        }
    }
}

pub(super) fn start(sender: glib::Sender<UiMessage>, shortcut: Arc<RwLock<Shortcut>>) {
    thread::Builder::new()
        .name("keyrc-evdev-input".into())
        .stack_size(512 * 1024)
        .spawn(move || listen(sender, shortcut))
        .expect("Could not start evdev input listener");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_evdev_keys() {
        assert_eq!(key_name(KeyCode::KEY_A), "A");
        assert_eq!(key_name(KeyCode::KEY_KPENTER), "Enter");
        assert_eq!(
            modifier_key(KeyCode::KEY_LEFTMETA),
            Some((SUPER_LEFT, "Super"))
        );
    }
}
