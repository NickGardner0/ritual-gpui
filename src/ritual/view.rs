use super::*;
use crate::ui::design::RITUAL;
use crate::{theme::Theme, ui::{self, TipExt}};
use gpui::prelude::FluentBuilder;

fn button(id: &'static str, label: &'static str, primary: bool) -> Stateful<Div> {
    let button = div().id(id).h(px(32.)).px(px(12.)).flex().items_center().justify_center().rounded(px(7.))
        .border_1().border_color(Theme::index_border()).cursor_pointer().child(label);
    let button = if primary { button.bg(Theme::navigation_ink(true)).text_color(Theme::bg_body()).hover(|s| s.opacity(0.87)) }
        else { button.text_color(Theme::navigation_ink(false)).hover(|s| s.bg(Theme::navigation_highlight())) };
    button.tab_index(4).focus(|el| el.border_1().border_color(Theme::primary()))
}

impl Habits {
    pub(crate) fn refresh_colors(&mut self, cx: &mut Context<Self>) {
        self.amount.update(cx, |f, cx| { f.color = Theme::text(); f.placeholder_color = Theme::text_placeholder(); cx.notify(); });
        cx.notify();
    }

    fn field(&self, field: &Entity<TextField>, label: &'static str, suffix: Option<&str>,
             window: &Window, cx: &App) -> Div {
        let handle = field.read(cx).focus_handle(cx);
        let focused = handle.is_focused(window);
        let control = div().id(label).w_full().h(px(36.)).px(px(10.)).flex().items_center().gap(px(8.))
            .border_1().rounded(px(RITUAL.control_radius)).border_color(if focused { Theme::primary() } else { Theme::line_border() }).bg(Theme::bg_body())
            .when(self.saving, |el| el.child(div().flex_1().child(field.read(cx).text.clone())))
            .when(!self.saving, |el| el.cursor(CursorStyle::IBeam).on_click(move |_, window, _| handle.focus(window)).child(field.clone()))
            .when_some(suffix, |el, unit| el.child(div().text_color(Theme::text_caption()).child(unit.to_owned())));
        div().flex().flex_col().gap(px(6.)).child(div().text_size(px(12.)).font_weight(FontWeight::MEDIUM).child(label)).child(control)
    }

    fn detail(&self, habit: &Habit, window: &Window, cx: &mut Context<Self>) -> Div {
        let id = habit.id;
        let mut view = div().flex().flex_col().size_full()
            .child(div().mb(px(16.)).flex().child(button("all-habits", "‹  All habits", false)
                .on_click(cx.listener(|v, _, w, cx| { v.show_list(cx); v.focus.focus(w); }))))
            .child(title(habit.name.clone())).child(div().mt(px(5.)).text_color(Theme::text_caption()).child(habit.category.clone()))
            .child(div().mt(px(30.)).flex().gap(px(8.)).items_end()
                .child(div().w(px(210.)).child(self.field(&self.amount, "Amount", Some(&habit.unit), window, cx)))
                .child(button("log", if self.saving { "Saving…" } else { "Log" }, true)
                    .when(self.saving || self.history.is_none(), |el| el.opacity(0.5))
                    .on_click(cx.listener(|view, _, window, cx| view.submit(window, cx)))))
            .when_some(self.error.clone(), |el, msg| el.child(div().mt(px(8.)).text_color(Theme::danger()).child(msg)))
            .child(div().mt(px(38.)).pb(px(12.)).border_b_1().border_color(Theme::line_border())
                .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child("History")));
        match &self.history {
            Some(rows) if rows.is_empty() => {
                view = view.child(div().py(px(17.)).text_color(Theme::text_caption()).child("No entries yet."));
            }
            Some(rows) => {
                view = view.child(div().h(px(34.)).flex_none().flex().items_center().text_size(px(12.))
                        .text_color(Theme::text_placeholder()).child(div().w(px(68.)).child("Amount"))
                        .child(div().flex_1().child("Unit")).child("Logged at"))
                    .child(uniform_list("history", rows.len(), cx.processor(move |view, range: std::ops::Range<usize>, _, _| {
                        let Some(rows) = &view.history else { return Vec::new() };
                        let Some(habit) = view.habits.iter().find(|h| h.habit.id == id) else { return Vec::new() };
                        range.filter_map(|i| rows.get(i)).map(|entry| {
                            div().w_full().h(px(43.)).flex().items_center().border_b_1().border_color(Theme::line_border().opacity(0.55))
                                .child(div().w(px(68.)).font_weight(FontWeight::MEDIUM).child(entry.amount.to_string()))
                                .child(div().flex_1().min_w_0().truncate().child(habit.habit.unit.clone()))
                                .child(div().text_color(Theme::text_caption()).child(entry.display_time()))
                        }).collect()
                    })).w_full().flex_1().min_h_0());
            }
            None if self.error.is_some() => view = view.child(div().mt(px(10.)).flex().child(button("reload", "Retry", false)
                .on_click(cx.listener(move |view, _, window, cx| view.select(id, window, cx))))),
            None => view = view.child(div().py(px(17.)).text_color(Theme::text_placeholder()).child("Loading history…")),
        }
        view
    }
}

