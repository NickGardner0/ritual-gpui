//! Home and the expanded composer share a session-only draft for each mode.
mod view;

use super::Habits;
use crate::input::{TextField, TextFieldEvent};
use gpui::*;
use std::path::PathBuf;
use uuid::Uuid;

pub(crate) struct PresentationChanged;
impl EventEmitter<PresentationChanged> for Composer {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Log, Chat }

impl Mode {
    fn label(self) -> &'static str { match self { Self::Log => "Log", Self::Chat => "Chat" } }
    fn placeholder(self) -> &'static str {
        match self { Self::Log => "Log anything...", Self::Chat => "Ask about your personal data..." }
    }
}

/// Ritual Desktop's surface-panel, text, and icon tokens (dashboard/app/globals.css).
struct ComposerColors {
    surface: Hsla,
    text: Hsla,
    secondary: Hsla,
    muted: Hsla,
    icon: Hsla,
    on_send: Hsla,
    action_fill: Hsla,
    mode_track: Hsla,
    mode_selected: Hsla,
    send_disabled: Hsla,
}

impl ComposerColors {
    fn current() -> Self {
        if let Some(p) = super::appearance::palette() {
            return Self { surface: p.surface, text: p.text, secondary: p.muted, muted: p.placeholder, icon: p.icon,
                on_send: p.page, action_fill: p.hover, mode_track: p.hover, mode_selected: p.selected, send_disabled: p.disabled };
        }
        let dark = crate::theme::dark();
        let color = |light, night| rgb(if dark { night } else { light }).into();
        let text: Hsla = color(0x111111, 0xf7f7f3);
        // WebKit composites these translucent controls differently from GPUI.
        // Light fills are sampled from Ritual Desktop; dark fills use its CSS opacities.
        let fill = |light, opacity| if dark { text.opacity(opacity) } else { rgb(light).into() };
        Self {
            surface: color(0xf4f4f3, 0x151516),
            text,
            secondary: color(0x666666, 0xb8b8b2),
            muted: color(0x7a7a7a, 0x9a9a94),
            icon: color(0x5f6368, 0xb7b7b0),
            on_send: color(0xffffff, 0x151516),
            action_fill: fill(0xe6e6e5, 0.07),
            mode_track: fill(0xeaeae9, 0.05),
            mode_selected: fill(0xd0cfcf, 0.14),
            send_disabled: fill(0xaeaead, 0.35),
        }
    }
}

#[derive(Clone, Default)]
struct Draft {
    text: String,
    habit: Option<(Uuid, String)>,
    attachment: Option<PathBuf>,
}

#[derive(Default)]
struct Drafts {
    input: Draft,
    previews: Vec<Draft>,
    editing: Option<usize>,
}

