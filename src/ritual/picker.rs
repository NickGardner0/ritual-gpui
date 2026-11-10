//! Home's add-habit palette. Provider presets can be logged manually until a source is connected.
use crate::ui::design::RITUAL;
use super::{Habits, TrackedHabit, catalog::{Source, SOURCES, METRICS}};
use crate::{input::{self, TextField, TextFieldEvent}, theme::Theme, ui};
use gpui::{prelude::FluentBuilder, *};
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;

pub(crate) struct PickerClosed { pub created: bool }
impl EventEmitter<PickerClosed> for Picker {}

#[derive(Clone, Copy)]
enum Step { Sources, Presets(&'static Source), Configure(&'static Source, Option<&'static str>) }
#[derive(Clone, Copy)]
enum Choice { Source(&'static Source), Preset(&'static Source, &'static str, &'static str), Metric(&'static str) }
impl Choice {
    fn label(self) -> &'static str {
        match self { Self::Source(s) => s.label, Self::Preset(_, name, _) | Self::Metric(name) => name }
    }
    fn group(self) -> &'static str {
        match self { Self::Source(s) => s.group, Self::Preset(s, ..) => s.label, Self::Metric(_) => "" }
    }
}

pub(crate) struct Picker {
    habits: Entity<Habits>,
    step: Step,
    query: Entity<TextField>,
    title: Entity<TextField>,
    focus: FocusHandle,
    metric_focus: FocusHandle,
    metric_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    start_focus: FocusHandle,
    metric: &'static str,
    metrics_open: bool,
    active: usize,
    scroll: ScrollHandle,
    submission: Uuid,
    error: Option<String>,
    _subs: Vec<Subscription>,
}

impl Picker {
    pub(crate) fn new(habits: Entity<Habits>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = |hint, cx: &mut Context<Self>| cx.new(|cx| {
            let mut f = TextField::new(hint, cx);
            f.tab_events = true;
            f.size = 14.;
            f
        });
        let query = field("Search devices or habits…", cx);
        let title = field("Name", cx);
        let mut subs = Vec::new();
        for f in [&query, &title] {
            subs.push(cx.subscribe_in(f, window, |p, _, event, w, cx| p.key(*event, w, cx)));
        }
        query.read(cx).focus_handle(cx).focus(window);
        Self { habits, step: Step::Sources, query, title, focus: cx.focus_handle(),
            metric_focus: cx.focus_handle(), metric_bounds: Rc::default(), start_focus: cx.focus_handle(), metric: "Count", metrics_open: false,
            active: 0, scroll: ScrollHandle::new(), submission: Uuid::new_v4(), error: None, _subs: subs }
    }

    fn search(&mut self, hint: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.query.update(cx, |q, cx| { q.set_placeholder(hint, cx); q.set_text("", cx); });
        self.query.read(cx).focus_handle(cx).focus(window);
        self.active = 0;
        self.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }

    fn choices(&self, cx: &App) -> Vec<Choice> {
        let filter = self.query.read(cx).text.trim().to_lowercase();
        let mut rows = Vec::new();
        let matches = |label: &str| label.to_lowercase().contains(&filter);
        if self.metrics_open {
            if !METRICS.contains(&self.metric) && matches(self.metric) { rows.push(Choice::Metric(self.metric)); }
            rows.extend(METRICS.iter().filter(|m| matches(m)).map(|m| Choice::Metric(m)));
        } else {
            match self.step {
                Step::Sources => {
                    rows.extend(SOURCES.iter().filter(|s| matches(s.label)).map(Choice::Source));
                    if !filter.is_empty() {
                        for s in SOURCES.iter().filter(|s| !s.device()) {
                            rows.extend(s.presets.iter().filter(|(n, _)| matches(n)).map(|(n,u)| Choice::Preset(s,n,u)));
                        }
                    }
                }
                Step::Presets(s) => rows.extend(s.presets.iter().filter(|(n, _)| matches(n)).map(|(n,u)| Choice::Preset(s,n,u))),
                Step::Configure(..) => {}
            }
        }
        rows
    }

    fn configure(&mut self, s: &'static Source, preset: Option<&'static str>, metric: &'static str, w: &mut Window, cx: &mut Context<Self>) {
        self.step = Step::Configure(s, preset);
        self.metric = metric;
        self.title.update(cx, |f, cx| f.set_text(preset.unwrap_or(""), cx));
        self.error = None;
        self.submission = Uuid::new_v4();
        if preset.is_none() { self.title.read(cx).focus_handle(cx).focus(w); } else { self.metric_focus.focus(w); }
        cx.notify();
    }

    fn choose(&mut self, row: Choice, w: &mut Window, cx: &mut Context<Self>) {
        match row {
            Choice::Source(s) if s.label == "Custom" => self.configure(s, None, "Count", w, cx),
            Choice::Source(s) => { self.step = Step::Presets(s); self.search("Search habits…", w, cx); }
            Choice::Preset(s, name, unit) => self.configure(s, Some(name), unit, w, cx),
            Choice::Metric(unit) => { self.metric = unit; self.metrics_open = false; self.metric_focus.focus(w); cx.notify(); }
        }
    }

    fn close(&self, cx: &mut Context<Self>) {
        if !self.habits.read(cx).saving { cx.emit(PickerClosed { created: false }); }
    }

    fn back(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.habits.read(cx).saving { return; }
        if self.metrics_open { self.metrics_open = false; self.metric_focus.focus(w); cx.notify(); return; }
        if let Step::Configure(s, Some(_)) = self.step {
            self.step = Step::Presets(s); self.search("Search habits…", w, cx);
        } else { self.step = Step::Sources; self.search("Search devices or habits…", w, cx); }
    }

    fn open_metrics(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.metrics_open = true;
        self.search("Find a metric…", w, cx);
        let ix = self.choices(cx).iter().position(|r| r.label() == self.metric).unwrap_or(0);
        self.active = ix;
        self.scroll.scroll_to_item(ix);
    }

    fn key(&mut self, key: TextFieldEvent, w: &mut Window, cx: &mut Context<Self>) {
        use TextFieldEvent::*;
        if self.habits.read(cx).saving { return; }
        let palette = !matches!(self.step, Step::Configure(..)) || self.metrics_open;
        match key {
            Changed => { self.error = None; self.active = 0; self.scroll.set_offset(point(px(0.), px(0.))); cx.notify(); }
            Up | Down if palette => {
                let rows = self.choices(cx);
                if !rows.is_empty() {
                    self.active = if key == Down { (self.active+1).min(rows.len()-1) } else { self.active.saturating_sub(1) };
                    self.scroll.scroll_to_item(self.active); cx.notify();
                }
            }
            Submit if palette => { if let Some(&row) = self.choices(cx).get(self.active) { self.choose(row, w, cx); } }
            Submit if self.metric_focus.is_focused(w) => self.open_metrics(w, cx),
            Submit => self.save(w, cx),
            Cancel if self.metrics_open => self.back(w, cx),
            Cancel => self.close(cx),
            Tab | ShiftTab if !palette => {
                let mut chain = vec![];
                if matches!(self.step, Step::Configure(_, None)) { chain.push(self.title.read(cx).focus_handle(cx)); }
                chain.extend([self.metric_focus.clone(), self.start_focus.clone()]);
                let i = chain.iter().position(|f| f.is_focused(w)).unwrap_or(0);
                let inc = if key == Tab { 1 } else { chain.len()-1 };
                chain[(i+inc)%chain.len()].focus(w);
            }
            _ => {}
        }
    }

    fn save(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Step::Configure(source, _) = self.step else { return };
        let name = self.title.read(cx).text.trim().to_string();
        if name.is_empty() || self.habits.read(cx).saving { return; }
        let Some(db) = self.habits.read(cx).db.clone() else {
            self.error = Some("The habit library is not open. Close this dialog and retry on Home.".into()); cx.notify(); return;
        };
        let (id, category, metric) = (self.submission, source.category(), self.metric);
        self.habits.update(cx, |h, cx| { h.saving = true; cx.notify(); });
        self.focus.focus(w);
        let task = cx.background_executor().spawn(async move { db.lock().await.create_habit(id, &name, category, metric) });
        cx.spawn_in(w, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |p, w, cx| {
                let saved = result.is_ok();
                let quit = p.habits.update(cx, |h, cx| {
                    h.saving = false;
                    if let Ok(habit) = &result {
                        h.habits.push(TrackedHabit { habit: habit.clone(), total: 0., count: 0 });
                        h.show_list(cx);
                    }
                    cx.notify();
                    std::mem::take(&mut h.quit_after_save) && saved
                });
                match result {
                    Ok(_) => cx.emit(PickerClosed { created: true }),
                    Err(err) => { p.error = Some(format!("{err:#}")); p.start_focus.focus(w); cx.notify(); }
                }
                if quit { cx.defer(crate::app::quit); }
            });
        }).detach();
        cx.notify();
    }
}

