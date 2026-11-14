use crate::{theme::{self, Theme, craft}, ui::design::metrics};
use gpui::*;

/// The window dimmed under a modal dialog: a light black veil.
pub fn scrim() -> Hsla {
    theme::ca(craft::N1000, 0.25)
}

/// Raised modal surface, shared by workspace dialogs.
pub fn dialog(id: &'static str) -> Stateful<Div> {
    div().id(id).occlude().rounded(px(metrics().dialog_radius)).bg(crate::theme::raised(Theme::bg_body())).shadow(theme::shadow_dialog()).font_family(theme::ui_font()).text_color(Theme::text())
}
