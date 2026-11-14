use super::{TipExt, design::metrics, icon};
use crate::theme::Theme;
use gpui::{prelude::FluentBuilder, *};

/// Window chrome above the page's top line works like a native title bar: a
/// press drags the window, a double press zooms it (per the system setting).
/// Controls on it opt out with `no_drag`, so double-clicking them doesn't zoom.
pub trait TitleBarExt: InteractiveElement + Sized {
    fn title_bar(self) -> Self {
        // Keep the workspace's focus handler from preventing the native caption action.
        if cfg!(target_os = "windows") { return self.occlude().window_control_area(WindowControlArea::Drag); }
        self.on_mouse_down(MouseButton::Left, |ev, window, _| match ev.click_count {
            1 => window.start_window_move(),
            2 => window.titlebar_double_click(),
            _ => {}
        })
    }
    fn no_drag(self) -> Self {
        let element = if cfg!(target_os = "windows") { self.occlude() } else { self };
        element.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    }
}

impl<E: InteractiveElement> TitleBarExt for E {}

pub(crate) const WINDOW_CONTROLS_W: f32 = if cfg!(any(target_os = "windows", target_os = "linux")) { 163. } else { 0. };
pub(crate) fn header_right_inset() -> f32 {
    if cfg!(any(target_os = "windows", target_os = "linux")) { WINDOW_CONTROLS_W } else { (crate::app::HEADER_H - metrics().toolbar_target) / 2. }
}

pub(crate) fn window_controls(window: &Window) -> Option<Div> {
    if !cfg!(any(target_os = "windows", target_os = "linux")) { return None; }
    // Windows must receive mouse-down to arm its native caption buttons.
    let button = |id: &'static str, area: WindowControlArea, label: &'static str| {
        div().id(id).size(px(metrics().toolbar_target)).mx(px(7.)).rounded(px(8.)).flex_none().flex().items_center().justify_center().cursor_pointer()
            .occlude()
            .when(cfg!(target_os = "linux"), |d| d.no_drag())
            .when(cfg!(target_os = "windows"), |d| d.window_control_area(area))
            .when(cfg!(target_os = "linux"), |d| d.on_click(move |_, window, cx| match area {
                WindowControlArea::Min => window.minimize_window(),
                WindowControlArea::Max => window.zoom_window(),
                WindowControlArea::Close => window.dispatch_action(Box::new(crate::app::CloseWindow), cx),
                WindowControlArea::Drag => {},
            })).hover(|s| s.bg(Theme::fill_hover())).active(|s| s.bg(Theme::fill_pressed())).tip(label, None)
    };
    let maximized = window.is_maximized();
    // Native hit-test areas preserve Snap Layouts and the normal WM_CLOSE save guard.
    Some(div().absolute().top_0().right_0().h(px(crate::app::HEADER_H)).flex().items_center().title_bar()
        .child(div().mx(px(12.)).w(px(1.)).h(px(18.)).bg(Theme::line_divider()))
        .child(button("window-minimize", WindowControlArea::Min, crate::settings::tr("最小化", "Minimize"))
            .child(icon("minus", 18., Theme::icon())))
        .child(button("window-maximize", WindowControlArea::Max, if maximized { crate::settings::tr("还原", "Restore") } else { crate::settings::tr("最大化", "Maximize") }).child(icon(if maximized { "copy" } else { "expand" }, 18., Theme::icon())))
        .child(button("window-close", WindowControlArea::Close, crate::settings::tr("关闭", "Close"))
            .child(icon("x", 18., Theme::icon()))))
}