fn icon_button(id: &'static str, icon: &'static str) -> Stateful<Div> {
    ui::icon_button(id, ui::icon(icon, RITUAL.menu_icon, Theme::text_caption()))
}

impl Picker {
    fn palette(&self, cx: &mut Context<Self>) -> Div {
        let choices = self.choices(cx);
        let mut list = div().id("habit-options").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.scroll).px(px(8.)).pb(px(8.));
        let mut group = "";
        for (i, &choice) in choices.iter().enumerate() {
            let section = choice.group();
            let header = group != section && matches!(self.step, Step::Sources);
            group = section;
            let active = i == self.active;
            let icon = match choice {
                Choice::Source(s) if s.icon.ends_with(".svg") => svg().path(format!("ritual/providers/{}", s.icon)).size(px(RITUAL.menu_icon)).text_color(Theme::icon()).into_any_element(),
                Choice::Source(s) => ui::icon(s.icon, RITUAL.menu_icon, Theme::icon()).into_any_element(),
                _ => ui::icon("list-todo", RITUAL.menu_icon, Theme::icon()).into_any_element(),
            };
            let row = ui::option_row(("habit-option", i)).h(px(RITUAL.menu_height + 4.)).text_size(px(14.))
                .when(active, |d| d.bg(Theme::fill_hover_strong()))
                .on_hover(cx.listener(move |p, hovering: &bool, _, cx| { if *hovering { p.active = i; cx.notify(); } }))
                .child(icon).child(choice.label()).child(div().flex_1())
                .when_some(match choice { Choice::Source(s) if s.device() => Some("Manual only"), _ if active => Some("↵"), _ => None },
                    |d, hint| d.child(div().text_size(px(12.)).text_color(Theme::text_placeholder()).child(hint)))
                .on_click(cx.listener(move |p, _, w, cx| p.choose(choice, w, cx)));
            list = list.child(div().when(header, |d| d.child(div().px(px(10.)).pt(px(10.)).pb(px(4.)).text_size(px(11.))
                .font_weight(FontWeight::MEDIUM).text_color(Theme::text_placeholder()).child(section))).child(row));
        }
        if choices.is_empty() { list = list.child(div().p(px(16.)).text_color(Theme::text_caption()).child("No matches.")); }
        div().flex_1().min_h_0().flex().flex_col().child(list)
    }

    fn config(&self, custom: bool, w: &Window, cx: &mut Context<Self>) -> Div {
        let saving = self.habits.read(cx).saving;
        let control = || div().flex_1().min_w_0().h(px(RITUAL.control_height)).max_w(px(260.)).px(px(10.)).flex().items_center()
            .rounded(px(RITUAL.control_radius)).border_1().border_color(Theme::line_border()).bg(Theme::bg_body());
        let row = |label: &'static str, content: AnyElement| div().px(px(13.)).py(px(4.)).flex().items_center().justify_between()
            .border_b_1().border_color(ui::list_line()).child(div().mr(px(8.)).text_color(Theme::text_caption()).child(label)).child(content);
        let name = self.title.read(cx).text.clone();
        let title = self.title.read(cx).focus_handle(cx);
        let enabled = !name.trim().is_empty() && !saving;
        let (y,m,d,_,_) = crate::time::civil(crate::time::now());
        let month = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][(m-1) as usize];
        let metric_bounds = self.metric_bounds.clone();
        let fields = div().rounded(px(RITUAL.menu_radius)).overflow_hidden().bg(Theme::fill_hover()).child(row("Title", control().id("habit-title")
            .when(custom && !saving, |el| el.cursor(CursorStyle::IBeam)
                .on_click(move |_, w, _| title.focus(w)).child(self.title.clone()))
            .when(!custom || saving, |el| el.child(name)).into_any_element()))
            .child(row("Metric", control().id("habit-metric").relative().track_focus(&self.metric_focus).cursor_pointer()
                .child(canvas(move |b, _, _| metric_bounds.set(Some(b)), |_, _, _, _| {}).absolute().inset_0())
                .when(self.metric_focus.is_focused(w), |d| d.border_color(Theme::text_placeholder()))
                .on_click(cx.listener(|p, _, w, cx| { if !p.habits.read(cx).saving { p.open_metrics(w,cx); } }))
                .child(self.metric).child(div().flex_1()).child(ui::icon("chevron-down", 14., Theme::text_caption())).into_any_element()))
            .child(row("Start date", control().bg(Theme::fill_hover()).text_color(Theme::text_caption()).gap(px(7.)).child(svg().path("editor-icons/calendar.svg").size(px(14.)).text_color(Theme::text_caption())).child(format!("Today, {month} {d}, {y}")).into_any_element()));
        div().flex().flex_col().p(px(16.)).pt(px(4.)).child(div().text_size(px(19.)).font_weight(FontWeight::SEMIBOLD).child("Configure"))
            .child(div().mt(px(5.)).mb(px(14.)).text_color(Theme::text_caption()).child("Choose how this habit should be tracked."))
            .child(fields)
            .when_some(self.error.clone(), |d,e| d.child(div().mt(px(6.)).text_color(Theme::danger()).child(e)))
            .child(div().mt(px(18.)).flex().justify_end().items_center().gap(px(6.))
                .child(div().id("cancel-config").h(px(RITUAL.control_height)).px(px(12.)).rounded(px(RITUAL.control_radius)).hover(|s| s.bg(Theme::fill_hover())).flex().items_center().cursor_pointer().text_color(Theme::text_caption())
                    .child("Cancel").on_click(cx.listener(|p, _, w, cx| p.back(w,cx))))
                .child(div().id("start-tracking").track_focus(&self.start_focus).h(px(RITUAL.control_height)).px(px(16.)).flex().items_center().rounded(px(RITUAL.control_radius))
                    .bg(Theme::text()).text_color(Theme::bg_body()).when(!enabled, |d| d.opacity(0.40))
                    .when(self.start_focus.is_focused(w), |d| d.border_2().border_color(Theme::primary()))
                    .when(enabled, |d| d.cursor_pointer().on_click(cx.listener(|p, _, w, cx| p.save(w,cx))))
                    .child(if saving { "Starting…" } else { "Start Tracking" })))
    }

    fn metrics(&self, w: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let b = self.metric_bounds.get()?;
        let mut options = div().id("metric-options").min_h_0().flex_1().overflow_y_scroll().track_scroll(&self.scroll);
        let rows = self.choices(cx);
        for (i, choice) in rows.iter().copied().enumerate() {
            options = options.child(ui::option_row(("metric", i))
                .when(self.active == i, |d| d.bg(Theme::fill_hover_strong()))
                .child(choice.label()).child(div().flex_1()).when(choice.label()==self.metric, |d| d.child(ui::icon("check",13.,Theme::icon())))
                .on_click(cx.listener(move |p, _, w, cx| p.choose(choice,w,cx))));
        }
        if rows.is_empty() { options = options.child(div().p(px(8.)).text_color(Theme::text_caption()).child("No matches.")); }
        let height = px(180.);
        let above = b.bottom() + height + px(12.) > w.viewport_size().height && b.top() > height + px(12.);
        let (corner, y) = if above { (Corner::BottomRight, b.top() - px(4.)) } else { (Corner::TopRight, b.bottom() + px(4.)) };
        let menu = ui::popover().occlude().w(b.size.width).h(height).p(px(6.)).flex().flex_col()
            .child(div().h(px(RITUAL.small_control_height)).flex_none().px(px(8.)).flex().items_center().border_b_1().border_color(ui::list_line()).child(self.query.clone())).child(options);
        Some(deferred(anchored().anchor(corner).position(point(b.right(), y)).snap_to_window_with_margin(px(8.)).child(menu)).with_priority(4).into_any_element())
    }
}

