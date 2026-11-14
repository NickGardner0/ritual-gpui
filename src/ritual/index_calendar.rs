//! Production Index presets and an inclusive custom range on local calendar days.
use super::appearance;
use crate::{app::Workspace, theme::Theme, ui};
use chrono::{Datelike, Days, Local, Months, NaiveDate};
use gpui::{prelude::FluentBuilder, *};
use std::ops::RangeInclusive;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum Period {
    #[default] All, Today, Yesterday, LastWeek, Fortnight, LastThirty, Week, Month, Quarter, Year, Custom(NaiveDate, NaiveDate),
}
impl Period {
    const PRESETS: [Self; 10] = [Self::Today, Self::Yesterday, Self::LastWeek, Self::Fortnight, Self::LastThirty,
        Self::Week, Self::Month, Self::Quarter, Self::Year, Self::All];
    pub(super) fn label(self) -> String {
        match self {
            Self::All => "All time", Self::Today => "Today", Self::Yesterday => "Yesterday", Self::LastWeek => "Last 7 days",
            Self::Fortnight => "Last 14 days", Self::LastThirty => "Last 30 days", Self::Week => "This week", Self::Month => "This month",
            Self::Quarter => "Last 3 months", Self::Year => "Last 12 months",
            Self::Custom(a,b) if a == b => return a.format("%b %-d").to_string(),
            Self::Custom(a,b) => return format!("{} – {}", a.format("%b %-d"), b.format("%b %-d")),
        }.into()
    }
    pub(super) fn range(self, today: NaiveDate) -> Option<RangeInclusive<NaiveDate>> {
        let start = match self {
            Self::All => return None, Self::Custom(a,b) => return Some(a..=b),
            Self::Today => today, Self::Yesterday => { let day = today - Days::new(1); return Some(day..=day); }
            Self::LastWeek => today - Days::new(6), Self::Fortnight => today - Days::new(13), Self::LastThirty => today - Days::new(29),
            Self::Week => { let from = today - Days::new(today.weekday().num_days_from_sunday().into()); return Some(from..=from + Days::new(6)); }
            Self::Month => { let from = today.with_day(1)?; return Some(from..=(from + Months::new(1) - Days::new(1))); }
            Self::Quarter => today - Months::new(3), Self::Year => today - Months::new(12),
        };
        Some(start..=today)
    }
}

pub(super) struct Calendar {
    month: NaiveDate, start: Option<NaiveDate>, press: Option<NaiveDate>, hover: Option<NaiveDate>,
    cursor: NaiveDate, presets: bool,
    controls: ui::MenuFocus<4>, options: ui::MenuFocus<10>, preset_scroll: ScrollHandle,
}
const PRESET: usize = 0;
const PREVIOUS_MONTH: usize = 1;
const NEXT_MONTH: usize = 2;
const DAY: usize = 3;

fn month_count(available: f32) -> u32 { if available >= 580. { 2 } else { 1 } }

impl Calendar {
    pub(super) fn new(cx: &App) -> Self {
        let today = Local::now().date_naive();
        Self { month: today.with_day(1).unwrap(), cursor: today, start: None, press: None, hover: None, presets: false,
            controls: ui::MenuFocus::new(cx), options: ui::MenuFocus::new(cx), preset_scroll: ScrollHandle::new() }
    }
    pub(super) fn open(&mut self, period: Period, window: &mut Window) {
        let today = Local::now().date_naive();
        self.cursor = period.range(today).map_or(today, |r| *r.start());
        self.month = self.cursor.with_day(1).unwrap();
        self.start = None; self.press = None; self.hover = None; self.presets = false;
        self.controls.items[PRESET].focus(window);
    }
    fn first_month(&self, count: u32) -> NaiveDate {
        let first = self.cursor.with_day(1).unwrap();
        if first < self.month { first } else { self.month.max(first - Months::new(count - 1)) }
    }
    fn move_to(&mut self, day: NaiveDate, count: u32) {
        self.month = self.first_month(count);
        self.cursor = day;
        self.month = self.first_month(count);
        if self.start.is_some() { self.hover = Some(day); }
    }
    fn range(&self, period: Period, today: NaiveDate) -> Option<RangeInclusive<NaiveDate>> {
        let Some(from) = self.start.or(self.press) else { return period.range(today) };
        let to = self.hover.unwrap_or(from);
        Some(from.min(to)..=from.max(to))
    }
}

