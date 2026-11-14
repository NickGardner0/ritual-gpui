//! The Index has its own title bar. The document library keeps its Home toolbar.
use super::{Habits, appearance, index_calendar::{Calendar, Period}};
use crate::{app::Workspace, home::HomeTab, theme::Theme, trash::page_header_with, ui::{self, TitleBarExt}};
use gpui::{prelude::FluentBuilder, *};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Menu { Actions, Dates }
impl Menu {
    fn id(self) -> &'static str { match self { Self::Actions => "index-actions", Self::Dates => "index-dates" } }
}

pub(super) struct IndexMenuState {
    open: Option<Menu>,
    triggers: [FocusHandle; 2],
    actions: ui::MenuFocus<6>,
    pub(super) calendar: Calendar,
}

impl IndexMenuState {
    pub(super) fn new(cx: &App) -> Self {
        Self { open: None, triggers: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            actions: ui::MenuFocus::new(cx), calendar: Calendar::new(cx) }
    }
    fn trigger(&self, menu: Menu) -> &FocusHandle { &self.triggers[menu as usize] }
}

fn pill(menu: Menu, state: &IndexMenuState) -> Stateful<Div> {
    let hover = Theme::navigation_highlight();
    ui::button_focus(ui::button_surface(menu.id(), true, hover), state.trigger(menu), hover)
        .relative().no_drag().h(px(28.)).px(px(10.)).flex_none().flex().items_center().gap(px(6.))
        .rounded_full().border_1().border_color(Theme::index_border())
        .text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(Theme::index_secondary())
        .when(state.open == Some(menu), |d| d.bg(hover))
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

    fn dismiss_index_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.navigation.as_mut().unwrap().index;
        if let Some(menu) = state.open.take() { state.trigger(menu).focus(window); cx.notify(); }
    }

    fn toggle_index_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        let state = &mut self.navigation.as_mut().unwrap().index;
        if state.open == Some(menu) { self.dismiss_index_menu(window, cx); return; }
        state.open = Some(menu);
        let habits = self.habits.as_ref().unwrap().read(cx);
        match menu {
            Menu::Actions => state.actions.items[usize::from(habits.grid)].focus(window),
            Menu::Dates => state.calendar.open(habits.period, window),
        }
        cx.notify();
    }

    pub(super) fn choose_period(&mut self, period: Period, window: &mut Window, cx: &mut Context<Self>) {
        self.habits.as_ref().unwrap().update(cx, |h, cx| { h.period = period; cx.notify(); });
        self.dismiss_index_menu(window, cx);
    }

    pub(crate) fn render_ritual_index(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let habits = self.habits.as_ref().unwrap();
        let state = &self.navigation.as_ref().unwrap().index;
        let period = habits.read(cx).period;
        let header = page_header_with(self, div(), 0., cx)
            .pr(px(ui::header_right_inset() + 10.)).gap(px(3.))
            .on_key_down(cx.listener(|w, event: &KeyDownEvent, win, cx| {
                let state = &w.navigation.as_ref().unwrap().index;
                if ui::plain_key(&event.keystroke) == Some("down") {
                    if let Some(menu) = [Menu::Actions, Menu::Dates].into_iter().find(|m| state.trigger(*m).is_focused(win)) {
                        w.toggle_index_menu(menu, win, cx); cx.stop_propagation();
                    }
                } else if ui::traverse_tab(&event.keystroke, win) { cx.stop_propagation(); }
            }))
            .child(pill(Menu::Actions, state)
                .child(ui::icon("plus", 14., Theme::index_secondary())).child("Actions").child(self.home.track(Menu::Actions.id()))
                .on_click(cx.listener(|w, _, win, cx| w.toggle_index_menu(Menu::Actions, win, cx))))
            .child(pill(Menu::Dates, state).min_w(px(104.)).justify_between()
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
                Menu::Dates => self.index_dates(period, available, f32::from(window.viewport_size().height), cx),
            };
            let shell = ui::popover().id("index-menu").occlude().tab_group()
                .font_family(appearance::navigation_font(cx)).rounded(px(9.)).border_color(Theme::index_border())
                .when(menu == Menu::Dates, |d| d.bg(Theme::calendar_surface()).rounded(px(14.)).text_color(Theme::navigation_ink(true)))
                .max_h(px((f32::from(window.viewport_size().height)-58.).max(60.))).overflow_y_scroll().child(body)
                .on_key_down(cx.listener(|w, event: &KeyDownEvent, win, cx| {
                    let menu = w.navigation.as_ref().unwrap().index.open;
                    match menu {
                        Some(Menu::Dates) if w.date_picker_key(&event.keystroke, win, cx) => {}
                        Some(_) if ui::plain_key(&event.keystroke) == Some("escape") => w.dismiss_index_menu(win, cx),
                        Some(Menu::Actions) if w.navigation.as_ref().unwrap().index.actions.navigate(&event.keystroke, win).is_some() => {}
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .on_mouse_down_out(cx.listener(|w, event: &MouseDownEvent, win, cx| {
                    if [Menu::Dates, Menu::Actions].iter().any(|m| w.home.anchor(m.id()).is_some_and(|b| b.contains(&event.position))) { return; }
                    w.dismiss_index_menu(win, cx);
                }));
            panel = panel.child(deferred(anchored().anchor(Corner::TopRight).position(point(bounds.right(), bounds.bottom()+px(5.)))
                .snap_to_window_with_margin(px(8.)).child(shell)).with_priority(2));
        }
        panel.into_any_element()
    }

    fn index_actions(&self, habits: &Entity<Habits>, cx: &mut Context<Self>) -> Div {
        let grid = habits.read(cx).grid;
        let actions = &self.navigation.as_ref().unwrap().index.actions.items;
        let option = |id: &'static str, index: usize| ui::menu_choice(id, &actions[index], Theme::navigation_highlight())
            .text_size(px(13.)).h(px(30.)).rounded(px(6.)).px(px(9.)).gap(px(8.)).text_color(Theme::navigation_ink(false));
        let mut menu = div().w(px(182.)).p(px(5.))
            .child(div().px(px(9.)).py(px(4.)).text_size(px(11.)).text_color(Theme::index_secondary()).child("View"));
        for (label, glyph, cards) in [("List", "view-list", false), ("Card", "view-grid", true)] {
            menu = menu.child(option(label, usize::from(cards)).child(ui::icon(glyph, 14., Theme::navigation_ink(false))).child(label).child(div().flex_1())
                .when(grid == cards, |d| d.child(ui::icon("check", 12., Theme::index_secondary())))
                .on_click(cx.listener(move |w, _, win, cx| {
                    w.habits.as_ref().unwrap().update(cx, |h, cx| { h.grid = cards; h.show_list(cx); });
                    let mut settings = w.store.settings(); settings.ritual_habits_grid = cards;
                    if let Err(e) = w.store.save_settings(&settings) { w.show_toast_kind(ui::ToastKind::Error, &e.to_string(), cx); }
                    w.dismiss_index_menu(win, cx);
                })));
        }
        let divider = || div().h(px(1.)).my(px(4.)).bg(Theme::index_border());
        menu = menu.child(divider()).child(option("index-add-habit", 2).child(ui::icon("plus", 14., Theme::navigation_ink(false))).child("Add habit")
            .on_click(cx.listener(|w, _, win, cx| { w.close_index_menu(); w.focus.focus(win); w.pick_habit(win, cx); }))).child(divider());
        for (i, (label, glyph, tab)) in [("My documents", "folder", HomeTab::Recent), ("Favorites", "star", HomeTab::Starred)].into_iter().enumerate() {
            menu = menu.child(option(label, 3 + i).child(ui::icon(glyph, 14., Theme::navigation_ink(false))).child(label)
                .on_click(cx.listener(move |w, _, win, cx| { w.close_index_menu(); w.home.tab = tab; w.focus.focus(win); cx.notify(); })));
        }
        menu.child(option("index-import-documents", 5).child(ui::icon("import", 14., Theme::navigation_ink(false))).child("Import documents…")
            .on_click(cx.listener(|w, _, win, cx| { w.close_index_menu(); w.focus.focus(win); w.open_import(win, cx); })))
    }
}
