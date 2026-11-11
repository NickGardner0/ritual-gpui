//! A fixed destination rail and an independently collapsible panel.
use super::{Destination, Page};
use crate::{app::{HEADER_H, View, Workspace}, theme::{self, Theme}, ui::{self, TipExt, TitleBarExt, design::RITUAL}};
use gpui::{prelude::FluentBuilder, *};
use ritual_core::{SpaceIndex, Thread};

pub(crate) const RAIL_WIDTH: f32 = 55.;

fn glyph(path: &'static str, size: f32, color: Hsla) -> Svg {
    svg().path(path).size(px(size)).flex_none().text_color(color)
}

fn tool(id: &'static str, label: &'static str, name: &'static str) -> Stateful<Div> {
    div().id(id).tab_index(0).no_drag().size(px(29.)).flex_none().flex().items_center().justify_center()
        .rounded(px(RITUAL.row_radius)).cursor_pointer().tip(label, None)
        .hover(|s| s.bg(Theme::sidebar_hover())).child(ui::icon(name, 16., Theme::icon()))
}

fn group(label: &'static str) -> Div {
    div().h(px(30.)).ml(px(8.)).flex_none().flex().items_center().gap(px(8.))
        .font_weight(FontWeight::MEDIUM).text_color(Theme::text_caption()).child(label)
        .child(div().flex_1().h(px(1.)).bg(Theme::line_border()))
}

fn tree_row(id: String, name: &str, selected: bool) -> Stateful<Div> {
    div().id(SharedString::from(id)).tab_index(0).min_w_0().h(px(RITUAL.nav_height)).px(px(8.))
        .flex().items_center().gap(px(8.)).rounded(px(RITUAL.row_radius)).cursor_pointer()
        .tip(name, None).when(selected, |d| d.bg(Theme::sidebar_selected()))
        .hover(|s| s.bg(Theme::sidebar_hover()))
}

impl Workspace {
    fn rail_button(&self, dest: Destination, path: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let active = match self.view {
            View::Home | View::Doc | View::Folder => dest == Destination::Home,
            View::Settings | View::Trash => dest == Destination::Settings,
            View::Ritual(Page::Destination(current)) => dest == current,
            _ => false,
        };
        div().id(SharedString::from(format!("rail-{}", dest.label()))).tab_index(0).size(px(37.)).flex_none().flex()
            .items_center().justify_center().rounded(px(RITUAL.nav_radius)).cursor_pointer()
            .tip_right(dest.label(), None).when(active, |d| d.bg(Theme::sidebar_selected()))
            .when(!active, |d| d.hover(|s| s.bg(Theme::sidebar_hover())))
            .child(glyph(path, RITUAL.nav_icon, if active { Theme::text() } else { Theme::text_caption() }))
            .on_click(cx.listener(move |ws, _, w, cx| ws.go_destination(dest, w, cx)))
    }

    fn rail(&self, cx: &mut Context<Self>) -> Div {
        use Destination::*;
        let top = [(Home, "icons/home.svg"), (Metrics, "ritual/navigation/chart-no-axes-column.svg"),
            (Logs, "icons/list.svg"), (Tasks, "icons/list-todo.svg"), (Calendar, "editor-icons/calendar.svg"),
            (Reports, "icons/file-text.svg"), (Routines, "ritual/navigation/repeat-2.svg"),
            (Experiments, "ritual/navigation/flask-conical.svg")];
        div().w(px(RAIL_WIDTH)).h_full().flex_none().flex().flex_col().items_center()
            .pt(px(12.)).pb(px(8.)).border_r_1().border_color(Theme::line_border())
            .child(div().id("rail-destinations").w_full().min_h_0().flex_1().overflow_y_scroll()
                .child(div().flex().flex_col().items_center().gap(px(3.))
                    .children(top.into_iter().map(|(dest, path)| self.rail_button(dest, path, cx)))))
            .child(self.rail_button(Integrations, "ritual/navigation/plug.svg", cx))
            .child(div().h(px(3.))).child(self.rail_button(Settings, "icons/settings-gear.svg", cx))
    }

