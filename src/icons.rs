use crate::model::Color;
use gtk::cairo::Context;
use gtk::gdk::prelude::GdkContextExt;
use gtk::gdk_pixbuf::{prelude::PixbufLoaderExt, Pixbuf, PixbufLoader};

const BACKSPACE: &str = r#"<path fill="currentColor" d="m11.4 16 2.6-2.6 2.6 2.6 1.4-1.4-2.6-2.6L18 9.4 16.6 8 14 10.6 11.4 8 10 9.4l2.6 2.6-2.6 2.6zM9 20q-.475 0-.9-.213t-.7-.587L2 12l5.4-7.2q.275-.375.7-.587T9 4h11q.825 0 1.413.587T22 6v12q0 .825-.587 1.413T20 20zm-4.5-8L9 18h11V6H9zm10 0"/>"#;
const ENTER: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="square" stroke-width="2" d="M5.75 16H16a3 3 0 0 0 3-3V5M8 12.5 4.5 16 8 19.5"/>"#;
const TAB: &str = r#"<path fill="currentColor" d="m10.78 8.53-3.75 3.75a.749.749 0 1 1-1.06-1.06l2.469-2.47H1.75a.75.75 0 0 1 0-1.5h6.689L5.97 4.78a.749.749 0 1 1 1.06-1.06l3.75 3.75a.75.75 0 0 1 0 1.06M13 12.25v-8.5a.75.75 0 0 1 1.5 0v8.5a.75.75 0 0 1-1.5 0"/>"#;
const SPACE: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>"#;
const CAPS_LOCK: &str = r#"<path fill="currentColor" d="M20.781 37.621h14.461c3.281 0 5.016-1.922 5.016-5.016v-4.148h8.882c1.946 0 3.493-1.148 3.493-2.953 0-1.102-.563-1.969-1.617-2.883L30.906 4.88c-.96-.844-1.851-1.406-2.906-1.406-1.031 0-1.922.562-2.883 1.406L4.984 22.645c-1.101.96-1.617 1.757-1.617 2.859 0 1.805 1.547 2.953 3.516 2.953h8.86v4.148c0 3.094 1.757 5.016 5.038 5.016m.375-3.539c-.89 0-1.5-.586-1.5-1.477v-6.89c0-.563-.21-.797-.773-.797H8.664c-.164 0-.234-.07-.234-.187a.33.33 0 0 1 .14-.282L27.508 7.996c.21-.187.328-.258.492-.258s.305.07.492.258L47.453 24.45a.33.33 0 0 1 .14.281c0 .118-.093.188-.257.188H37.14c-.563 0-.774.234-.774.797v6.89c0 .868-.656 1.477-1.5 1.477Zm-1.383 18.445h16.29c2.695 0 4.242-1.5 4.242-4.218v-3.375c0-2.72-1.547-4.266-4.243-4.266H19.773c-2.718 0-4.265 1.57-4.265 4.266v3.375c0 2.695 1.547 4.218 4.265 4.218m.54-3.304c-.82 0-1.266-.422-1.266-1.242v-2.72c0-.82.445-1.288 1.265-1.288h15.211c.797 0 1.242.468 1.242 1.289v2.718c0 .82-.445 1.243-1.242 1.243Z"/>"#;
const SHIFT: &str = r#"<path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5 8 2.731 1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/>"#;
const CTRL: &str = r#"<path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/>"#;
const ALT: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/>"#;
const SUPER: &str = r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13 1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14 1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/>"#;

#[derive(Clone, Copy)]
pub(crate) enum SpecialIcon {
    Backspace,
    Enter,
    Tab,
    Space,
    CapsLock,
}

impl SpecialIcon {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "Backspace" => Some(Self::Backspace),
            "Enter" => Some(Self::Enter),
            "Tab" => Some(Self::Tab),
            "Space" => Some(Self::Space),
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
    fn new(regular: (i32, i32), large: (i32, i32), view_box: &str, body: &str) -> Self {
        Self {
            regular: load_svg(regular.0, regular.1, view_box, body),
            large: load_svg(large.0, large.1, view_box, body),
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
    special: [IconPair; 5],
    modifiers: [IconPair; 4],
}

impl Icons {
    pub(crate) fn new() -> Self {
        Self {
            special: [
                IconPair::new((36, 36), (44, 44), "0 0 24 24", BACKSPACE),
                IconPair::new((36, 36), (44, 44), "0 0 24 24", ENTER),
                IconPair::new((36, 36), (44, 44), "0 0 16 16", TAB),
                IconPair::new((36, 36), (44, 44), "0 0 24 24", SPACE),
                IconPair::new((36, 36), (44, 44), "0 0 56 56", CAPS_LOCK),
            ],
            modifiers: [
                IconPair::new((26, 22), (36, 36), "0 0 16 16", SHIFT),
                IconPair::new((29, 29), (36, 36), "0 0 16 16", CTRL),
                IconPair::new((26, 22), (36, 36), "0 0 24 24", ALT),
                IconPair::new((26, 22), (36, 36), "0 0 24 24", SUPER),
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

fn load_svg(width: i32, height: i32, view_box: &str, body: &str) -> Pixbuf {
    let loader = PixbufLoader::with_type("svg").expect("SVG loader is unavailable");
    loader.set_size(width, height);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{view_box}" color="white">{body}</svg>"#
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
