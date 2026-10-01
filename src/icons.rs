use crate::model::Color;
use gtk::cairo::Context;
use gtk::gdk::prelude::GdkContextExt;
use gtk::gdk_pixbuf::{prelude::PixbufLoaderExt, Pixbuf, PixbufLoader};

const BACKSPACE: &str = r#"<g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"><path d="M9 6h10a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H9l-6-6z"/><path d="m12 9 6 6m0-6-6 6"/></g>"#;
const DELETE: &str = r#"<g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"><path d="M15 6H5a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h10l6-6z"/><path d="m6 9 6 6m0-6-6 6"/></g>"#;
const ENTER: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M20 5v7a4 4 0 0 1-4 4H5m4-4-4 4 4 4"/>"#;
const TAB: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M20 5v14M4 12h12m-4-4 4 4-4 4"/>"#;
const SPACE: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 8v8h14V8"/>"#;
const ESCAPE: &str = r#"<g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"><path d="M9.5 4.4A8 8 0 1 1 4.4 9.5"/><path d="M4 4h6M4 4v6m0-6 6 6"/></g>"#;
const LEFT: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 12h4m3 0h3m-6-5-5 5 5 5"/>"#;
const RIGHT: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M19 12h-4m-3 0H9m6-5 5 5-5 5"/>"#;
const UP: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 19v-4m0-3V9M7 9l5-5 5 5"/>"#;
const DOWN: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 5v4m0 3v3m-5 0 5 5 5-5"/>"#;
const HOME: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M19 19 5 5m0 0v8m0-8h8"/>"#;
const END: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 5l14 14m0 0v-8m0 8h-8"/>"#;
const PAGE_UP: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 20V5m-5 5 5-5 5 5M8 14h8m-8 4h8"/>"#;
const PAGE_DOWN: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 4v15m-5-5 5 5 5-5M8 10h8M8 6h8"/>"#;
const CAPS_LOCK: &str = r#"<g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"><path d="m4 12 8-8 8 8h-5v5H9v-5z"/><path d="M8 21h8"/></g>"#;

const SHIFT: &str = r#"<path fill="currentColor" transform="scale(1.5)" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5 8 2.731 1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/>"#;
const CTRL: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="m5 15 7-7 7 7"/>"#;
const ALT: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 7h5l7 10h4M14 7h6"/>"#;
const SUPER: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 6v12c0 2.7 3.3 4 5.2 2.2S20.7 15 18 15H6c-2.7 0-4 3.3-2.2 5.2S9 20.7 9 18V6c0-2.7-3.3-4-5.2-2.2S3.3 9 6 9h12c2.7 0 4-3.3 2.2-5.2S15 3.3 15 6"/>"#;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SpecialIcon {
    Backspace,
    Delete,
    Enter,
    Tab,
    Space,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    CapsLock,
}

impl SpecialIcon {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "Backspace" => Some(Self::Backspace),
            "Delete" => Some(Self::Delete),
            "Enter" => Some(Self::Enter),
            "Tab" => Some(Self::Tab),
            "Space" => Some(Self::Space),
            "Escape" => Some(Self::Escape),
            "Left" => Some(Self::Left),
            "Right" => Some(Self::Right),
            "Up" => Some(Self::Up),
            "Down" => Some(Self::Down),
            "Home" => Some(Self::Home),
            "End" => Some(Self::End),
            "PageUp" => Some(Self::PageUp),
            "PageDown" => Some(Self::PageDown),
            "CapsLock" => Some(Self::CapsLock),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ModifierIcon {
    Shift,
    Ctrl,
    Alt,
    Super,
}

struct IconPair {
    regular: Pixbuf,
    large: Pixbuf,
}

impl IconPair {
    fn new(regular: (i32, i32), large: (i32, i32), body: &str) -> Self {
        Self {
            regular: load_svg(regular.0, regular.1, body),
            large: load_svg(large.0, large.1, body),
        }
    }

    fn get(&self, large: bool) -> &Pixbuf {
        if large {
            &self.large
        } else {
            &self.regular
        }
    }
}

pub(crate) struct Icons {
    special: [IconPair; 15],
    modifiers: [IconPair; 4],
}

impl Icons {
    pub(crate) fn new() -> Self {
        let special = |body| IconPair::new((36, 36), (44, 44), body);
        Self {
            special: [
                special(BACKSPACE),
                special(DELETE),
                special(ENTER),
                special(TAB),
                special(SPACE),
                special(ESCAPE),
                special(LEFT),
                special(RIGHT),
                special(UP),
                special(DOWN),
                special(HOME),
                special(END),
                special(PAGE_UP),
                special(PAGE_DOWN),
                special(CAPS_LOCK),
            ],
            modifiers: [
                IconPair::new((26, 22), (36, 36), SHIFT),
                IconPair::new((29, 29), (36, 36), CTRL),
                IconPair::new((26, 22), (36, 36), ALT),
                IconPair::new((26, 22), (36, 36), SUPER),
            ],
        }
    }

    pub(crate) fn special(&self, icon: SpecialIcon, large: bool) -> &Pixbuf {
        self.special[icon as usize].get(large)
    }

    pub(crate) fn modifier(&self, icon: ModifierIcon, large: bool) -> &Pixbuf {
        self.modifiers[icon as usize].get(large)
    }
}

fn load_svg(width: i32, height: i32, body: &str) -> Pixbuf {
    let loader = PixbufLoader::with_type("svg").expect("SVG loader is unavailable");
    loader.set_size(width, height);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 24 24" color="white">{body}</svg>"#
    );
    loader
        .write(svg.as_bytes())
        .expect("Could not render KeyRC icon");
    loader.close().expect("Could not finish KeyRC icon");
    loader.pixbuf().expect("Rendered KeyRC icon is empty")
}

pub(crate) fn draw(context: &Context, icon: &Pixbuf, x: f64, y: f64, color: Color) {
    context.push_group();
    context.set_source_pixbuf(icon, x, y);
    let _ = context.paint();
    if let Ok(mask) = context.pop_group() {
        color.set(context);
        let _ = context.mask(mask);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keycastr_special_keys() {
        assert_eq!(
            SpecialIcon::from_key("Backspace"),
            Some(SpecialIcon::Backspace)
        );
        assert_eq!(SpecialIcon::from_key("Delete"), Some(SpecialIcon::Delete));
        assert_eq!(SpecialIcon::from_key("Escape"), Some(SpecialIcon::Escape));
        assert_eq!(SpecialIcon::from_key("Left"), Some(SpecialIcon::Left));
        assert_eq!(
            SpecialIcon::from_key("PageDown"),
            Some(SpecialIcon::PageDown)
        );
        assert_eq!(SpecialIcon::from_key("A"), None);
    }
}
