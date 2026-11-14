use super::icon;
use crate::{theme::{self, Theme, craft}, ui::{TitleBarExt, design::metrics}};
use gpui::{prelude::FluentBuilder, *};

/// Supply the interaction states independently of a control's production geometry.
pub(crate) fn button_surface(id: impl Into<ElementId>, enabled: bool, hover: Hsla) -> Stateful<Div> {
    div().id(id).when(enabled, |d| pressable(d).hover(move |s| s.bg(hover)))
}

pub(super) fn pressable(d: Stateful<Div>) -> Stateful<Div> {
    d.cursor_pointer().active(|s| s.bg(Theme::fill_pressed()))
}

pub(crate) fn button_focus(d: Stateful<Div>, handle: &FocusHandle, highlight: Hsla) -> Stateful<Div> {
    d.track_focus(handle).focus(move |s| s.bg(highlight).border_color(Theme::primary()))
}

/// The header button: a white pill on the note (hairline, 1px drop, radius 12).
pub fn pill() -> Div {
    div()
        .flex_none()
        .rounded(px(12.))
        .bg(crate::theme::raised(Theme::bg_body()))
        .border_1()
        .border_color(Theme::button_border())
        .shadow(theme::shadow_button())
}

/// The primary button: the accent, a white 13pt semibold label, at the workspace control height.
pub fn primary_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    filled_button(id, label, false)
}

pub(crate) fn filled_button(id: &'static str, label: &'static str, danger: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .h(px(metrics().control_height))
        .px(px(16.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(metrics().control_radius))
        .cursor_pointer()
        .bg(if danger { Theme::danger() } else { Theme::primary() })
        .shadow(theme::shadow_button())
        .hover(move |s| s.bg(if danger { theme::c(craft::R600) } else { crate::ritual::appearance::palette().map_or_else(|| theme::c(craft::B600), |p| p.accent.opacity(0.85)) }))
        .active(move |s| s.bg(if danger { theme::c(craft::R700) } else { crate::ritual::appearance::palette().map_or_else(|| theme::c(craft::B700), |p| p.accent.opacity(0.7)) }))
        .text_size(px(13.))
        .line_height(px(20.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(crate::ritual::appearance::palette().map_or_else(gpui::white, |p| if p.dark { p.page } else { gpui::white() }))
        .child(crate::settings::label(label))
}

/// The secondary button: white with a hairline and a 1px drop, at the workspace control height.
pub fn secondary_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .h(px(metrics().control_height))
        .px(px(16.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(metrics().control_radius))
        .cursor_pointer()
        .bg(crate::theme::raised(Theme::bg_body()))
        .border_1()
        .border_color(Theme::button_border())
        .shadow(theme::shadow_button())
        .hover(|s| s.bg(Theme::button_hover()))
        .active(|s| s.bg(Theme::button_pressed()))
        .text_size(px(13.))
        .line_height(px(20.))
        .text_color(Theme::text())
        .child(crate::settings::label(label))
}

/// The panel close button: a grey disc with a white cross (measured
/// #B6B7B8 on #F7F7F7, black at about 26%).
pub fn close_button(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .size(px(18.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .bg(theme::ca(craft::N1000, 0.26))
        .hover(|s| s.bg(theme::ca(craft::N1000, 0.36)))
        .child(icon("x", 16., gpui::white()))
}

/// A single action with a square hit area and a rounded hover background.
pub(crate) fn icon_button(id: &'static str, glyph: impl IntoElement) -> Stateful<Div> {
    icon_button_frame(id).child(glyph)
}

pub(crate) fn icon_button_frame(id: impl Into<ElementId>) -> Stateful<Div> {
    button_surface(id, true, Theme::fill_hover()).size(px(metrics().toolbar_target)).rounded(px(metrics().control_radius))
        .flex_none().flex().items_center().justify_center()
}

pub fn header_icon(id: &'static str, name: &'static str) -> Stateful<Div> {
    icon_button(id, icon(name, metrics().nav_icon, Theme::icon())).no_drag()
}
