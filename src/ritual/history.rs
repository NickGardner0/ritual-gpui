//! Window-local history of visited pages. Menus, filter changes and editing stay on the same page.
use super::{Page, Uuid};
use crate::{app::{View, Workspace}, home::HomeTab, settings};
use gpui::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Location { Home(HomeTab), Habit(Uuid), Ritual(Page), LibraryItem(String), Settings(settings::Page), Trash }

#[derive(Default)]
pub(super) struct PageHistory { pages: Vec<Location>, cursor: usize }

impl PageHistory {
    fn visit(&mut self, location: Location) {
        if self.pages.get(self.cursor) == Some(&location) { return; }
        self.pages.truncate(self.cursor + 1);
        self.pages.push(location);
        self.cursor = self.pages.len() - 1;
    }

    fn target(&self, forward: bool, available: impl Fn(&Location) -> bool) -> Option<usize> {
        let valid = |i: &usize| self.pages.get(self.cursor) != Some(&self.pages[*i]) && available(&self.pages[*i]);
        if forward { (self.cursor + 1..self.pages.len()).find(valid) }
        else { (0..self.cursor).rev().find(valid) }
    }
}

impl Workspace {
    pub(crate) fn record_ritual_page(&mut self, cx: &App) {
        let Some(nav) = &mut self.navigation else { return };
        let location = match self.view {
            View::Ritual(page) => Location::Ritual(page),
            View::Home => self.habits.as_ref().filter(|_| self.home.tab == HomeTab::Habits).and_then(|h| h.read(cx).selected)
                .map_or(Location::Home(self.home.tab), Location::Habit),
            View::Doc | View::Folder => {
                let Some(id) = (if self.view == View::Doc { &self.current } else { &self.folder_current }) else { return };
                Location::LibraryItem(id.clone())
            }
            View::Settings => Location::Settings(self.settings_page),
            View::Trash => Location::Trash,
        };
        nav.visits.visit(location);
    }

    pub(super) fn history_target(&self, forward: bool, cx: &App) -> Option<usize> {
        let habits = self.habits.as_ref()?.read(cx);
        self.navigation.as_ref()?.visits.target(forward, |location| match location {
            Location::Habit(id) => habits.habits.iter().any(|h| h.habit.id == *id),
            Location::LibraryItem(id) => self.entries.iter().any(|e| e.id == *id),
            Location::Ritual(Page::Space(id) | Page::Draft(Some(id))) => habits.spaces.spaces.iter().any(|s| s.id == *id),
            Location::Ritual(Page::Thread(id)) => habits.spaces.threads.iter().any(|t| t.id == *id),
            _ => true,
        })
    }

    pub(super) fn step_page_history(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.history_target(forward, cx) else { return };
        let location = self.navigation.as_ref().unwrap().visits.pages[target].clone();
        self.close_index_menu();
        self.batch = None;
        match location {
            Location::Ritual(page) => self.go_ritual(page, window, cx),
            Location::LibraryItem(id) => {
                if !self.save_before(cx, move |ws, w, cx| ws.step_page_history(forward, w, cx)) { return; }
                if !self.try_open(&id, window, cx) { return; }
                if self.view == View::Doc && let Some(editor) = &self.editor { editor.read(cx).focus(window); }
            }
            Location::Home(_) | Location::Habit(_) => {
                let habits = self.habits.as_ref().unwrap().clone();
                if habits.read(cx).saving() { return; }
                self.show_view(View::Home, cx);
                self.focus.focus(window);
                match location {
                    Location::Home(tab) => self.home.tab = tab,
                    Location::Habit(id) => habits.update(cx, |h, cx| h.select(id, window, cx)),
                    _ => unreachable!(),
                }
            }
            Location::Settings(page) => { self.open_settings(page, cx); self.focus.focus(window); }
            Location::Trash => { self.show_view(View::Trash, cx); self.focus.focus(window); }
        }
        self.navigation.as_mut().unwrap().visits.cursor = target;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{HomeTab, Location, Page, PageHistory};
    use crate::ritual::Destination;

    #[test]
    fn revisiting_a_page_branches_only_after_a_new_visit() {
        let mut h = PageHistory::default();
        let index = Location::Home(HomeTab::Habits);
        let metrics = Location::Ritual(Page::Destination(Destination::Metrics));
        h.visit(index.clone());
        h.visit(metrics.clone());
        h.visit(Location::LibraryItem("deleted".into()));
        h.visit(Location::Trash);
        h.cursor = h.target(false, |p| !matches!(p, Location::LibraryItem(_))).unwrap();
        assert_eq!(h.pages[h.cursor], metrics);
        h.visit(metrics);
        assert_eq!(h.target(true, |_| true), Some(2));
        h.cursor = h.target(false, |_| true).unwrap();
        assert_eq!(h.pages[h.cursor], index);
        assert_eq!(h.target(false, |_| true), None);
        h.visit(Location::Home(HomeTab::Starred));
        assert_eq!(h.target(true, |_| true), None);
        assert_eq!(h.pages.len(), 2);
    }
}