impl Render for Picker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = matches!(self.step, Step::Configure(..));
        let bar = div().h(px(50.)).px(px(12.)).flex_none().flex().gap(px(5.)).items_center()
            .when(!config, |d| d.border_b_1().border_color(ui::list_line()))
            .when(!matches!(self.step,Step::Sources), |d| d.child(icon_button("habit-back","arrow-left").on_click(cx.listener(|p,_,w,cx| p.back(w,cx)))))
            .when(!config, |d| d.child(div().flex_1().min_w_0().flex().child(self.query.clone())))
            .when(config, |d| d.child(div().flex_1()))
            .child(icon_button("habit-close","x").on_click(cx.listener(|p,_,_,cx| p.close(cx))));
        ui::dialog("habit-selection-modal").relative().w(px(540.)).max_w(relative(0.96))
            .h(px((if config { 440.0_f32 } else { 510.0_f32 }).min(f32::from(w.viewport_size().height)-32.)))
            .mx(px(12.)).border_1().border_color(Theme::line_border()).overflow_hidden().flex().flex_col().text_size(px(14.))
            .track_focus(&self.focus).key_context("HabitPicker").child(bar)
            .on_mouse_down(MouseButton::Left, |_,_,cx| cx.stop_propagation())
            .on_action(cx.listener(|p,_: &input::Cancel,w,cx| p.key(TextFieldEvent::Cancel,w,cx)))
            .on_action(cx.listener(|p,_: &input::Submit,w,cx| p.key(TextFieldEvent::Submit,w,cx)))
            .on_action(cx.listener(|p,_: &input::Tab,w,cx| p.key(TextFieldEvent::Tab,w,cx)))
            .on_action(cx.listener(|p,_: &input::ShiftTab,w,cx| p.key(TextFieldEvent::ShiftTab,w,cx)))
            .when(!config && self.query.read(cx).text.is_empty(), |d| d.child(div().px(px(16.)).pt(px(7.)).pb(px(4.))
                .text_size(px(13.)).text_color(Theme::text_caption()).child(match self.step {
                    Step::Presets(s) if s.device() => "Choose a measurement to track manually. Device sync is unavailable in Ritual Dev.",
                    Step::Presets(_) => "Choose a habit, then configure its metric.",
                    _ => "Create a custom habit or choose a preset. You can log each measurement by hand."
                })))
            .when(!config, |d| d.child(self.palette(cx)))
            .when(config, |d| d.child(self.config(matches!(self.step,Step::Configure(_,None)),w,cx)))
            .when(self.metrics_open, |d| d.child(div().id("metrics-backdrop").absolute().inset_0().occlude()
                .on_click(cx.listener(|p,_,w,cx| p.back(w,cx)))).children(self.metrics(w, cx)))
    }
}