    pub(super) fn thread_row(&self, thread: &Thread, show_icon: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = thread.id;
        tree_row(format!("thread-{id}"), &thread.title, self.view == View::Ritual(Page::Thread(id)))
            .when(show_icon, |d| d.child(glyph("ritual/navigation/message-circle.svg", 15., Theme::text_caption())))
            .child(div().min_w_0().flex_1().truncate().child(thread.title.clone()))
            .on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Thread(id), w, cx)))
    }

    fn spaces_panel(&self, index: &SpaceIndex, cx: &mut Context<Self>) -> AnyElement {
        let nav = self.navigation.as_ref().unwrap();
        let mut panel = div().id("spaces-panel").flex_1().min_w_0().h_full().overflow_y_scroll()
            .pt(px(14.)).px(px(9.)).pb(px(12.)).text_size(px(13.)).line_height(px(19.))
            .child(group("Spaces").child(tool("create-space", "Create Space", "plus")
                .on_click(cx.listener(|ws, _, w, cx| ws.go_ritual(Page::CreateSpace, w, cx)))));
        if index.spaces.is_empty() {
            panel = panel.child(div().mx(px(8.)).mt(px(5.)).text_color(Theme::text_caption()).child("Create a space to group threads."));
        }
        for space in &index.spaces {
            let id = space.id;
            let open = nav.expanded.contains(&id);
            panel = panel.child(div().w_full().flex().items_center()
                .child(div().id(SharedString::from(format!("fold-{id}"))).w(px(22.)).h(px(30.)).flex_none()
                    .flex().items_center().justify_center().rounded(px(8.)).cursor_pointer().hover(|s| s.bg(Theme::sidebar_hover()))
                    .tip(if open { "Collapse space" } else { "Expand space" }, None)
                    .child(ui::icon(if open { "chevron-down" } else { "chevron-right" }, 12., Theme::text_caption()))
                    .on_click(cx.listener(move |ws, _, _, cx| {
                        let expanded = &mut ws.navigation.as_mut().unwrap().expanded;
                        if !expanded.remove(&id) { expanded.insert(id); }
                        cx.notify();
                    })))
                .child(tree_row(format!("space-{id}"), &space.name, self.view == View::Ritual(Page::Space(id)))
                    .flex_1().child(ui::icon("folder", 15., Theme::icon()))
                    .child(div().min_w_0().truncate().child(space.name.clone()))
                    .on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Space(id), w, cx)))));
            if open {
                let mut children = div().ml(px(17.)).pl(px(7.)).mb(px(5.)).border_l_1().border_color(Theme::line_border());
                let mut threads = index.threads.iter().filter(|thread| thread.space == Some(id)).peekable();
                if threads.peek().is_none() { children = children.child(self.new_thread_row(Some(id), cx)); }
                panel = panel.child(children.children(threads.map(|t| self.thread_row(t, false, cx))));
            }
        }
        let mut loose = index.threads.iter().filter(|t| t.space.is_none()).peekable();
        panel = panel.child(group("Threads").mt(px(22.)));
        if loose.peek().is_none() { panel = panel.child(self.new_thread_row(None, cx)); }
        panel.children(loose.map(|t| self.thread_row(t, true, cx))).into_any_element()
    }

    fn new_thread_row(&self, scope: Option<uuid::Uuid>, cx: &mut Context<Self>) -> Stateful<Div> {
        tree_row(format!("new-thread-{scope:?}"), "New thread", self.view == View::Ritual(Page::Draft(scope)))
            .text_color(Theme::text_caption()).child(glyph("ritual/navigation/square-pen.svg", 15., Theme::text_caption()))
            .child("New thread").on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Draft(scope), w, cx)))
    }

    pub(crate) fn ritual_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = matches!(self.view, View::Settings | View::Trash);
        let documents = self.navigation.as_ref().unwrap().documents_open;
        let head = div().h(px(HEADER_H)).w_full().flex_none().flex().items_center().title_bar()
            .border_b_1().border_color(Theme::line_border())
            .when(self.sidebar_open, |d| d.pl(px(theme::TRAFFIC_LIGHTS_W)).pr(px(10.)).gap(px(3.))
                .child(tool("hide-panel", "Hide side panel", "panel-left-close").on_click(cx.listener(|ws, _, _, cx| {
                    ws.sidebar_open = false; cx.notify();
                })))
                .when(!settings, |d| d.child(tool("panel-library", if documents { "Show Spaces and threads" } else { "Show document tree" },
                    if documents { "folder" } else { "library" }).on_click(cx.listener(|ws, _, _, cx| {
                        let nav = ws.navigation.as_mut().unwrap(); nav.documents_open = !nav.documents_open;
                        ws.batch = None; cx.notify();
                    }))))
                .child(div().flex_1())
                .child(div().id("compose-thread").tab_index(0).no_drag().size(px(32.)).rounded_full().flex_none()
                    .flex().items_center().justify_center().bg(Theme::text()).cursor_pointer().tip("New thread", None)
                    .hover(|s| s.opacity(0.85)).child(glyph("ritual/navigation/square-pen.svg", 16., Theme::bg_body()))
                    .on_click(cx.listener(|ws, _, w, cx| ws.go_ritual(Page::Draft(None), w, cx)))));
        let panel = if !self.sidebar_open { None }
            else if settings { Some(self.render_settings_sidebar(window, cx)) }
            else if documents { Some(self.render_sidebar(window, cx)) }
            else {
                let habits = self.habits.as_ref().unwrap().clone();
                let index = habits.read(cx).spaces.clone();
                Some(self.spaces_panel(&index, cx))
            };
        div().w(px(self.sidebar_inset(window))).bg(Theme::navigation_panel()).h_full().flex_none().flex().flex_col()
            .when(self.sidebar_open, |d| d.border_r_1().border_color(Theme::line_border()))
            .child(head).child(div().w_full().min_h_0().flex_1().flex().child(self.rail(cx)).children(panel))
            .into_any_element()
    }
}
