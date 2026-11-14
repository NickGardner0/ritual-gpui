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
    presets: bool, trigger: Option<FocusHandle>,
}
impl Default for Calendar { fn default() -> Self { Self::new(Period::All) } }
impl Calendar {
    pub(super) fn new(period: Period) -> Self {
        let today = Local::now().date_naive();
        let day = period.range(today).map_or(today, |r| *r.start());
        Self { month: day.with_day(1).unwrap(), start: None, press: None, hover: None, presets: false, trigger: None }
    }
    pub(super) fn presets_open(&self) -> bool { self.presets }
    fn range(&self, period: Period, today: NaiveDate) -> Option<RangeInclusive<NaiveDate>> {
        let Some(from) = self.start.or(self.press) else { return period.range(today) };
        let to = self.hover.unwrap_or(from);
        Some(from.min(to)..=from.max(to))
    }
}

impl Workspace {
    pub(super) fn toggle_date_presets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cal = &mut self.navigation.as_mut().unwrap().index.calendar;
        cal.presets = !cal.presets;
        let handle = cal.trigger.get_or_insert_with(|| cx.focus_handle());
        handle.focus(window);
        cx.notify();
    }

    fn period_presets(&self, selected: Period, max_h: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let muted = Theme::index_secondary();
        let custom = matches!(selected, Period::Custom(..));
        let mark = |checked| div().w(px(16.)).flex_none().when(checked, |d| d.child(ui::icon("check", 14., Theme::navigation_ink(true))));
        let row = || div().h(px(32.)).flex_none().px(px(8.)).flex().gap(px(8.)).items_center().rounded(px(8.));
        ui::popover().id("index-period-presets").occlude().absolute().left(px(16.)).top(px(52.))
            .w(px(200.)).max_h(px(max_h)).overflow_y_scroll().rounded(px(13.)).p(px(5.))
            .bg(Theme::calendar_surface()).border_color(Theme::index_border())
            .font_family(appearance::navigation_font(cx)).text_size(px(14.)).text_color(Theme::navigation_ink(true))
            .child(row().text_color(muted).child(mark(custom)).child("Custom range"))
            .children(Period::PRESETS.map(|p| row().id(SharedString::from(p.label())).tab_index(0)
                .cursor_pointer().hover(|s| s.bg(Theme::calendar_hover())).focus(|s| s.bg(Theme::calendar_hover()))
                .child(mark(p == selected)).child(p.label())
                .on_click(cx.listener(move |w, _, win, cx| w.choose_period(p, win, cx)))))
    }

    fn calendar_arrow(&self, next: bool, focusable: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        div().id(if next { "next-month" } else { "previous-month" }).size(px(28.)).rounded_full().flex().items_center().justify_center()
            .when(focusable, |d| d.tab_index(0)).cursor_pointer().hover(|s| s.bg(Theme::calendar_hover())).focus(|s| s.bg(Theme::calendar_hover()))
            .child(svg().path(if next { "editor-icons/chevron-right.svg" } else { "editor-icons/chevron-left.svg" }).size(px(16.)).text_color(Theme::index_secondary()))
            .on_click(cx.listener(move |w, _, _, cx| {
                let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                cal.month = if next { cal.month + Months::new(1) } else { cal.month - Months::new(1) }; cx.notify();
            }))
    }

    fn calendar_day(&self, date: NaiveDate, first: NaiveDate, range: Option<&RangeInclusive<NaiveDate>>, today: NaiveDate, cx: &mut Context<Self>) -> Stateful<Div> {
        let active = range.is_some_and(|r| r.contains(&date));
        let outside = date.month() != first.month();
        let keyboard = !self.navigation.as_ref().unwrap().index.calendar.presets;
        div().id((SharedString::from(first.to_string()), date.num_days_from_ce() as usize)).size(px(36.)).max_w_full()
            .rounded_full().flex().items_center().justify_center().cursor_pointer().when(keyboard, |d| d.tab_index(0))
            .text_size(px(14.)).font_weight(FontWeight::NORMAL)
            .text_color(if outside { Theme::index_secondary() } else { Theme::navigation_ink(true) })
            .when(date == today, |d| d.bg(Theme::calendar_hover()))
            .when(active, |d| d.bg(Theme::calendar_range()))
            .hover(move |s| s.bg(if active { Theme::calendar_range() } else { Theme::calendar_hover() }))
            .focus(|s| s.bg(Theme::calendar_range()))
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
                else { cal.start = Some(date); cal.hover = Some(date); cx.notify(); }
            }))
    }

    pub(super) fn index_dates(&self, period: Period, available: f32, height: f32, cx: &mut Context<Self>) -> Div {
        let cal = &self.navigation.as_ref().unwrap().index.calendar;
        let today = Local::now().date_naive();
        let months = if available >= 580. { 2 } else { 1 };
        let width = if months == 2 { 580_f32 } else { 284. }.min(available);
        let preset_label = if matches!(period, Period::Custom(..)) { "Custom range".to_string() } else { period.label() };
        let trigger = div().id("range-presets").w(px(200.)).max_w_full().h(px(36.)).px(px(12.)).flex().items_center().justify_between()
            .tab_index(0).when_some(cal.trigger.as_ref(), |d, h| d.track_focus(h))
            .cursor_pointer().border_1().border_color(Theme::button_border()).rounded(px(3.)).text_size(px(14.))
            .hover(|s| s.bg(Theme::calendar_hover())).focus(|s| s.border_color(Theme::index_secondary()))
            .child(preset_label).child(ui::icon("chevron-down", 15., Theme::index_secondary()))
            .on_click(cx.listener(|w, _, win, cx| w.toggle_date_presets(win, cx)));
        let range = cal.range(period, today);
        let mut calendars = div().w_full().flex().items_start().gap(px(16.)).p(px(16.));
        for i in 0..months {
            let first = cal.month + Months::new(i);
            let title = div().w_full().h(px(28.)).flex().items_center()
                .child(div().w(px(28.)).when(i == 0, |d| d.child(self.calendar_arrow(false, !cal.presets, cx))))
                .child(div().flex_1().text_center().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(first.format("%B %Y").to_string()))
                .child(div().w(px(28.)).when(i + 1 == months, |d| d.child(self.calendar_arrow(true, !cal.presets, cx))));
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
