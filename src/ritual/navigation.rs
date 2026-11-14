//! Ritual navigation lives in the main workspace. Quick-note windows keep their document chrome.
use super::*;
use crate::{app::{View, Workspace}, theme::Theme};
use ritual_core::{Space, Thread};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Destination { Home, Metrics, Logs, Tasks, Calendar, Reports, Routines, Experiments, Integrations, Settings }

impl Destination {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Home => "Index", Self::Metrics => "Metrics", Self::Logs => "Logs", Self::Tasks => "Tasks",
            Self::Calendar => "Calendar", Self::Reports => "Reports", Self::Routines => "Routines",
            Self::Experiments => "Experiments", Self::Integrations => "Integrations", Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page { Destination(Destination), CreateSpace, Space(Uuid), Draft(Option<Uuid>), Thread(Uuid) }

pub(crate) struct Navigation {
    pub(super) index: super::index::IndexMenuState,
    pub(super) expanded: HashSet<Uuid>,
    pub(super) history_buttons: [FocusHandle; 2],
    pub(super) visits: super::history::PageHistory,
    pub(super) hovered_destination: Option<Destination>,
    pub(super) name: Entity<TextField>,
    pub(super) goal: Entity<TextField>,
    pub(super) message: Entity<TextField>,
    drafts: HashMap<Page, String>,
    pub(super) previous: View,
    pub(super) messages: Vec<String>,
    pub(super) logs: Vec<(String, String, Entry)>,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
    request: u64,
    pub(super) history_scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl Navigation {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Workspace>) -> Self {
        let field = |label, index, multiline, cx: &mut Context<Workspace>| cx.new(|cx| {
            let mut f = TextField::new(label, cx); f.set_tab_index(index); f.tab_events = true;
            f.multiline = multiline; f.wrap = multiline; f.submit_on_enter = true; f
        });
        let name = field("e.g. Sleep & recovery", 1, false, cx);
        let goal = field("What would you like to understand or change?", 2, true, cx);
        let message = field("Write a message…", 1, true, cx);
        let mut subscriptions = vec![];
        for input in [&name, &goal, &message] {
            subscriptions.push(cx.subscribe_in(input, window, |ws, field, event, window, cx| {
                let Some(nav) = &mut ws.navigation else { return };
                match event {
                    TextFieldEvent::Submit => ws.save_conversation(window, cx),
                    TextFieldEvent::Tab => window.focus_next(),
                    TextFieldEvent::ShiftTab => window.focus_prev(),
                    TextFieldEvent::Cancel if ws.view == View::Ritual(Page::CreateSpace) => {
                        ws.cancel_create_space(window, cx);
                    }
                    TextFieldEvent::Changed => {
                        if *field == nav.message && let View::Ritual(page) = ws.view {
                            nav.drafts.insert(page, field.read(cx).text.clone());
                        }
                        cx.notify();
                    }
                    _ => {}
                }
            }));
        }
        Self { index: super::index::IndexMenuState::new(cx), history_buttons: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)), name, goal, message, previous: View::Home, expanded: Default::default(), visits: Default::default(), hovered_destination: None,
            drafts: Default::default(), messages: vec![], logs: vec![], loading: false, error: None, request: 0,
            history_scroll: ScrollHandle::new(), _subscriptions: subscriptions }
    }

    pub(crate) fn refresh_colors(&self, cx: &mut Context<Workspace>) {
        for field in [&self.name, &self.goal, &self.message] {
            field.update(cx, |f, cx| { f.color = Theme::text(); f.placeholder_color = Theme::text_placeholder(); cx.notify(); });
        }
    }
}

enum Saved { Space(Space), Thread(Thread), Message }

