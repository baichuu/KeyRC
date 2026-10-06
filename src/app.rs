use crate::icons::Icons;
use crate::input;
use crate::model::{AppState, UiMessage, WIDTH};
#[cfg(debug_assertions)]
use crate::model::{DisplayMode, KeyMessage};
use crate::{
    glass::{DesktopMonitor, DesktopSnapshot},
    native_compositor, position, render, theme,
};
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

fn refresh_glass_backdrop(
    desktop: &RefCell<Option<DesktopSnapshot>>,
    state: &RefCell<AppState>,
    area: &gtk::DrawingArea,
    position: (i32, i32),
) {
    let height = {
        let state = state.borrow();
        if !state.theme.liquid_glass || state.native_glass {
            return;
        }
        state.theme.mode.height()
    };
    let Some(snapshot) = &*desktop.borrow() else {
        return;
    };
    let scale = area.scale_factor();
    let frame = snapshot.render(position.0, position.1, WIDTH, height, scale);
    let mut state = state.borrow_mut();
    if let Some((backdrop, luminance)) = frame {
        state.glass_backdrop = Some(backdrop);
        state.glass_luminance = luminance;
    } else {
        state.glass_backdrop = None;
    }
    state.glass_scale = scale;
    area.queue_draw();
}

fn recapture_glass_backdrop(
    window: &gtk::Window,
    desktop: &RefCell<Option<DesktopSnapshot>>,
    state: &RefCell<AppState>,
    area: &gtk::DrawingArea,
    position: (i32, i32),
) {
    {
        let state = state.borrow();
        if !state.theme.liquid_glass || state.native_glass {
            return;
        }
    }

    window.hide();
    if let Some(display) = gdk::Display::default() {
        display.sync();
    }
    *desktop.borrow_mut() = DesktopSnapshot::capture();
    window.move_(position.0, position.1);
    window.show_all();
    window.set_keep_above(true);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.stick();
    window.set_accept_focus(false);
    if let Some(native) = window.window() {
        native.stick();
        native.set_accept_focus(false);
    }
    refresh_glass_backdrop(desktop, state, area, position);
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
            for modifier in parts {
                let key = match modifier {
                    "Shift" => "Shift",
                    "Ctrl" => "Ctrl",
                    "Alt" => "Alt",
                    "Super" => "Super",
                    _ => continue,
                };
                state.push_key(KeyMessage { key });
            }
            let key = match raw_key {
                "Right" => "Right",
                "Left" => "Left",
                "Up" => "Up",
                "Down" => "Down",
                "Enter" => "Enter",
                other => Box::leak(other.to_owned().into_boxed_str()),
            };
            state.push_key(KeyMessage { key });
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
    let keymap = Arc::new(RwLock::new(settings.keymap));
    #[cfg(debug_assertions)]
    let initial_theme = apply_preview_mode(initial_theme);

    let icons = Rc::new(Icons::new());
    let position_path = position::path();
    let initial_position = position::read(position_path.as_deref());
    let native_compositor = Rc::new(RefCell::new(native_compositor::NativeCompositor::start(
        initial_theme.liquid_glass,
    )));
    let native_glass = native_compositor.borrow().active();
    let desktop = Rc::new(RefCell::new(
        if initial_theme.liquid_glass && !native_glass {
            DesktopSnapshot::capture()
        } else {
            None
        },
    ));
    let mut initial_state = AppState::new(initial_theme.clone());
    initial_state.native_glass = native_glass;
    #[cfg(debug_assertions)]
    let initial_state = apply_preview_keys(initial_state);
    let state = Rc::new(RefCell::new(initial_state));

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    configure_window(&window, initial_theme.mode.height());
    if let Some((x, y)) = initial_position {
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

    window.connect_delete_event(|_, _| glib::Propagation::Stop);

    let position_ready = Rc::new(Cell::new(false));
    let current_position = Rc::new(Cell::new(initial_position.unwrap_or((0, 0))));
    let write_position = position_path.map(position::start_writer);
    let configure_ready = Rc::clone(&position_ready);
    let configure_position = Rc::clone(&current_position);
    let configure_state = Rc::clone(&state);
    let configure_area = drawing_area.clone();
    let configure_desktop = Rc::clone(&desktop);
    let glass_update_scheduled = Rc::new(Cell::new(false));
    let configure_scheduled = Rc::clone(&glass_update_scheduled);
    let drag_refresh_running = Rc::new(Cell::new(false));
    let configure_dragging = Rc::clone(&drag_refresh_running);
    window.connect_configure_event(move |_, event| {
        let position = event.position();
        configure_position.set(position);
        if configure_ready.get() {
            if let Some(writer) = &write_position {
                writer(position);
            }
        }
        let glass_needs_snapshot = {
            let state = configure_state.borrow();
            state.theme.liquid_glass && !state.native_glass
        };
        if !configure_dragging.get() && glass_needs_snapshot && !configure_scheduled.replace(true) {
            let position = Rc::clone(&configure_position);
            let state = Rc::clone(&configure_state);
            let area = configure_area.clone();
            let desktop = Rc::clone(&configure_desktop);
            let scheduled = Rc::clone(&configure_scheduled);
            glib::timeout_add_local_once(Duration::from_millis(24), move || {
                refresh_glass_backdrop(&desktop, &state, &area, position.get());
                scheduled.set(false);
            });
        }
        false
    });

    let drag_window = window.clone();
    let drag_area = drawing_area.clone();
    let drag_state = Rc::clone(&state);
    let drag_desktop = Rc::clone(&desktop);
    let drag_position = Rc::clone(&current_position);
    let drag_running = Rc::clone(&drag_refresh_running);
    drawing_area.connect_button_press_event(move |_, event| {
        if event.button() == 1 {
            let (pointer_x, pointer_y) = event.root();
            drag_window.begin_move_drag(1, pointer_x as i32, pointer_y as i32, event.time());
            let glass_needs_snapshot = {
                let state = drag_state.borrow();
                state.theme.liquid_glass && !state.native_glass
            };
            if glass_needs_snapshot && !drag_running.replace(true) {
                let window = drag_window.clone();
                let area = drag_area.clone();
                let state = Rc::clone(&drag_state);
                let desktop = Rc::clone(&drag_desktop);
                let position = Rc::clone(&drag_position);
                let running = Rc::clone(&drag_running);
                let mut rendered_position = position.get();
                glib::timeout_add_local(Duration::from_millis(16), move || {
                    let latest = window.position();
                    if latest != rendered_position {
                        rendered_position = latest;
                        position.set(latest);
                        refresh_glass_backdrop(&desktop, &state, &area, latest);
                    }

                    let button_held = gdk::Display::default()
                        .and_then(|display| display.default_seat())
                        .and_then(|seat| seat.pointer())
                        .and_then(|pointer| {
                            window
                                .window()
                                .map(|native| native.device_position(&pointer).3)
                        })
                        .is_some_and(|mask| mask.contains(gdk::ModifierType::BUTTON1_MASK));
                    if button_held {
                        ControlFlow::Continue
                    } else {
                        let latest = window.position();
                        position.set(latest);
                        refresh_glass_backdrop(&desktop, &state, &area, latest);
                        running.set(false);
                        ControlFlow::Break
                    }
                });
            }
        }
        glib::Propagation::Stop
    });

    #[allow(deprecated)]
    let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
    #[cfg(debug_assertions)]
    let previewing = std::env::var_os("KEYRC_PREVIEW_KEYS").is_some();
    #[cfg(not(debug_assertions))]
    let previewing = false;
    if !previewing {
        input::start_listener(sender.clone(), Arc::clone(&keymap));
    }
    let _theme_monitor = theme::start_listener(sender, keymap);

    let animation_running = Rc::new(Cell::new(false));
    let receiver_state = Rc::clone(&state);
    let receiver_area = drawing_area.clone();
    let receiver_window = window.clone();
    let receiver_position = Rc::clone(&current_position);
    let receiver_desktop = Rc::clone(&desktop);
    let receiver_compositor = Rc::clone(&native_compositor);
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
                let old_theme = receiver_state.borrow().theme.clone();
                let old_mode = old_theme.mode;
                let new_mode = theme.mode;
                let native_glass = receiver_compositor.borrow_mut().ensure(theme.liquid_glass);
                {
                    let mut state = receiver_state.borrow_mut();
                    state.theme = theme;
                    state.native_glass = native_glass;
                    if native_glass {
                        state.glass_backdrop = None;
                    }
                }
                if let Some(native) = receiver_window.window() {
                    native_compositor::mark_window(
                        &native,
                        receiver_state.borrow().theme.liquid_glass && native_glass,
                    );
                }
                if new_mode != old_mode {
                    let height = new_mode.height();
                    receiver_area.set_size_request(WIDTH, height);
                    receiver_window.set_size_request(WIDTH, height);
                    receiver_window.resize(WIDTH, height);
                }
                if receiver_state.borrow().theme.liquid_glass && !native_glass {
                    if receiver_desktop.borrow().is_none() {
                        receiver_window.hide();
                        if let Some(display) = gdk::Display::default() {
                            display.flush();
                        }
                        *receiver_desktop.borrow_mut() = DesktopSnapshot::capture();
                        receiver_window.show_all();
                        receiver_window.stick();
                        receiver_window.set_accept_focus(false);
                    }
                    refresh_glass_backdrop(
                        &receiver_desktop,
                        &receiver_state,
                        &receiver_area,
                        receiver_position.get(),
                    );
                } else if old_theme.liquid_glass && !native_glass {
                    receiver_state.borrow_mut().glass_backdrop = None;
                }
                receiver_area.queue_draw();
            }
            UiMessage::Quit => {
                gtk::main_quit();
                return ControlFlow::Break;
            }
        }
        ControlFlow::Continue
    });

    let _tray = setup_tray();
    window.show_all();
    window.stick();
    window.set_accept_focus(false);
    if let Some(native) = window.window() {
        native_compositor::mark_window(
            &native,
            initial_theme.liquid_glass && state.borrow().native_glass,
        );
    }
    if initial_theme.liquid_glass {
        let position = window.position();
        current_position.set(position);
        refresh_glass_backdrop(&desktop, &state, &drawing_area, position);
    }
    if let Some(native) = window.window() {
        native.stick();
        native.set_accept_focus(false);
    }

    glib::timeout_add_local_once(Duration::from_millis(500), move || {
        position_ready.set(true);
    });

    if let Some(monitor) = DesktopMonitor::new() {
        let monitor = Rc::new(monitor);
        let rendered_context = Rc::new(Cell::new(monitor.context()));
        let refresh_scheduled = Rc::new(Cell::new(false));
        let monitor_window = window.clone();
        let monitor_area = drawing_area.clone();
        let monitor_state = Rc::clone(&state);
        let monitor_desktop = Rc::clone(&desktop);
        let monitor_position = Rc::clone(&current_position);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let context = monitor.context();
            let fallback_active = {
                let state = monitor_state.borrow();
                state.theme.liquid_glass && !state.native_glass
            };
            if !fallback_active {
                rendered_context.set(context);
                return ControlFlow::Continue;
            }
            if context != rendered_context.get() && !refresh_scheduled.replace(true) {
                let monitor = Rc::clone(&monitor);
                let rendered_context = Rc::clone(&rendered_context);
                let refresh_scheduled = Rc::clone(&refresh_scheduled);
                let window = monitor_window.clone();
                let area = monitor_area.clone();
                let state = Rc::clone(&monitor_state);
                let desktop = Rc::clone(&monitor_desktop);
                let position = Rc::clone(&monitor_position);
                glib::timeout_add_local_once(Duration::from_millis(60), move || {
                    let context = monitor.context();
                    if context != rendered_context.get() {
                        let latest = window.position();
                        position.set(latest);
                        recapture_glass_backdrop(&window, &desktop, &state, &area, latest);
                        rendered_context.set(context);
                    }
                    refresh_scheduled.set(false);
                });
            }
            ControlFlow::Continue
        });
    }
    gtk::main();
}