pub(crate) struct Composer {
    habits: Entity<Habits>,
    input: Entity<TextField>,
    focus: FocusHandle,
    mode: Mode,
    log: Drafts,
    chat: Drafts,
    pub(crate) expanded: bool,
    context_open: bool,
    picking_file: bool,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl Composer {
    pub(crate) fn new(habits: Entity<Habits>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let family = super::appearance::production_font(cx);
        let colors = ComposerColors::current();
        let input = cx.new(|cx| {
            let mut f = TextField::new(Mode::Log.placeholder(), cx);
            f.multiline = true;
            f.submit_on_enter = true;
            f.wrap = true;
            f.font = Some(family);
            f.size = 15.;
            f.line_height = 20.;
            f.color = colors.text;
            f.placeholder_color = colors.muted;
            f.tab_events = true;
            f.set_tab_index(1);
            f
        });
        let sub = cx.subscribe_in(&input, window, |p, field, event, w, cx| match event {
            TextFieldEvent::Changed => {
                p.drafts_mut().input.text = field.read(cx).text.clone();
                cx.notify();
            }
            TextFieldEvent::Submit => p.preview(w, cx),
            TextFieldEvent::Cancel => p.escape(w, cx),
            TextFieldEvent::Tab => p.advance_focus(false, w, cx),
            TextFieldEvent::ShiftTab => p.advance_focus(true, w, cx),
            _ => {}
        });
        Self { habits, input, focus: cx.focus_handle(), mode: Mode::Log, log: Drafts::default(), chat: Drafts::default(),
            expanded: false, context_open: false, picking_file: false, scroll: ScrollHandle::new(), _subscriptions: vec![sub] }
    }

    fn drafts(&self) -> &Drafts { match self.mode { Mode::Log => &self.log, Mode::Chat => &self.chat } }
    fn drafts_mut(&mut self) -> &mut Drafts { match self.mode { Mode::Log => &mut self.log, Mode::Chat => &mut self.chat } }

    fn focus_input(&self, w: &mut Window, cx: &App) { self.input.read(cx).focus_handle(cx).focus(w); }

    fn advance_focus(&self, backwards: bool, window: &mut Window, cx: &App) {
        if backwards { window.focus_prev(); } else { window.focus_next(); }
        if self.expanded && !self.focus.contains_focused(window, cx) { self.focus_input(window, cx); }
    }

    fn expand(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.expanded { self.expanded = true; cx.emit(PresentationChanged); }
        self.context_open = false;
        self.focus_input(w, cx);
        cx.notify();
    }

    pub(crate) fn collapse(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.expanded = false;
        self.context_open = false;
        self.focus_input(w, cx);
        cx.emit(PresentationChanged);
        cx.notify();
    }

    fn escape(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.context_open { self.context_open = false; self.focus_input(w, cx); }
        else if self.expanded { self.collapse(w, cx); }
        else { self.focus.focus(w); }
        cx.notify();
    }

    fn switch_mode(&mut self, mode: Mode, w: &mut Window, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            self.context_open = false;
            let text = self.drafts().input.text.clone();
            self.input.update(cx, |f, cx| { f.set_text(&text, cx); f.set_placeholder(mode.placeholder(), cx); });
            self.scroll.scroll_to_bottom();
        }
        if mode == Mode::Chat { self.expand(w, cx); } else { self.focus_input(w, cx); }
        cx.notify();
    }

    fn preview(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text.trim().to_owned();
        if text.is_empty() { return; }
        let drafts = self.drafts_mut();
        let mut draft = std::mem::take(&mut drafts.input);
        draft.text = text;
        if let Some(index) = drafts.editing.take() { drafts.previews[index] = draft; } else { drafts.previews.push(draft); }
        self.input.update(cx, |f,cx| f.set_text("",cx));
        self.scroll.scroll_to_bottom();
        self.expand(w, cx);
    }

    fn can_replace_input(&self) -> bool {
        let draft = &self.drafts().input;
        draft.text.is_empty() && draft.habit.is_none() && draft.attachment.is_none()
    }

    fn use_suggestion(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_replace_input() { return; }
        self.drafts_mut().input.text = text.to_owned();
        self.input.update(cx, |field, cx| field.set_text(text, cx));
        self.focus_input(window, cx);
        cx.notify();
    }

    fn edit(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        if !self.can_replace_input() { return; }
        let drafts = self.drafts_mut();
        drafts.input = drafts.previews[index].clone();
        drafts.editing = Some(index);
        let text = drafts.input.text.clone();
        self.input.update(cx, |f,cx| f.set_text(&text,cx));
        self.focus_input(w,cx);
        cx.notify();
    }

    fn attach(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.picking_file { return; }
        self.context_open = false;
        self.picking_file = true;
        let mode = self.mode;
        let scripted = crate::devtools::take_pick();
        let prompt = scripted.is_none().then(|| cx.prompt_for_paths(PathPromptOptions { files: true, directories: false,
            multiple: false, prompt: Some("Attach to draft".into()), extensions: None }));
        cx.spawn_in(w, async move |this,cx| {
            let files = match prompt { Some(p) => p.await.ok().and_then(Result::ok).flatten(), None => scripted };
            let _ = this.update_in(cx, |p,w,cx| {
                p.picking_file = false;
                if let Some(path) = files.and_then(|paths| paths.into_iter().next()) {
                    let drafts = match mode { Mode::Log => &mut p.log, Mode::Chat => &mut p.chat };
                    drafts.input.attachment = Some(path);
                }
                p.focus_input(w, cx);
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}

fn filename(path: &std::path::Path) -> String { path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().into_owned() }
