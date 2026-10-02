use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Backend {
    X11,
    Wayland,
}

impl Backend {
    pub(crate) fn detect() -> Self {
        match std::env::var("KEYRC_BACKEND").as_deref() {
            Ok("wayland") | Ok("evdev") => return Self::Wayland,
            Ok("x11") => return Self::X11,
            _ => {}
        }
        if std::env::var_os("WAYLAND_DISPLAY").is_some()
            && std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("x11")
        {
            Self::Wayland
        } else {
            Self::X11
        }
    }

    pub(crate) fn position_file(self) -> &'static str {
        match self {
            Self::X11 => "position",
            Self::Wayland => "position-wayland",
        }
    }

    pub(crate) fn configure_window(
        self,
        window: &gtk::Window,
        position: Option<(i32, i32)>,
    ) -> ((i32, i32), bool) {
        match self {
            Self::X11 => {
                if let Some((x, y)) = position {
                    window.move_(x, y);
                }
                (position.unwrap_or_default(), false)
            }
            Self::Wayland => {
                let (x, y) = position.unwrap_or((24, 24));
                let layer_shell = gtk_layer_shell::is_supported();
                if layer_shell {
                    window.init_layer_shell();
                    window.set_namespace("keyrc");
                    window.set_layer(Layer::Overlay);
                    window.set_keyboard_mode(KeyboardMode::None);
                    window.set_exclusive_zone(-1);
                    window.set_anchor(Edge::Left, true);
                    window.set_anchor(Edge::Top, true);
                    window.set_layer_shell_margin(Edge::Left, x.max(0));
                    window.set_layer_shell_margin(Edge::Top, y.max(0));
                } else {
                    eprintln!(
                        "Wayland compositor does not support layer-shell; using a normal window"
                    );
                }
                ((x.max(0), y.max(0)), layer_shell)
            }
        }
    }

    pub(crate) fn set_wayland_position(self, window: &gtk::Window, x: i32, y: i32) {
        if self == Self::Wayland && window.is_layer_window() {
            window.set_layer_shell_margin(Edge::Left, x.max(0));
            window.set_layer_shell_margin(Edge::Top, y.max(0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_separate_position_cache() {
        assert_eq!(Backend::X11.position_file(), "position");
        assert_eq!(Backend::Wayland.position_file(), "position-wayland");
    }
}
