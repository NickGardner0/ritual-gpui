use super::icon;
use crate::theme::Theme;
use gpui::{prelude::FluentBuilder, *};

pub(crate) fn checkbox(on: bool) -> Div {
    div()
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .when(on, |d| d.bg(Theme::primary()).child(icon("todo-check", 12., Theme::bg_body())))
        .when(!on, |d| d.border_1().border_color(Theme::todo_box()).bg(crate::theme::raised(Theme::bg_body())))
}