impl Workspace {
    pub(super) fn toggle_date_presets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.habits.as_ref().unwrap().read(cx).period;
        let cal = &mut self.navigation.as_mut().unwrap().index.calendar;
        cal.presets = !cal.presets;
        if cal.presets {
            let i = Period::PRESETS.iter().position(|p| *p == selected).unwrap_or(0);
            cal.options.items[i].focus(window);
            cal.preset_scroll.scroll_to_item(i + 1);
        } else { cal.controls.items[PRESET].focus(window); }
        cx.notify();
    }

    pub(super) fn date_picker_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let cal = &mut self.navigation.as_mut().unwrap().index.calendar;
        let Some(command) = ui::plain_key(key) else { return false };
        if cal.presets {
            if command == "escape" { self.toggle_date_presets(window, cx); return true; }
            if let Some(i) = cal.options.navigate(key, window) { cal.preset_scroll.scroll_to_item(i + 1); return true; }
            return false;
        }
        if matches!(command, "tab" | "shift-tab") { return cal.controls.navigate(key, window).is_some(); }
        if cal.controls.items[PRESET].is_focused(window) && matches!(command, "down" | "up") {
            self.toggle_date_presets(window, cx); return true;
        }
        if !cal.controls.items[DAY].is_focused(window) { return false; }
        let day = cal.cursor;
        let weekday = u64::from(day.weekday().num_days_from_sunday());
        let next = match command {
            "left" => day - Days::new(1), "right" => day + Days::new(1),
            "up" => day - Days::new(7), "down" => day + Days::new(7),
            "home" => day - Days::new(weekday), "end" => day + Days::new(6 - weekday),
            "pageup" => day - Months::new(1), "pagedown" => day + Months::new(1),
            _ => return false,
        };
        cal.move_to(next, month_count(f32::from(window.viewport_size().width) - 16.));
        cx.notify(); true
    }

    fn period_presets(&self, selected: Period, max_h: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let muted = Theme::index_secondary();
        let cal = &self.navigation.as_ref().unwrap().index.calendar;
        let custom = matches!(selected, Period::Custom(..));
        let mark = |checked| div().w(px(16.)).flex_none().when(checked, |d| d.child(ui::icon("check", 14., Theme::navigation_ink(true))));
        let row = || div().h(px(32.)).flex_none().px(px(8.)).flex().gap(px(8.)).items_center().rounded(px(8.));
        ui::popover().id("index-period-presets").occlude().absolute().left(px(16.)).top(px(52.))
            .w(px(200.)).max_h(px(max_h)).overflow_y_scroll().track_scroll(&cal.preset_scroll).rounded(px(13.)).p(px(5.))
            .bg(Theme::calendar_surface()).border_color(Theme::index_border())
            .font_family(appearance::navigation_font(cx)).text_size(px(14.)).text_color(Theme::navigation_ink(true))
            .child(row().text_color(muted).child(mark(custom)).child("Custom range"))
            .children(Period::PRESETS.iter().copied().enumerate().map(|(i, p)|
                ui::menu_choice(SharedString::from(p.label()), &cal.options.items[i], Theme::calendar_hover())
                .h(px(32.)).px(px(8.)).gap(px(8.)).rounded(px(8.))
                .child(mark(p == selected)).child(p.label())
                .on_click(cx.listener(move |w, _, win, cx| w.choose_period(p, win, cx)))))
    }

    fn calendar_arrow(&self, next: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let cal = &self.navigation.as_ref().unwrap().index.calendar;
        let button = ui::button_surface(if next { "next-month" } else { "previous-month" }, true, Theme::calendar_hover());
        ui::button_focus(button, &cal.controls.items[if next { NEXT_MONTH } else { PREVIOUS_MONTH }], Theme::calendar_hover())
            .size(px(28.)).rounded_full().flex().items_center().justify_center()
            .child(svg().path(if next { "editor-icons/chevron-right.svg" } else { "editor-icons/chevron-left.svg" }).size(px(16.)).text_color(Theme::index_secondary()))
            .on_click(cx.listener(move |w, _, win, cx| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                let months = month_count(f32::from(win.viewport_size().width) - 16.);
                let shift = |date: NaiveDate| if next { date + Months::new(1) } else { date - Months::new(1) };
                cal.month = shift(cal.first_month(months)); cal.cursor = shift(cal.cursor);
                if cal.start.is_some() { cal.hover = Some(cal.cursor); }
                cx.notify();
            }))
    }

    fn calendar_day(&self, date: NaiveDate, first: NaiveDate, range: Option<&RangeInclusive<NaiveDate>>, today: NaiveDate, cx: &mut Context<Self>) -> Stateful<Div> {
        let active = range.is_some_and(|r| r.contains(&date));
        let outside = date.month() != first.month();
        let cal = &self.navigation.as_ref().unwrap().index.calendar;
        let hover = if active { Theme::calendar_range() } else { Theme::calendar_hover() };
        let id = (SharedString::from(first.to_string()), date.num_days_from_ce() as usize);
        ui::button_surface(id, true, hover).size(px(36.)).max_w_full().rounded_full().border_1().border_color(transparent_black())
            .flex().items_center().justify_center().text_size(px(14.)).font_weight(FontWeight::NORMAL)
            .text_color(if outside { Theme::index_secondary() } else { Theme::navigation_ink(true) })
            .when(date == today, |d| d.bg(Theme::calendar_hover()))
            .when(active, |d| d.bg(Theme::calendar_range()))
            .when(!outside && date == cal.cursor, |d| ui::button_focus(d, &cal.controls.items[DAY], hover))
            .child(date.day().to_string())
            .on_mouse_down(MouseButton::Left, cx.listener(move |w, _, _, _| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar; cal.press = Some(date); cal.hover = Some(date);
            }))
            .on_hover(cx.listener(move |w, over: &bool, _, cx| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                if *over && (cal.start.is_some() || cal.press.is_some()) { cal.hover = Some(date); cx.notify(); }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(move |w, _, win, cx| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                if let Some(from) = cal.press.take() && from != date {
                    w.choose_period(Period::Custom(from.min(date), from.max(date)), win, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |w, _, win, cx| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                if let Some(from) = cal.start { w.choose_period(Period::Custom(from.min(date), from.max(date)), win, cx); }
                else {
                    cal.start = Some(date);
                    cal.move_to(date, month_count(f32::from(win.viewport_size().width) - 16.));
                    cal.controls.items[DAY].focus(win); cx.notify();
                }
            }))
    }

    pub(super) fn index_dates(&self, period: Period, available: f32, height: f32, cx: &mut Context<Self>) -> Div {
        let cal = &self.navigation.as_ref().unwrap().index.calendar;
        let today = Local::now().date_naive();
        let months = month_count(available);
        let width = if months == 2 { 580_f32 } else { 284. }.min(available);
        let preset_label = if matches!(period, Period::Custom(..)) { "Custom range".to_string() } else { period.label() };
        let trigger = ui::button_focus(ui::button_surface("range-presets", true, Theme::calendar_hover()), &cal.controls.items[PRESET], Theme::calendar_hover())
            .w(px(200.)).max_w_full().h(px(36.)).px(px(12.)).flex().items_center().justify_between()
            .border_1().border_color(Theme::button_border()).rounded(px(3.)).text_size(px(14.))
            .child(preset_label).child(ui::icon("chevron-down", 15., Theme::index_secondary()))
            .on_click(cx.listener(|w, _, win, cx| w.toggle_date_presets(win, cx)));
        let range = cal.range(period, today);
        let mut calendars = div().w_full().flex().items_start().gap(px(16.)).p(px(16.));
        for i in 0..months {
            let first = cal.first_month(months) + Months::new(i);
            let title = div().w_full().h(px(28.)).flex().items_center()
                .child(div().w(px(28.)).when(i == 0, |d| d.child(self.calendar_arrow(false, cx))))
                .child(div().flex_1().text_center().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(first.format("%B %Y").to_string()))
                .child(div().w(px(28.)).when(i + 1 == months, |d| d.child(self.calendar_arrow(true, cx))));
            let mut month = div().w(px(252.)).min_w_0().flex().flex_col().child(title)
                .child(div().mt(px(12.)).h(px(24.)).flex().items_center().children(["Su","Mo","Tu","We","Th","Fr","Sa"].map(|day| div().flex_1().text_center()
                    .text_size(px(13.)).text_color(Theme::index_secondary()).child(day))));
            let grid_start = first - Days::new(first.weekday().num_days_from_sunday().into());
            for week in 0..6 {
                month = month.child(div().mt(px(8.)).flex().children((0..7).map(|day| {
                    let date = grid_start + Days::new(week * 7 + day);
                    div().flex_1().min_w_0().flex().justify_center().child(self.calendar_day(date, first, range.as_ref(), today, cx))
                })));
            }
            calendars = calendars.child(month);
        }
        div().w(px(width)).relative().font_family(appearance::navigation_font(cx)).text_color(Theme::navigation_ink(true))
            .child(div().px(px(16.)).py(px(12.)).border_b_1().border_color(Theme::index_border()).child(trigger))
            .child(calendars).when(cal.presets, |d| d.child(div().id("dismiss-presets").occlude().absolute().inset_0().top(px(60.))
                .on_mouse_down(MouseButton::Left, cx.listener(|w, _, win, cx| { w.toggle_date_presets(win, cx); cx.stop_propagation(); })))
                .child(self.period_presets(period, (height-118.).clamp(80., 366.), cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|w, _, _, cx| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                if cal.press.take().is_some() { cal.hover = None; cx.notify(); }
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Period, NaiveDate};
    #[test]
    fn calendar_presets_include_both_ends_and_respect_month_boundaries() {
        let date = |s: &str| s.parse::<NaiveDate>().unwrap();
        let day = date("2024-03-01");
        for (preset, from, to) in [
            (Period::Yesterday, "2024-02-29", "2024-02-29"), (Period::LastWeek, "2024-02-24", "2024-03-01"),
            (Period::Week, "2024-02-25", "2024-03-02"), (Period::Month, "2024-03-01", "2024-03-31"),
        ] { assert_eq!(preset.range(day), Some(date(from)..=date(to))); }
        assert_eq!(Period::All.range(day), None);
    }
}
