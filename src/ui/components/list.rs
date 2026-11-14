use crate::{theme::{self, Theme, craft}, ui::design::metrics};
use gpui::*;

/// Column headers of the page lists: tertiary text (measured #88898B on
/// the name / last-viewed / created columns).
pub fn list_header_text() -> Hsla {
    theme::c(0x88898b)
}

/// Hairline under a list header and between rows (black 4.5%).
pub fn list_line() -> Hsla {
    crate::ritual::appearance::palette().map_or_else(|| theme::ca(craft::N1000, 0.045), |p| p.border)
}

/// The quiet list hover: a rounded fill that also hides the row's
/// hairline above it.
pub fn row_hover(s: StyleRefinement) -> StyleRefinement {
    s.bg(Theme::fill_hover()).border_color(transparent_black()).rounded(px(metrics().row_radius))
}
