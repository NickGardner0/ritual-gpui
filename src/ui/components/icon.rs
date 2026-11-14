use gpui::*;

pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    let dir = if crate::ritual::appearance::zed_icons() && matches!(name, "doc" | "doc-md" | "folder" | "folder-open") {
        "ritual/file-icons"
    } else { "icons" };
    svg().path(SharedString::from(format!("{dir}/{name}.svg"))).size(px(size)).flex_none().text_color(color)
}
