use crate::icons::Icons;
use crate::input;
use crate::model::{AppState, UiMessage, WIDTH};
#[cfg(debug_assertions)]
use crate::model::{DisplayMode, KeyMessage, Modifiers};
use crate::{position, render, theme};
use gtk::gdk;
use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
#[cfg(debug_assertions)]
use std::time::Instant;

fn tray_icon_directory() -> Option<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/share"))
        });
    let mut directories = Vec::new();
    if let Some(data_home) = data_home {
        directories.push(data_home.join("icons/hicolor/1024x1024/apps"));
    }
    directories.extend([
        PathBuf::from("/usr/local/share/icons/hicolor/1024x1024/apps"),
        PathBuf::from("/usr/share/icons/hicolor/1024x1024/apps"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
    ]);
    directories
        .into_iter()
        .find(|directory| directory.join("keyrc.png").is_file())
}

fn setup_tray() -> AppIndicator {
    let mut menu = gtk::Menu::new();
    let toggle = gtk::MenuItem::with_label("Toggle mode");
    toggle.connect_activate(|_| {
        if let Err(error) = theme::toggle_mode() {
            eprintln!("Could not toggle KeyRC mode: {error}");
        }
    });
    menu.append(&toggle);
    menu.append(&gtk::SeparatorMenuItem::new());
    let quit = gtk::MenuItem::with_label("Quit");
    quit.connect_activate(|_| gtk::main_quit());
    menu.append(&quit);
    menu.show_all();

    let mut indicator = if let Some(directory) = tray_icon_directory() {
        AppIndicator::with_path("keyrc", "keyrc", &directory.to_string_lossy())
    } else {
        AppIndicator::new("keyrc", "input-keyboard")
    };
    indicator.set_title("KeyRC");
    indicator.set_menu(&mut menu);
    indicator.set_status(AppIndicatorStatus::Active);
    indicator
}

fn configure_window(window: &gtk::Window, height: i32) {
    window.set_title("keyrc");
    window.set_decorated(false);
    window.set_resizable(false);
    window.set_keep_above(true);
    window.stick();
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_accept_focus(false);
    window.set_focus_on_map(false);
    window.set_deletable(false);
    window.set_app_paintable(true);
    window.set_default_size(WIDTH, height);
    window.set_size_request(WIDTH, height);
    if let Some(screen) = WidgetExt::screen(window) {
        if let Some(visual) = screen.rgba_visual() {
            window.set_visual(Some(&visual));
        }
    }
}

#[cfg(debug_assertions)]
fn apply_preview_mode(mut theme: crate::model::Theme) -> crate::model::Theme {
    match std::env::var("KEYRC_PREVIEW_MODE").as_deref() {
        Ok("keys_only") => theme.mode = DisplayMode::KeysOnly,
        Ok("full") => theme.mode = DisplayMode::Full,
        _ => {}
    }
    theme
}

#[cfg(debug_assertions)]
fn apply_preview_keys(mut state: AppState) -> AppState {
    if let Ok(keys) = std::env::var("KEYRC_PREVIEW_KEYS") {
        for specification in keys.split(',') {
            let mut parts = specification.split('+').collect::<Vec<_>>();
            let raw_key = parts.pop().unwrap_or_default();
            let key = match raw_key {
                "Right" => "Right",
                "Left" => "Left",
                "Up" => "Up",
                "Down" => "Down",
                "Enter" => "Enter",
                other => Box::leak(other.to_owned().into_boxed_str()),
            };
            let modifiers = Modifiers {
                shift: parts.contains(&"Shift"),
                ctrl: parts.contains(&"Ctrl"),
                alt: parts.contains(&"Alt"),
                super_key: parts.contains(&"Super"),
            };
            state.push_key(KeyMessage { key, modifiers });
        }
        for key in &mut state.history {
            key.created = Instant::now() - Duration::from_secs(1);
        }
    }
    state
}

