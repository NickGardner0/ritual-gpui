//! Tracked habits on the Ritual Home tab.
mod view;
mod picker;
mod catalog;
pub(crate) use picker::{Picker, PickerClosed};

use crate::input::{self, TextField, TextFieldEvent};
use anyhow::{Context as _, Result};
use async_lock::Mutex;
use gpui::*;
use ritual_core::{Entry, Habit, ManualLog, Store, TrackedHabit};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

pub(crate) struct Profile;
impl Global for Profile {}

pub fn run() { crate::run_workspace(true); }

pub(super) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", input::Cancel, Some("HabitPicker")),
        KeyBinding::new("enter", input::Submit, Some("HabitPicker")),
        KeyBinding::new("tab", input::Tab, Some("HabitPicker")),
        KeyBinding::new("shift-tab", input::ShiftTab, Some("HabitPicker")),
        KeyBinding::new("tab", input::Tab, Some("Habits")),
        KeyBinding::new("shift-tab", input::ShiftTab, Some("Habits")),
        KeyBinding::new("escape", input::Cancel, Some("Habits")),
    ]);
}

fn database_path() -> Result<PathBuf> {
    let root = match std::env::var_os("RITUAL_GPUI_DATA_DIR") {
        Some(value) => {
            anyhow::ensure!(!value.is_empty(), "RITUAL_GPUI_DATA_DIR is empty.");
            PathBuf::from(value)
        }
        None => dirs::data_dir().context("Could not locate Application Support")?.join("Ritual GPUI Dev"),
    };
    Ok(root.join("ritual.sqlite3"))
}

pub(crate) struct Habits {
    db: Option<Arc<Mutex<Store>>>,
    habits: Vec<TrackedHabit>,
    pub(crate) grid: bool,
    selected: Option<Uuid>,
    history: Option<Vec<Entry>>,
    history_request: u64,
    saving: bool,
    quit_after_save: bool,
    log_draft: Option<ManualLog>,
    error: Option<String>,
    amount: Entity<TextField>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Habits {
    pub(crate) fn new(grid: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let amount = cx.new(|cx| {
            let mut f = TextField::new("e.g. 2", cx); f.set_tab_index(1); f
        });
        let mut subscriptions = vec![cx.subscribe_in(&amount, window, |h, _, event, w, cx| {
            if h.saving { return; }
            match event {
                TextFieldEvent::Changed => { h.log_draft = None; if h.history.is_some() { h.error = None; } cx.notify(); }
                TextFieldEvent::Submit => h.submit(w, cx),
                TextFieldEvent::Cancel => { h.show_list(cx); h.focus.focus(w); }
                _ => {}
            }
        })];
        subscriptions.push(cx.observe_window_appearance(window, |this, window, cx| this.sync_colors(window, cx)));
        let mut this = Self { db: None, habits: vec![], selected: None, history: None, history_request: 0,
            grid, saving: false, quit_after_save: false, log_draft: None, error: None,
            amount, focus: cx.focus_handle(),
            _subscriptions: subscriptions };
        this.sync_colors(window, cx);
        this.open_database(window, cx);
        this
    }

    fn open_database(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let load = cx.background_executor().spawn(async {
            let path = database_path()?;
            let db = Store::open(&path).with_context(|| format!("Open {}", path.display()))?;
            let habits = db.index()?;
            anyhow::Ok((db, habits))
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok((db, habits)) => {
                        view.db = Some(Arc::new(Mutex::new(db)));
                        view.habits = habits;
                    }
                    Err(err) => view.error = Some(format!("{err:#}")),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn select(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving { return; }
        self.load_habit(id, cx);
        self.amount.read(cx).focus_handle(cx).focus(window);
    }

    fn load_habit(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        self.selected = Some(id);
        self.error = None;
        self.log_draft = None;
        self.amount.update(cx, |f, cx| f.set_text("", cx));
        self.history = None;
        self.history_request += 1;
        let request = self.history_request;
        let load = cx.background_executor().spawn(async move { db.lock().await.history(id) });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |view, cx| {
                if view.history_request != request || view.selected != Some(id) { return; }
                match result {
                    Ok(entries) => view.history = Some(entries),
                    Err(err) => view.error = Some(format!("Could not load history. {err:#}")),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    pub(crate) fn saving(&self) -> bool { self.saving }

    pub(crate) fn wait_to_quit(&mut self) -> bool {
        self.quit_after_save = self.saving;
        self.saving
    }

    pub(crate) fn show_list(&mut self, cx: &mut Context<Self>) {
        if self.saving { return; }
        self.selected = None;
        self.history = None;
        if self.db.is_some() { self.error = None; }
        cx.notify();
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving { return; }
        let Some(db) = self.db.clone() else { return };
        let task = {
            let Some(id) = self.selected else { return };
            if self.history.is_none() { return; }
            let Ok(amount) = self.amount.read(cx).text.trim().parse::<f64>() else {
                self.error = Some("Enter a numeric amount greater than zero.".into());
                cx.notify();
                return;
            };
            let command = self.log_draft.get_or_insert_with(|| ManualLog::now(id, amount)).clone();
            cx.background_executor().spawn(async move { db.lock().await.append(&command) })
        };
        self.saving = true;
        self.error = None;
        self.focus.focus(window);
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |view, window, cx| {
                let restore_focus = view.focus.is_focused(window);
                view.saving = false;
                let quit = std::mem::take(&mut view.quit_after_save) && result.is_ok();
                match result {
                    Ok(entry) => {
                        if let Some(rows) = &mut view.history {
                            let pos = rows.partition_point(|row| (row.completed_at, row.id) > (entry.completed_at, entry.id));
                            if rows.get(pos).is_none_or(|row| row.id != entry.id) {
                                if let Some(h) = view.habits.iter_mut().find(|h| h.habit.id == entry.habit_id) {
                                    h.count += 1; h.total += entry.amount;
                                }
                                rows.insert(pos, entry);
                            }
                        }
                        view.log_draft = None;
                        view.amount.update(cx, |f, cx| f.set_text("", cx));
                        if restore_focus { view.amount.read(cx).focus_handle(cx).focus(window); }
                    }
                    Err(err) => {
                        view.error = Some(format!("{err:#}"));
                        let field = &view.amount;
                        if restore_focus { field.read(cx).focus_handle(cx).focus(window); }
                    }
                }
                cx.notify();
                if quit { cx.defer(crate::app::quit); }
            });
        }).detach();
        cx.notify();
    }
}
