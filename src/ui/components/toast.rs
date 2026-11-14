use super::{icon, ui_text};
use crate::theme::{self, Theme, craft};
use gpui::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// Toast: a white rounded card 32px below the top, centred, with a filled
/// status circle (green check / red x / accent i). It lasts 3s and fades over
/// its last .3s; None once it is over.
pub fn toast_el(kind: ToastKind, msg: &SharedString, t0: std::time::Instant, window: &mut Window) -> Option<AnyElement> {
    let e = t0.elapsed().as_secs_f32();
    if e >= 3. {
        return None;
    }
    let op = if e > 2.7 {
        window.request_animation_frame();
        (3. - e) / 0.3
    } else {
        1.
    };
    let (fill, mark, mark_size) = match kind {
        ToastKind::Success => (theme::c(craft::old::G500), "check", 10.),
        ToastKind::Error => (Theme::danger(), "x", 10.),
        ToastKind::Info => (Theme::primary(), "info-mark", 12.),
    };
    let vw = window.viewport_size().width;
    let card = div()
        .flex()
        .items_center()
        .px(px(14.))
        .py(px(10.))
        .rounded(px(12.))
        .bg(crate::theme::raised(Theme::bg_body()))
        .border_1()
        .border_color(Theme::line_border())
        .shadow(theme::shadow_panel())
        .font_family(theme::ui_font())
        .text_size(px(13.))
        .line_height(px(20.))
        .text_color(Theme::text())
        .child(
            div()
                .relative()
                .size(px(16.))
                .mr(px(8.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(icon("circle-filled", 16., fill).absolute().top_0().left_0())
                .child(icon(mark, mark_size, gpui::white())),
        )
        .child(ui_text(crate::settings::label(msg)));
    Some(
        deferred(
            anchored()
                .position_mode(AnchoredPositionMode::Window)
                .position(point(px(0.), px(32.)))
                .child(div().w(vw).flex().justify_center().opacity(op).child(card)),
        )
        .with_priority(20)
        .into_any_element(),
    )
}
