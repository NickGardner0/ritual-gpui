use super::icon;
use crate::{theme::{self, Theme}, ui::design::metrics};
use gpui::*;

/// Popover shell shared by the slash menu, block menu and toolbar dropdowns:
/// the light panel (macOS 26 menu material, near-white), hairline,
/// rounded edge and soft shadow.
pub fn popover() -> Div {
    div()
        .bg(crate::theme::raised(Theme::popover_bg()))
        .border_1()
        .border_color(Theme::line_border())
        .rounded(px(metrics().menu_radius))
        .shadow(theme::shadow_menu())
        .font_family(theme::ui_font())
        .text_color(Theme::text())
}

/// Legacy editor row geometry; its inline tools have their own layout.
pub const ROW_RADIUS: f32 = 6.;

/// A menu row: 32 tall, 13pt label, 16pt glyph; hovering lays the quiet
/// fill the editor's menus use, and text and glyph keep their colours.
pub const MENU_ROW_H: f32 = 32.;
pub const MENU_PAD_Y: f32 = 6.;
/// How far a menu row sits in from the menu's sides.
pub const MENU_INSET: f32 = 5.;

/// A full-width interactive option. The containing list supplies its outer inset.
pub(crate) fn option_row(id: impl Into<ElementId>) -> Stateful<Div> {
    option_row_enabled(id, true)
}

pub(crate) fn option_row_enabled(id: impl Into<ElementId>, enabled: bool) -> Stateful<Div> {
    let style = metrics();
    super::button_surface(id, enabled, Theme::fill_hover()).flex_none().h(px(style.menu_height)).px(px(style.row_padding))
        .flex().items_center().gap(px(style.icon_gap)).rounded(px(style.row_radius))
        .text_size(px(13.)).line_height(px(20.)).text_color(if enabled { Theme::text() } else { Theme::text_disabled() })
}

pub(crate) fn menu_choice(id: impl Into<ElementId>, focus: &FocusHandle, hover: Hsla) -> Stateful<Div> {
    let handle = focus.clone();
    // Keep one highlighted row even when an arrow key moves focus away from a stationary pointer.
    super::button_focus(super::pressable(div().id(id)), focus, hover).flex_none().flex().items_center()
        .on_mouse_move(move |_, win, _| { if !handle.is_focused(win) { handle.focus(win); } })
}

/// Only actionable rows get a handle. Headings and separators never enter the cycle.
pub(crate) struct MenuFocus<const N: usize> { pub(crate) items: [FocusHandle; N] }
impl<const N: usize> MenuFocus<N> {
    pub(crate) fn new(cx: &App) -> Self { Self { items: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)) } }

    pub(crate) fn navigate(&self, key: &Keystroke, window: &mut Window) -> Option<usize> {
        let current = self.items.iter().position(|h| h.is_focused(window));
        let index = match plain_key(key)? {
            "down" | "tab" => current.map_or(0, |i| (i + 1) % N),
            "up" | "shift-tab" => current.map_or(N - 1, |i| (i + N - 1) % N),
            "home" => 0, "end" => N - 1, _ => return None,
        };
        self.items[index].focus(window);
        Some(index)
    }
}

pub(crate) fn plain_key(key: &Keystroke) -> Option<&str> {
    if key.key == "tab" && key.modifiers == (Modifiers { shift: true, ..Default::default() }) { Some("shift-tab") }
    else if !key.modifiers.modified() { Some(&key.key) } else { None }
}

pub(crate) fn traverse_tab(key: &Keystroke, window: &mut Window) -> bool {
    match plain_key(key) {
        Some("tab") => window.focus_next(), Some("shift-tab") => window.focus_prev(), _ => return false,
    }
    true
}

pub fn menu_row(id: impl Into<ElementId>, group: SharedString) -> Stateful<Div> {
    option_row(id).group(group).mx(px(MENU_INSET))
}

/// A [`menu_row`]'s label; it keeps its colour on hover (the row's fill
/// shows it). `_group` and `_hover` are left from the accent highlight.
pub fn menu_label(label: impl Into<SharedString>, _group: SharedString, _hover: Hsla) -> Div {
    div().whitespace_nowrap().child(crate::settings::label(label.into()))
}

/// Glyph of a [`menu_row`]: secondary, on hover too.
pub fn menu_glyph(name: &str, size: f32, _group: SharedString) -> Svg { icon(name, size, Theme::icon()) }
