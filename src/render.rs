use crate::icons::{draw as draw_icon, Icons, ModifierIcon, SpecialIcon};
use crate::keys::{display_text, is_function_key};
use crate::model::{
    AppState, Color, DisplayMode, StoredKey, KEY_HEIGHT, MAX_DISPLAY_UNITS, MODIFIER_HEIGHT,
    MODIFIER_Y, WIDTH,
};
use gtk::cairo::{Context, Operator};
use gtk::glib::translate::{from_glib_full, ToGlibPtr};
use gtk::pango::{FontDescription, Layout};
use std::f64::consts::{FRAC_PI_2, PI};

const CORNER_RADIUS: f64 = 24.0;
const FONT: &str = "Iosevka Nerd Font Mono";

#[link(name = "pangocairo-1.0")]
unsafe extern "C" {
    fn pango_cairo_create_layout(
        context: *mut gtk::cairo::ffi::cairo_t,
    ) -> *mut gtk::pango::ffi::PangoLayout;
    fn pango_cairo_show_layout(
        context: *mut gtk::cairo::ffi::cairo_t,
        layout: *mut gtk::pango::ffi::PangoLayout,
    );
}

fn rounded_panel(context: &Context, x: f64, y: f64, width: f64, height: f64, corners: u8) {
    let right = x + width;
    let bottom = y + height;
    let radius = CORNER_RADIUS.min(width / 2.0).min(height / 2.0);
    context.new_sub_path();
    context.move_to(x + if corners & 1 != 0 { radius } else { 0.0 }, y);
    context.line_to(right - if corners & 2 != 0 { radius } else { 0.0 }, y);
    if corners & 2 != 0 {
        context.arc(right - radius, y + radius, radius, -FRAC_PI_2, 0.0);
    }
    context.line_to(right, bottom - if corners & 4 != 0 { radius } else { 0.0 });
    if corners & 4 != 0 {
        context.arc(right - radius, bottom - radius, radius, 0.0, FRAC_PI_2);
    }
    context.line_to(x + if corners & 8 != 0 { radius } else { 0.0 }, bottom);
    if corners & 8 != 0 {
        context.arc(x + radius, bottom - radius, radius, FRAC_PI_2, PI);
    }
    context.line_to(x, y + if corners & 1 != 0 { radius } else { 0.0 });
    if corners & 1 != 0 {
        context.arc(x + radius, y + radius, radius, PI, PI + FRAC_PI_2);
    }
    context.close_path();
}

fn fill_panel(
    context: &Context,
    state: &AppState,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corners: u8,
) {
    rounded_panel(context, x, y, width, height, corners);
    state.theme.background.set(context);
    let _ = context.fill_preserve();
    state.theme.border.set(context);
    context.set_line_width(1.0);
    let _ = context.stroke();
}

fn text_layout(context: &Context, text: &str, size: f64) -> Layout {
    let layout: Layout =
        unsafe { from_glib_full(pango_cairo_create_layout(context.to_raw_none())) };
    let mut description = FontDescription::from_string(FONT);
    description.set_absolute_size(size * f64::from(gtk::pango::SCALE));
    layout.set_font_description(Some(&description));
    layout.set_text(text);
    layout
}

fn text_width(context: &Context, text: &str, size: f64) -> f64 {
    f64::from(text_layout(context, text, size).pixel_size().0)
}

fn draw_centered_text(context: &Context, text: &str, center_x: f64, center_y: f64, size: f64) {
    let layout = text_layout(context, text, size);
    let (width, height) = layout.pixel_size();
    context.move_to(
        center_x - f64::from(width) / 2.0,
        center_y - f64::from(height) / 2.0,
    );
    unsafe {
        pango_cairo_show_layout(context.to_raw_none(), layout.to_glib_none().0);
    }
}

fn key_width(context: &Context, key: &StoredKey, mode: DisplayMode, caps_lock: bool) -> f64 {
    if SpecialIcon::from_key(key.key).is_some() {
        if mode == DisplayMode::KeysOnly {
            44.0
        } else {
            36.0
        }
    } else if ModifierIcon::from_key(key.key).is_some() {
        36.0
    } else {
        text_width(context, &display_text(key.key, caps_lock), 36.0)
    }
}

fn draw_special_key(
    context: &Context,
    icons: &Icons,
    key: &str,
    x: f64,
    center_y: f64,
    large: bool,
    color: Color,
) {
    let Some(icon) = SpecialIcon::from_key(key) else {
        return;
    };
    let icon = icons.special(icon, large);
    draw_icon(
        context,
        icon,
        x,
        center_y - f64::from(icon.height()) / 2.0,
        color,
    );
}

fn draw_modifier(
    context: &Context,
    icons: &Icons,
    modifier: ModifierIcon,
    x: f64,
    y: f64,
    large: bool,
    color: Color,
) {
    let icon = icons.modifier(modifier, large);
    draw_icon(
        context,
        icon,
        x - f64::from(icon.width()) / 2.0,
        y - f64::from(icon.height()) / 2.0,
        color,
    );
}

