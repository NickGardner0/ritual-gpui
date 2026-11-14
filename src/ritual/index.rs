//! The Index has its own title bar. The document library keeps its Home toolbar.
use super::{Habits, appearance, index_calendar::{Calendar, Period}};
use crate::{app::Workspace, home::HomeTab, theme::Theme, trash::page_header_with, ui::{self, TitleBarExt}};
use gpui::{prelude::FluentBuilder, *};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Menu { Actions, Dates }
impl Menu {
    fn id(self) -> &'static str { match self { Self::Actions => "index-actions", Self::Dates => "index-dates" } }
}

#[derive(Default)]
pub(super) struct IndexMenuState {
    open: Option<Menu>,
    focus: Option<FocusHandle>,
    pub(super) calendar: Calendar,
}

fn pill(id: &'static str, open: bool) -> Stateful<Div> {
    div().id(id).relative().no_drag().tab_index(0).h(px(28.)).px(px(10.)).flex_none().flex().items_center().gap(px(6.))
        .rounded_full().border_1().border_color(Theme::index_border()).cursor_pointer()
        .text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(Theme::index_secondary())
        .when(open, |d| d.bg(Theme::navigation_highlight()))
        .hover(|s| s.bg(Theme::navigation_highlight()).text_color(Theme::navigation_ink(true)))
        .focus(|s| s.border_color(Theme::primary()))
}

pub(super) fn option(id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).tab_index(0).flex_none().flex().items_center().cursor_pointer().text_size(px(13.)).h(px(30.)).rounded(px(6.)).px(px(9.)).gap(px(8.))
        .text_color(Theme::navigation_ink(false)).hover(|s| s.bg(Theme::navigation_highlight()))
        .focus(|s| s.bg(Theme::navigation_highlight()))
}

impl Workspace {
    pub(crate) fn close_index_menu(&mut self) {
        if let Some(nav) = &mut self.navigation { nav.index.open = None; }
    }

