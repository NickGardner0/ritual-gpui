use super::ui_text;
use crate::theme;
use gpui::*;
use std::rc::Rc;

/// Tooltip bubble: the dark rounded tip (12pt white on near-black, radius 7, arrow).
pub struct Tooltip {
    /// One or more lines.
    label: SharedString,
    /// Window bounds of the trigger, when it was registered through [`TipExt::tip`].
    anchor: Option<Bounds<Pixels>>,
    /// Where GPUI anchors the tooltip root (mouse + 1 at show time).
    origin: Point<Pixels>,
    side: TipSide,
}

/// Preferred side: chrome shows tooltips below, document controls above,
/// menu items to the right. Flipped when there is no room.
#[derive(Clone, Copy, PartialEq)]
pub enum TipSide {
    Below,
    Above,
    Right,
}

const TIP_GAP: f32 = 8.;
const TIP_ARROW: f32 = 5.;
const TIP_PAD: f32 = 12.;
const TIP_LINE: f32 = 18.;
const TIP_BG: u32 = 0x2a2c2f;
/// Tooltips stay dark in both looks (a step lighter on the dark page).
fn tip_bg() -> Hsla { if theme::dark() { theme::rgbaf(0x3a, 0x3b, 0x3d, 1.) } else { theme::rgbaf((TIP_BG >> 16) as u8, (TIP_BG >> 8) as u8, TIP_BG as u8, 1.) } }

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let lines: Vec<SharedString> = self.label.split('\n').map(|l| SharedString::from(l.to_string())).collect();
        let bubble = div()
            .px(px(10.))
            .py(px(6.))
            .rounded(px(7.))
            .bg(tip_bg())
            .text_color(gpui::white())
            .text_size(px(12.))
            .line_height(px(TIP_LINE))
            .font_family(theme::ui_font())
            .whitespace_nowrap()
            .flex()
            .flex_col()
            .children(lines.iter().map(|l| ui_text(l.clone())));
        let Some(anchor) = self.anchor else {
            return div().pl(px(8.)).pt(px(10.)).child(bubble).into_any_element();
        };
        // 10px from the trigger with an arrow pointing at it, on the preferred
        // side unless it doesn't fit, and kept inside the window.
        let mut style = window.text_style();
        style.font_family = theme::ui_font().into();
        let text_w = lines
            .iter()
            .map(|l| {
                let runs = ui_text(l.clone()).runs(&style);
                window.text_system().shape_line(l.clone(), px(12.), &runs, None).width
            })
            .fold(px(0.), |a, b| a.max(b));
        let w = text_w + px(20.);
        let h = px(TIP_PAD + TIP_LINE * lines.len() as f32);
        let vp = window.viewport_size();
        let gap = px(TIP_GAP);
        let s = px(TIP_ARROW);
        // (bubble origin, arrow base centre, arrow direction)
        let (x, y, arrow): (Pixels, Pixels, Box<dyn Fn(&mut PathBuilder)>) = if self.side == TipSide::Right
            && (anchor.right() + gap + w <= vp.width || anchor.left() - gap - w < px(0.))
        {
            let x = anchor.right() + gap;
            let cy = anchor.center().y;
            let y = (cy - h / 2.).min(vp.height - h).max(px(0.));
            let tip_y = cy.max(y + px(10.)).min(y + h - px(10.));
            (x, y, Box::new(move |p: &mut PathBuilder| {
                p.move_to(point(x, tip_y - s));
                p.line_to(point(x - s * 0.88, tip_y - px(0.8)));
                p.line_to(point(x - s * 0.88, tip_y + px(0.8)));
                p.line_to(point(x, tip_y + s));
            }))
        } else if self.side == TipSide::Right {
            let x = anchor.left() - gap - w;
            let cy = anchor.center().y;
            let y = (cy - h / 2.).min(vp.height - h).max(px(0.));
            let tip_y = cy.max(y + px(10.)).min(y + h - px(10.));
            let bx = x + w;
            (x, y, Box::new(move |p: &mut PathBuilder| {
                p.move_to(point(bx, tip_y - s));
                p.line_to(point(bx + s * 0.88, tip_y - px(0.8)));
                p.line_to(point(bx + s * 0.88, tip_y + px(0.8)));
                p.line_to(point(bx, tip_y + s));
            }))
        } else {
            let cx = anchor.center().x;
            let x = (cx - w / 2.).min(vp.width - w).max(px(0.));
            let fits_below = anchor.bottom() + gap + h <= vp.height;
            let fits_above = anchor.top() - gap - h >= px(0.);
            let below = if self.side == TipSide::Above { !fits_above } else { fits_below };
            let y = if below { anchor.bottom() + gap } else { anchor.top() - gap - h };
            let tip_x = cx.max(x + px(10.)).min(x + w - px(10.));
            let (base, apex) = if below { (y, y - s) } else { (y + h, y + h + s) };
            (x, y, Box::new(move |p: &mut PathBuilder| {
                p.move_to(point(tip_x - s, base));
                p.line_to(point(tip_x - px(0.8), apex + (base - apex) * 0.12));
                p.line_to(point(tip_x + px(0.8), apex + (base - apex) * 0.12));
                p.line_to(point(tip_x + s, base));
            }))
        };
        let o = self.origin;
        let arrow = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let mut p = PathBuilder::fill();
                arrow(&mut p);
                p.close();
                if let Ok(path) = p.build() {
                    window.paint_path(path, tip_bg());
                }
            },
        )
        .absolute()
        .size(px(1.));
        div()
            .size(px(1.))
            .child(div().absolute().left(x - o.x).top(y - o.y).child(bubble))
            .child(arrow)
            .into_any_element()
    }
}

