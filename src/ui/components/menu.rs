use super::icon;
use crate::{theme::{self, Theme}, ui::design::metrics};
use gpui::{prelude::FluentBuilder, *};

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
    div().id(id).flex_none().h(px(style.menu_height)).px(px(style.row_padding))
        .flex().items_center().gap(px(style.icon_gap)).rounded(px(style.row_radius))
        .text_size(px(13.)).line_height(px(20.)).text_color(if enabled { Theme::text() } else { Theme::text_disabled() })
        .when(enabled, |d| d.cursor_pointer().hover(|s| s.bg(Theme::fill_hover())).active(|s| s.bg(Theme::fill_pressed())))
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