    fn toggle_index_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.navigation.as_mut().unwrap().index;
        if state.open == Some(menu) {
            state.open = None;
            self.focus.focus(window);
        } else {
            state.open = Some(menu);
            if menu == Menu::Dates { state.calendar = Calendar::new(self.habits.as_ref().unwrap().read(cx).period); }
            state.focus.get_or_insert_with(|| cx.focus_handle()).focus(window);
        }
        cx.notify();
    }

    pub(super) fn choose_period(&mut self, period: Period, window: &mut Window, cx: &mut Context<Self>) {
        self.habits.as_ref().unwrap().update(cx, |h, cx| { h.period = period; cx.notify(); });
        self.close_index_menu();
        self.focus.focus(window);
        cx.notify();
    }

    pub(crate) fn render_ritual_index(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let habits = self.habits.as_ref().unwrap();
        let state = &self.navigation.as_ref().unwrap().index;
        let period = habits.read(cx).period;
        let header = page_header_with(self, div(), 0., cx)
            .pr(px(ui::header_right_inset() + 10.)).gap(px(3.))
            .child(pill(Menu::Actions.id(), state.open == Some(Menu::Actions))
                .child(ui::icon("plus", 14., Theme::index_secondary())).child("Actions").child(self.home.track(Menu::Actions.id()))
                .on_click(cx.listener(|w, _, win, cx| w.toggle_index_menu(Menu::Actions, win, cx))))
            .child(pill(Menu::Dates.id(), state.open == Some(Menu::Dates)).min_w(px(104.)).justify_between()
                .child(period.label()).child(ui::icon("chevron-down", 12., Theme::index_secondary())).child(self.home.track(Menu::Dates.id()))
                .on_click(cx.listener(|w, _, win, cx| w.toggle_index_menu(Menu::Dates, win, cx))));
        let mut panel = div().size_full().min_w_0().flex().flex_col().font_family(appearance::navigation_font(cx))
            .child(header).child(habits.clone())
            .children(self.composer.as_ref().filter(|c| !c.read(cx).expanded).cloned()
                .map(|composer| div().w_full().flex_none().flex().justify_center().px(px(16.)).pb(px(32.)).child(composer)));
        if let Some(menu) = state.open && let Some(bounds) = self.home.anchor(menu.id()) {
            let available = f32::from(window.viewport_size().width) - 16.;
            let body = match menu {
                Menu::Actions => self.index_actions(habits, cx),
                Menu::Dates => self.index_dates(period, available, cx),
            };
            let shell = ui::popover().id("index-menu").occlude().track_focus(state.focus.as_ref().unwrap()).tab_group()
                .font_family(appearance::navigation_font(cx)).rounded(px(9.)).border_color(Theme::index_border())
                .max_h(px((f32::from(window.viewport_size().height)-58.).max(60.))).overflow_y_scroll().child(body)
                .on_key_down(cx.listener(|w, event: &KeyDownEvent, win, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => { w.close_index_menu(); w.focus.focus(win); cx.notify(); }
                        "down" => win.focus_next(), "up" => win.focus_prev(),
                        "tab" if event.keystroke.modifiers.shift => win.focus_prev(), "tab" => win.focus_next(),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .on_mouse_down_out(cx.listener(|w, event: &MouseDownEvent, _, cx| {
                    if [Menu::Dates, Menu::Actions].iter().any(|m| w.home.anchor(m.id()).is_some_and(|b| b.contains(&event.position))) { return; }
                    w.close_index_menu(); cx.notify();
                }));
            panel = panel.child(deferred(anchored().anchor(Corner::TopRight).position(point(bounds.right(), bounds.bottom()+px(5.)))
                .snap_to_window_with_margin(px(8.)).child(shell)).with_priority(2));
        }
        panel.into_any_element()
    }

    fn index_actions(&self, habits: &Entity<Habits>, cx: &mut Context<Self>) -> Div {
        let grid = habits.read(cx).grid;
        let mut menu = div().w(px(182.)).p(px(5.))
            .child(div().px(px(9.)).py(px(4.)).text_size(px(11.)).text_color(Theme::index_secondary()).child("View"));
        for (label, glyph, cards) in [("List", "view-list", false), ("Card", "view-grid", true)] {
            menu = menu.child(option(label).child(ui::icon(glyph, 14., Theme::navigation_ink(false))).child(label).child(div().flex_1())
                .when(grid == cards, |d| d.child(ui::icon("check", 12., Theme::index_secondary())))
                .on_click(cx.listener(move |w, _, win, cx| {
                    w.habits.as_ref().unwrap().update(cx, |h, cx| { h.grid = cards; h.show_list(cx); });
                    let mut settings = w.store.settings(); settings.ritual_habits_grid = cards;
                    if let Err(e) = w.store.save_settings(&settings) { w.show_toast_kind(ui::ToastKind::Error, &e.to_string(), cx); }
                    w.close_index_menu(); w.focus.focus(win); cx.notify();
                })));
        }
        let divider = || div().h(px(1.)).my(px(4.)).bg(Theme::index_border());
        menu = menu.child(divider()).child(option("index-add-habit").child(ui::icon("plus", 14., Theme::navigation_ink(false))).child("Add habit")
            .on_click(cx.listener(|w, _, win, cx| { w.close_index_menu(); w.focus.focus(win); w.pick_habit(win, cx); }))).child(divider());
        for (label, glyph, tab) in [("My documents", "folder", HomeTab::Recent), ("Favorites", "star", HomeTab::Starred)] {
            menu = menu.child(option(label).child(ui::icon(glyph, 14., Theme::navigation_ink(false))).child(label)
                .on_click(cx.listener(move |w, _, win, cx| { w.close_index_menu(); w.home.tab = tab; w.focus.focus(win); cx.notify(); })));
        }
        menu.child(option("index-import-documents").child(ui::icon("import", 14., Theme::navigation_ink(false))).child("Import documents…")
            .on_click(cx.listener(|w, _, win, cx| { w.close_index_menu(); w.focus.focus(win); w.open_import(win, cx); })))
    }
}