/// Tooltip anchored on the element rather than the mouse.
pub trait TipExt: Sized {
    /// Window chrome: below, shortcut in full-width parens ("展开面板（⌘ + ]）").
    fn tip(self, text: &str, keys: Option<&str>) -> Self {
        let text = crate::settings::label(text);
        let label = match keys {
            Some(k) => format!("{text}（{k}）"),
            None => text.to_string(),
        };
        self.tip_label(label, TipSide::Below)
    }
    /// Document controls: above, in the editor's shortcut format
    /// ("粗体 (⌘ + B)"), optionally with a "Markdown: …" second line.
    fn tip_above(self, text: &str, keys: Option<&str>) -> Self {
        self.tip_label(doc_tip(text, keys, None), TipSide::Above)
    }
    fn tip_md(self, text: &str, keys: Option<&str>, md: Option<&str>) -> Self {
        self.tip_label(doc_tip(text, keys, md), TipSide::Above)
    }
    /// Menu items: to the right.
    fn tip_right(self, text: &str, keys: Option<&str>) -> Self {
        self.tip_label(doc_tip(text, keys, None), TipSide::Right)
    }
    fn tip_label(self, label: String, side: TipSide) -> Self;
}

/// "名称 (键 + 键)" plus an optional "Markdown: …" line.
pub fn doc_tip(text: &str, keys: Option<&str>, md: Option<&str>) -> String {
    let text = crate::settings::text(text);
    let mut s = match keys {
        Some(k) => format!("{text} ({k})"),
        None => text.to_string(),
    };
    if let Some(m) = md {
        s.push_str("\nMarkdown: ");
        s.push_str(crate::settings::text(m));
    }
    s
}

impl TipExt for Stateful<Div> {
    fn tip_label(self, label: String, side: TipSide) -> Self {
        let bounds = Rc::new(std::cell::Cell::new(None::<Bounds<Pixels>>));
        let record = bounds.clone();
        let label = SharedString::from(label);
        self.child(canvas(move |b, _, _| record.set(Some(b)), |_, _, _, _| {}).absolute().inset_0()).tooltip(move |window, cx| {
            let origin = window.mouse_position() + point(px(1.), px(1.));
            let (label, anchor) = (label.clone(), bounds.get());
            cx.new(|_| Tooltip { label, anchor, origin, side }).into()
        })
    }
}