fn title(name: impl Into<SharedString>) -> Div {
    div().text_size(px(27.)).line_height(px(34.)).font_weight(FontWeight::SEMIBOLD).child(name.into())
}

impl Render for Habits {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = div().id("habit-content").flex_1().min_h_0().w_full().p(px(26.));
        if let Some(h) = self.habits.iter().find(|h| Some(h.habit.id) == self.selected) {
            content = content.child(self.detail(&h.habit, window, cx));
        } else if self.db.is_some() {
            let mut items = div().w_full().mx_auto().flex();
            if self.grid {
                let width = f32::from(window.viewport_size().width);
                let columns = if width >= 1024. { 3 } else if width >= 640. { 2 } else { 1 };
                items = items.grid().grid_cols(columns).max_w(px(960.)).gap(px(10.));
            }
            else { items = items.max_w(px(408.)).flex_col(); }
            let range = self.period.range(chrono::Local::now().date_naive());
            for h in &self.habits {
                let id = h.habit.id;
                let row = div().id(SharedString::from(id.to_string())).tab_index(0).group("index-habit").flex().rounded(px(6.)).cursor_pointer()
                    .focus(|s| s.bg(Theme::navigation_highlight()))
                    .hover(|s| s.bg(Theme::navigation_highlight()))
                    .on_click(cx.listener(move |v, _, w, cx| v.select(id, w, cx)));
                let name = div().min_w_0().text_color(Theme::index_name()).child(h.habit.name.clone());
                let value = display_amount(range.as_ref().map_or_else(|| h.value(), |r| self.days.value(h, r)));
                items = items.child(if self.grid {
                    row.min_w_0().h(px(150.)).px(px(16.)).py(px(14.)).flex_col().rounded(px(4.))
                        .border_1().border_color(Theme::index_border())
                        .child(div().text_size(px(12.)).text_color(Theme::text_caption()).child(h.habit.category.clone()))
                        .child(name.w_full().overflow_hidden().max_h(px(42.)).mt(px(3.)).text_size(px(17.)).line_height(px(21.))).child(div().flex_1()).child(div().text_size(px(22.)).text_color(Theme::index_value()).child(format!("{} {}", value, h.habit.unit)))
                } else {
                    let remove = div().id(SharedString::from(format!("remove-{id}"))).size(px(20.)).flex_none().rounded_full()
                        .flex().items_center().justify_center().invisible().group_hover("index-habit", |s| s.visible())
                        .hover(|s| s.bg(Theme::fill_hover_strong())).tip("Delete habit", None)
                        .child(ui::icon("x", 11., Theme::index_secondary()))
                        .on_click(cx.listener(move |v, _, w, cx| { cx.stop_propagation(); v.confirm_remove(id, w, cx); }));
                    row.w_full().h(px(27.)).px(px(4.)).flex_none().gap(px(16.)).items_center().text_size(px(17.5)).line_height(px(19.))
                        .on_key_down(cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                            if ev.keystroke.key == "delete" || ev.keystroke.key == "backspace" {
                                v.confirm_remove(id, w, cx); cx.stop_propagation();
                            }
                        }))
                        .child(name.flex_1().truncate()).child(div().flex().items_center().gap(px(4.)).flex_none()
                            .child(div().text_color(Theme::index_value()).child(value))
                            .child(div().text_color(Theme::index_name()).child(h.habit.unit.clone())).child(remove))
                });
            }
            if self.habits.is_empty() { content = content.child(div().size_full().flex().flex_col().items_center().justify_center()
                .gap(px(6.)).child(title("Track your first habit")).child(div().text_color(Theme::text_caption()).child("Open Actions and choose Add habit."))); }
            else { content = content.overflow_y_scroll().child(items); }
            if let Some(error) = &self.error { content = content.child(div().pt(px(8.)).text_center().text_color(Theme::danger()).child(error.clone())); }
        } else if let Some(error) = &self.error {
            content = content.child(title("Could not open habits")).child(div().my(px(15.)).text_color(Theme::danger()).child(error.clone()))
                .child(div().flex().child(button("retry-open", "Retry", true)
                    .on_click(cx.listener(|view, _, window, cx| view.open_database(window, cx)))));
        } else {
            content = content.child("Opening habits…");
        }
        div().flex_1().min_h_0().flex().flex_col().tab_group().track_focus(&self.focus).key_context("Habits")
            .font_family(super::appearance::navigation_font(cx)).text_size(px(14.)).text_color(Theme::navigation_ink(false))
            .on_action(|_: &input::Tab, window, _| window.focus_next())
            .on_action(|_: &input::ShiftTab, window, _| window.focus_prev())
            .on_action(cx.listener(|view, _: &input::Cancel, window, cx| { view.show_list(cx); view.focus.focus(window); }))
            .child(content)
    }
}

/// Dashboard precision: at most two decimals and grouped thousands.
pub(super) fn display_amount(value: f64) -> String {
    let raw = format!("{value:.2}");
    let (whole, fraction) = raw.split_once('.').unwrap_or((&raw, ""));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) { out.push(','); }
        out.push(c);
    }
    let fraction = fraction.trim_end_matches('0');
    if !fraction.is_empty() { out.push('.'); out.push_str(fraction); }
    out
}