impl Workspace {
    pub(super) fn cancel_create_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.navigation.as_ref().unwrap().previous;
        if let View::Ritual(page) = previous { self.go_ritual(page, window, cx); }
        else { self.show_view(previous, cx); self.focus.focus(window); }
    }

    pub(super) fn go_destination(&mut self, dest: Destination, window: &mut Window, cx: &mut Context<Self>) {
        self.batch = None;
        window.focus(&self.focus);
        match dest {
            Destination::Home => self.show_view(View::Home, cx),
            Destination::Settings => { self.sidebar_open = true; self.show_view(View::Settings, cx); }
            _ => self.go_ritual(Page::Destination(dest), window, cx),
        }
    }

    pub(super) fn go_ritual(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.view;
        self.show_view(View::Ritual(page), cx);
        self.batch = None;
        let nav = self.navigation.as_mut().unwrap();
        nav.error = None;
        nav.loading = false;
        nav.request += 1;
        let request = nav.request;
        self.focus.focus(window);
        match page {
            Page::CreateSpace => {
                if previous != self.view { nav.previous = previous; }
                nav.name.read(cx).focus_handle(cx).focus(window);
            }
            Page::Space(id) => { nav.expanded.insert(id); }
            Page::Thread(_) | Page::Draft(_) => {
                let text = nav.drafts.get(&page).cloned().unwrap_or_default();
                nav.message.update(cx, |f, cx| f.set_text(&text, cx));
                nav.message.read(cx).focus_handle(cx).focus(window);
                nav.messages.clear();
            }
            _ => {}
        }
        if !matches!(page, Page::Thread(_) | Page::Destination(Destination::Logs)) { return; }
        let Some(db) = self.habits.as_ref().and_then(|h| h.read(cx).db.clone()) else {
            nav.error = Some("The Ritual database is unavailable.".into()); return;
        };
        nav.loading = true;
        let load = cx.background_executor().spawn(async move {
            let db = db.lock().await;
            match page {
                Page::Thread(id) => Ok((db.messages(id)?, vec![])),
                _ => Ok((vec![], db.all_logs()?)),
            }
        });
        cx.spawn(async move |ws, cx| {
            let result: Result<_> = load.await;
            let _ = ws.update(cx, |ws, cx| {
                let nav = ws.navigation.as_mut().unwrap();
                if request != nav.request { return; }
                nav.loading = false;
                match result {
                    Ok((messages, logs)) => {
                        nav.messages = messages; nav.logs = logs;
                        if matches!(page, Page::Thread(_)) { nav.history_scroll.scroll_to_bottom(); }
                    }
                    Err(e) => nav.error = Some(e.to_string()),
                }
                cx.notify();
            });
        }).detach();
    }

    pub(super) fn save_conversation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let View::Ritual(page) = self.view else { return };
        let Some(nav) = &mut self.navigation else { return };
        let Some(habits) = self.habits.clone() else { return };
        let Some(db) = habits.read(cx).db.clone() else { return };
        if habits.read(cx).saving || nav.loading { return; }
        let name = nav.name.read(cx).text.clone();
        let goal = nav.goal.read(cx).text.clone();
        let text = nav.message.read(cx).text.clone();
        match page {
            Page::CreateSpace if !name.trim().is_empty() => {}
            Page::Draft(_) | Page::Thread(_) if !text.trim().is_empty() => {}
            _ => return,
        }
        nav.error = None;
        habits.update(cx, |h, cx| { h.saving = true; cx.notify(); });
        let body = text.clone();
        let save = cx.background_executor().spawn(async move {
            let mut db = db.lock().await;
            match page {
                Page::CreateSpace => db.create_space(&name, &goal).map(Saved::Space),
                Page::Draft(scope) => db.start_thread(scope, &body).map(Saved::Thread),
                Page::Thread(id) => db.add_message(id, &body).map(|_| Saved::Message),
                _ => unreachable!(),
            }
        });
        cx.spawn_in(window, async move |ws, cx| {
            let result = save.await;
            let _ = ws.update_in(cx, |ws, window, cx| {
                let quit = habits.update(cx, |h, cx| {
                    h.saving = false; cx.notify();
                    std::mem::take(&mut h.quit_after_save) && result.is_ok()
                });
                let nav = ws.navigation.as_mut().unwrap();
                match result {
                    Ok(saved) => {
                        let next = match saved {
                            Saved::Space(space) => {
                                let id = space.id;
                                habits.update(cx, |h, cx| { Arc::make_mut(&mut h.spaces).spaces.push(space); cx.notify(); });
                                for field in [&nav.name, &nav.goal] { field.update(cx, |f, cx| f.set_text("", cx)); }
                                nav.expanded.insert(id); Page::Space(id)
                            }
                            Saved::Thread(thread) => {
                                let id = thread.id;
                                if let Some(scope) = thread.space { nav.expanded.insert(scope); }
                                habits.update(cx, |h, cx| { Arc::make_mut(&mut h.spaces).threads.insert(0, thread); cx.notify(); });
                                Page::Thread(id)
                            }
                            Saved::Message => page,
                        };
                        if nav.drafts.get(&page) == Some(&text) { nav.drafts.remove(&page); }
                        if ws.view == View::Ritual(page) { ws.go_ritual(next, window, cx); }
                    }
                    Err(err) => ws.show_toast_kind(crate::ui::ToastKind::Error, &format!("Could not save. {err}"), cx),
                }
                cx.notify();
                if quit { cx.defer(crate::app::quit); }
            });
        }).detach();
    }
}