pub(crate) fn run() {
    gtk::init().expect("Could not initialize GTK");

    let settings = theme::read(theme::path().as_deref());
    let initial_theme = settings.theme;
    let shortcut = Arc::new(RwLock::new(settings.toggle_mode));
    #[cfg(debug_assertions)]
    let initial_theme = apply_preview_mode(initial_theme);

    let icons = Rc::new(Icons::new());
    let initial_state = AppState::new(initial_theme.clone());
    #[cfg(debug_assertions)]
    let initial_state = apply_preview_keys(initial_state);
    let state = Rc::new(RefCell::new(initial_state));

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    configure_window(&window, initial_theme.mode.height());
    let position_path = position::path();
    if let Some((x, y)) = position::read(position_path.as_deref()) {
        window.move_(x, y);
    }

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_size_request(WIDTH, initial_theme.mode.height());
    drawing_area.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    window.add(&drawing_area);

    let draw_state = Rc::clone(&state);
    let draw_icons = Rc::clone(&icons);
    drawing_area.connect_draw(move |_, context| {
        render::draw(context, &draw_icons, &draw_state.borrow());
        glib::Propagation::Stop
    });

    let drag_window = window.clone();
    drawing_area.connect_button_press_event(move |_, event| {
        if event.button() == 1 {
            let (x, y) = event.root();
            drag_window.begin_move_drag(1, x as i32, y as i32, event.time());
        }
        glib::Propagation::Stop
    });
    window.connect_delete_event(|_, _| glib::Propagation::Stop);

    let position_ready = Rc::new(Cell::new(false));
    if let Some(path) = position_path {
        let write_position = position::start_writer(path);
        let position_ready = Rc::clone(&position_ready);
        window.connect_configure_event(move |_, event| {
            if position_ready.get() {
                write_position(event.position());
            }
            false
        });
    }

    #[allow(deprecated)]
    let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
    #[cfg(debug_assertions)]
    let previewing = std::env::var_os("KEYRC_PREVIEW_KEYS").is_some();
    #[cfg(not(debug_assertions))]
    let previewing = false;
    if !previewing {
        input::start_listener(sender.clone(), Arc::clone(&shortcut));
    }
    let _theme_monitor = theme::start_listener(sender, shortcut);

    let animation_running = Rc::new(Cell::new(false));
    let receiver_state = Rc::clone(&state);
    let receiver_area = drawing_area.clone();
    let receiver_window = window.clone();
    receiver.attach(None, move |message| {
        match message {
            UiMessage::Key(message) => {
                receiver_state.borrow_mut().push_key(message);
                receiver_area.queue_draw();
                if !animation_running.replace(true) {
                    let area = receiver_area.clone();
                    let animation_running = Rc::clone(&animation_running);
                    let state = Rc::clone(&receiver_state);
                    glib::timeout_add_local(Duration::from_millis(16), move || {
                        area.queue_draw();
                        let animating = state
                            .borrow()
                            .history
                            .iter()
                            .any(|key| key.created.elapsed() < Duration::from_millis(140));
                        if !animating {
                            animation_running.set(false);
                        }
                        ControlFlow::from(animating)
                    });
                }
            }
            UiMessage::Modifiers(modifiers) => {
                receiver_state.borrow_mut().active_modifiers = modifiers;
                receiver_area.queue_draw();
            }
            UiMessage::Theme(theme) => {
                let old_mode = receiver_state.borrow().theme.mode;
                let new_mode = theme.mode;
                receiver_state.borrow_mut().theme = theme;
                if new_mode != old_mode {
                    let height = new_mode.height();
                    receiver_area.set_size_request(WIDTH, height);
                    receiver_window.set_size_request(WIDTH, height);
                    receiver_window.resize(WIDTH, height);
                }
                receiver_area.queue_draw();
            }
        }
        ControlFlow::Continue
    });

    let _tray = setup_tray();
    window.show_all();
    window.stick();
    window.set_accept_focus(false);
    if let Some(native) = window.window() {
        native.stick();
        native.set_accept_focus(false);
    }

    glib::timeout_add_local_once(Duration::from_millis(500), move || {
        position_ready.set(true);
    });
    gtk::main();
}
