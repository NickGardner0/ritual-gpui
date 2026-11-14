use crate::theme;
use gpui::*;
use std::{cell::RefCell, rc::Rc};

/// Full-width CJK punctuation (、。〈〉《》「」『』【】〔〕（）：，… in U+3001–301B / U+FF01–FF65).
pub fn is_cjk_punct(ch: char) -> bool {
    matches!(ch, '\u{3001}'..='\u{3003}' | '\u{3008}'..='\u{3011}' | '\u{3014}'..='\u{301B}'
        | '\u{FF01}'..='\u{FF0F}' | '\u{FF1A}'..='\u{FF20}' | '\u{FF3B}'..='\u{FF40}' | '\u{FF5B}'..='\u{FF65}')
}

/// Font for full-width punctuation. When CoreText falls back from the
/// system UI font it squeezes these (an opening paren after a CJK
/// character loses ~5px). Browsers draw them full width with PingFang SC.
pub const CJK_PUNCT_FONT: &str = "PingFang SC";

/// Single-line UI text that keeps full-width punctuation at full width,
/// with an optional colored range (search hits). Inherits the parent text
/// style; honours `text_ellipsis()` against the final width (StyledText
/// caches its first, unconstrained measurement when nowrap).
pub struct UiText {
    text: SharedString,
    hit: Option<(std::ops::Range<usize>, Hsla)>,
}

pub fn ui_text(text: impl Into<SharedString>) -> UiText {
    UiText { text: text.into(), hit: None }
}

pub struct UiTextLayout {
    runs: Vec<TextRun>,
    font_size: Pixels,
    line_height: Pixels,
    ellipsis: Option<SharedString>,
    full: Rc<RefCell<Option<ShapedLine>>>,
}

impl UiText {
    pub fn hit(mut self, range: Option<std::ops::Range<usize>>, color: Hsla) -> Self {
        self.hit = range.map(|r| (r, color));
        self
    }

    pub(super) fn runs(&self, style: &TextStyle) -> Vec<TextRun> {
        match &self.hit {
            Some((r, c)) => ui_runs(&self.text, style, std::slice::from_ref(r), *c),
            None => ui_runs(&self.text, style, &[], style.color),
        }
    }
}

/// Text runs that keep full-width punctuation at full width (see
/// [`CJK_PUNCT_FONT`]), with `hits` drawn in `hit_color`.
pub fn ui_runs(text: &str, style: &TextStyle, hits: &[std::ops::Range<usize>], hit_color: Hsla) -> Vec<TextRun> {
    let mut cuts = vec![0, text.len()];
    for (i, ch) in text.char_indices() {
        if is_cjk_punct(ch) {
            cuts.extend([i, i + ch.len_utf8()]);
        }
    }
    for r in hits {
        cuts.extend([r.start.min(text.len()), r.end.min(text.len())]);
    }
    cuts.sort();
    cuts.dedup();
    cuts.windows(2)
        .map(|w| {
            let mut run = style.to_run(w[1] - w[0]);
            if text[w[0]..w[1]].chars().all(is_cjk_punct) {
                run.font.family = CJK_PUNCT_FONT.into();
            }
            if hits.iter().any(|r| r.start <= w[0] && w[1] <= r.end) {
                run.color = hit_color;
            }
            run
        })
        .collect()
}

/// Longest prefix of `line` that fits `width` together with `suffix`.
fn truncated(line: &ShapedLine, runs: &[TextRun], suffix: &str, font_size: Pixels, width: Pixels, window: &mut Window) -> ShapedLine {
    let text = line.text.clone();
    let mut suffix_runs: Vec<TextRun> = runs.last().cloned().into_iter().collect();
    if let Some(r) = suffix_runs.first_mut() {
        r.len = suffix.len();
        r.font.family = theme::ui_font().into();
    }
    let suffix_w = window.text_system().shape_line(suffix.to_string().into(), font_size, &suffix_runs, None).width;
    let mut cut = 0;
    for (i, ch) in text.char_indices() {
        let end = i + ch.len_utf8();
        if line.x_for_index(end) + suffix_w > width {
            break;
        }
        cut = end;
    }
    let mut out = vec![];
    let mut at = 0;
    for r in runs {
        if at >= cut {
            break;
        }
        let mut r = r.clone();
        r.len = r.len.min(cut - at);
        at += r.len;
        out.push(r);
    }
    out.extend(suffix_runs);
    let s: SharedString = format!("{}{}", &text[..cut], suffix).into();
    window.text_system().shape_line(s, font_size, &out, None)
}

impl IntoElement for UiText {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for UiText {
    type RequestLayoutState = UiTextLayout;
    type PrepaintState = Option<ShapedLine>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, UiTextLayout) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = style.line_height.to_pixels(font_size.into(), window.rem_size());
        let runs = if self.text.is_empty() { vec![] } else { self.runs(&style) };
        let ellipsis = style.text_overflow.as_ref().map(|TextOverflow::Truncate(s)| s.clone());
        let full = Rc::new(RefCell::new(None));
        let (text, r, f, clip) = (self.text.clone(), runs.clone(), full.clone(), ellipsis.is_some());
        let id = window.request_measured_layout(Style::default(), move |known, avail, window, _| {
            let mut slot = f.borrow_mut();
            let line = slot.get_or_insert_with(|| window.text_system().shape_line(text.clone(), font_size, &r, None));
            let mut w = line.width.ceil();
            if clip {
                let limit = known.width.or(match avail.width {
                    AvailableSpace::Definite(x) => Some(x),
                    _ => None,
                });
                if let Some(l) = limit {
                    w = w.min(l);
                }
            }
            size(w, line_height)
        });
        (id, UiTextLayout { runs, font_size, line_height, ellipsis, full })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut UiTextLayout,
        window: &mut Window,
        _: &mut App,
    ) -> Option<ShapedLine> {
        let full = layout.full.borrow().clone()?;
        match &layout.ellipsis {
            Some(suffix) if full.width > bounds.size.width + px(0.5) => {
                Some(truncated(&full, &layout.runs, suffix, layout.font_size, bounds.size.width, window))
            }
            _ => Some(full),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut UiTextLayout,
        line: &mut Option<ShapedLine>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(line) = line {
            let _ = line.paint(bounds.origin, layout.line_height, window, cx);
        }
    }
}

/// [`UiText`] that wraps: a paragraph keeping full-width punctuation at full
/// width, in the inherited text style.
pub fn ui_para(text: impl Into<SharedString>) -> UiPara {
    UiPara { text: text.into(), inner: None }
}

pub struct UiPara {
    text: SharedString,
    inner: Option<StyledText>,
}

impl IntoElement for UiPara {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for UiPara {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let style = window.text_style();
        let runs = ui_runs(&self.text, &style, &[], style.color);
        self.inner.insert(StyledText::new(self.text.clone()).with_runs(runs)).request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(t) = &mut self.inner {
            t.prepaint(id, inspector_id, bounds, layout, window, cx)
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut (),
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(t) = &mut self.inner {
            t.paint(id, inspector_id, bounds, layout, state, window, cx)
        }
    }
}