fn draw_key_token(
    context: &Context,
    icons: &Icons,
    key: &StoredKey,
    state: &AppState,
    x: f64,
    width: f64,
) {
    let progress = (key.created.elapsed().as_secs_f64() / 0.14).min(1.0);
    let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
    let center_x = x + width / 2.0;
    let center_y = f64::from(KEY_HEIGHT) / 2.0 + 4.0 * (1.0 - eased);
    context.save().ok();
    context.translate(center_x, center_y);
    context.scale(0.92 + 0.08 * eased, 0.92 + 0.08 * eased);
    context.translate(-center_x, -center_y);
    let key_color = if is_function_key(key.key) {
        state.theme.active_fg
    } else {
        state.theme.key_text
    }
    .with_alpha(eased);
    if SpecialIcon::from_key(key.key).is_some() {
        draw_special_key(
            context,
            icons,
            key.key,
            x,
            center_y,
            state.theme.mode == DisplayMode::KeysOnly,
            key_color,
        );
    } else if let Some(icon) = ModifierIcon::from_key(key.key) {
        draw_modifier(context, icons, icon, x + 18.0, center_y, true, key_color);
    } else {
        let size = 36.0;
        let text = display_text(key.key, state.caps_lock);
        key_color.set(context);
        draw_centered_text(
            context,
            &text,
            x + text_width(context, &text, size) / 2.0,
            center_y,
            size,
        );
    }
    context.restore().ok();
}

fn draw_modifier_row(context: &Context, icons: &Icons, state: &AppState) {
    let panels = [
        (ModifierIcon::Shift, state.active_modifiers.shift),
        (ModifierIcon::Ctrl, state.active_modifiers.ctrl),
        (ModifierIcon::Alt, state.active_modifiers.alt),
        (ModifierIcon::Super, state.active_modifiers.super_key),
    ];
    for (index, (icon, active)) in panels.into_iter().enumerate() {
        let x = index as f64 * 73.0;
        let corners = match index {
            0 => 8,
            3 => 4,
            _ => 0,
        };
        rounded_panel(
            context,
            x + 0.5,
            MODIFIER_Y + 0.5,
            70.0,
            MODIFIER_HEIGHT - 1.0,
            corners,
        );
        if active {
            state.theme.active_bg.set(context);
        } else {
            state.theme.background.set(context);
        }
        let _ = context.fill_preserve();
        state.theme.border.set(context);
        context.set_line_width(1.0);
        let _ = context.stroke();
        let color = if active {
            state.theme.active_fg
        } else {
            state.theme.key_text.with_alpha(0.35)
        };
        draw_modifier(
            context,
            icons,
            icon,
            x + 35.5,
            MODIFIER_Y + MODIFIER_HEIGHT / 2.0,
            false,
            color,
        );
    }
}

fn stroke_full_outline(context: &Context, state: &AppState) {
    rounded_panel(
        context,
        0.5,
        0.5,
        f64::from(WIDTH) - 1.0,
        f64::from(state.theme.mode.height()) - 1.0,
        15,
    );
    state.theme.border.set(context);
    context.set_line_width(1.0);
    let _ = context.stroke();
}

pub(crate) fn draw(context: &Context, icons: &Icons, state: &AppState) {
    context.set_operator(Operator::Source);
    context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    let _ = context.paint();
    context.set_operator(Operator::Over);
    context.push_group();
    if state.theme.mode == DisplayMode::Full {
        rounded_panel(
            context,
            0.5,
            0.5,
            f64::from(WIDTH) - 1.0,
            f64::from(state.theme.mode.height()) - 1.0,
            15,
        );
        state.theme.background.set(context);
        let _ = context.fill();
    }
    fill_panel(
        context,
        state,
        0.5,
        0.5,
        f64::from(WIDTH) - 1.0,
        f64::from(KEY_HEIGHT) - 1.0,
        if state.theme.mode == DisplayMode::KeysOnly {
            15
        } else {
            3
        },
    );
    let mut shown: Vec<_> = state.history.iter().take(MAX_DISPLAY_UNITS).collect();
    shown.reverse();
    let widths: Vec<f64> = shown
        .iter()
        .map(|key| key_width(context, key, state.theme.mode, state.caps_lock))
        .collect();
    let total = widths.iter().sum::<f64>() + 4.0 * widths.len().saturating_sub(1) as f64;
    let mut x = (f64::from(WIDTH) - total) / 2.0;
    for (key, width) in shown.into_iter().zip(widths) {
        draw_key_token(context, icons, key, state, x, width);
        x += width + 4.0;
    }
    if state.theme.mode == DisplayMode::Full {
        draw_modifier_row(context, icons, state);
        stroke_full_outline(context, state);
    }
    let _ = context.pop_group_to_source();
    let _ = context.paint_with_alpha(state.theme.opacity);
}
