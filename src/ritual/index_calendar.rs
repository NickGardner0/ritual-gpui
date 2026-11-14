//! Production Index presets and an inclusive custom range on local calendar days.
use super::{appearance, index::option};
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

pub(super) struct Calendar { month: NaiveDate, start: Option<NaiveDate>, press: Option<NaiveDate>, hover: Option<NaiveDate>, presets: bool }
impl Default for Calendar { fn default() -> Self { Self::new(Period::All) } }
impl Calendar {
    pub(super) fn new(period: Period) -> Self {
        let today = Local::now().date_naive();
        let day = period.range(today).map_or(today, |r| *r.start());
        Self { month: day.with_day(1).unwrap(), start: None, press: None, hover: None, presets: false }
    }
}

impl Workspace {
    pub(super) fn index_dates(&self, period: Period, available: f32, cx: &mut Context<Self>) -> Div {
        let cal = &self.navigation.as_ref().unwrap().index.calendar;
        let today = Local::now().date_naive();
        let months = if available >= 570. { 2 } else { 1 };
        let width = (months as f32 * 274. + 16.).min(available);
        let mut panel = div().w(px(width)).font_family(appearance::navigation_font(cx)).flex().flex_col();
        let presets = option("range-presets").w(px(160.)).border_1().border_color(Theme::index_border())
            .child(period.label()).child(div().flex_1()).child(ui::icon("chevron-down", 12., Theme::index_secondary()))
            .on_click(cx.listener(|w, _, _, cx| { let cal = &mut w.navigation.as_mut().unwrap().index.calendar; cal.presets = !cal.presets; cx.notify(); }));
        panel = panel.child(div().p(px(10.)).border_b_1().border_color(Theme::index_border()).child(presets));
        if cal.presets {
            return panel.child(div().p(px(5.)).children(Period::PRESETS.map(|p| option(SharedString::from(p.label())).child(p.label())
                .child(div().flex_1()).when(period == p, |d| d.child(ui::icon("check", 13., Theme::index_secondary())))
                .on_click(cx.listener(move |w, _, win, cx| w.choose_period(p, win, cx))))));
        }
        let range = cal.start.or(cal.press).map(|d| {
            let end = cal.hover.unwrap_or(d); d.min(end)..=d.max(end)
        }).or_else(|| period.range(today));
        let mut calendars = div().w_full().p(px(8.)).flex();
        for i in 0..months {
            let first = cal.month + Months::new(i);
            let arrow = |next: bool, cx: &mut Context<Self>| option(if next { "next-month" } else { "previous-month" })
                .w(px(28.)).px(px(0.)).justify_center().child(ui::icon(if next { "chevron-right" } else { "arrow-left" }, 13., Theme::index_secondary()))
                .on_click(cx.listener(move |w, _, _, cx| {
                    let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
                    cal.month = if next { cal.month + Months::new(1) } else { cal.month - Months::new(1) }; cx.notify();
                }));
            let title = div().flex().items_center().h(px(34.))
                .child(div().w(px(28.)).when(i == 0, |d| d.child(arrow(false, cx))))
                .child(div().flex_1().text_center().text_size(px(14.)).font_weight(FontWeight::MEDIUM).child(first.format("%B %Y").to_string()))
                .child(div().w(px(28.)).when(i + 1 == months, |d| d.child(arrow(true, cx))));
            let mut month = div().flex_1().min_w_0().px(px(4.)).child(title)
                .child(div().flex().children(["Su","Mo","Tu","We","Th","Fr","Sa"].map(|day| div().flex_1().text_center()
                    .text_size(px(11.)).text_color(Theme::index_secondary()).child(day))));
            let grid_start = first - Days::new(first.weekday().num_days_from_sunday().into());
            for week in 0..6 {
                month = month.child(div().flex().children((0..7).map(|day| {
                    let date = grid_start + Days::new(week * 7 + day);
                    let in_month = date.month() == first.month();
                    div().flex_1().min_w_0().h(px(32.)).when(in_month, |d| d.child(option(SharedString::from(date.to_string()))
                        .size_full().px(px(0.)).justify_center().rounded(px(5.))
                        .when(range.as_ref().is_some_and(|r| r.contains(&date)), |d| d.bg(Theme::fill_hover_strong()))
                        .when(date == today, |d| d.border_1().border_color(Theme::index_border()))
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
                            if let Some(start) = cal.start { w.choose_period(Period::Custom(start.min(date), start.max(date)), win, cx); }
                            else { cal.start = Some(date); cx.notify(); }
                        }))))
                })));
            }
            calendars = calendars.child(month);
        }
        panel.on_mouse_up_out(MouseButton::Left, cx.listener(|w, _, _, cx| {
            let cal = &mut w.navigation.as_mut().unwrap().index.calendar;
            if cal.press.take().is_some() { cal.hover = None; cx.notify(); }
        })).child(calendars).child(div().px(px(14.)).pb(px(10.)).text_size(px(12.)).text_color(Theme::index_secondary())
            .child(if cal.start.is_some() { "Choose the last day." } else { "Choose a preset or select a start and end date." }))
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
