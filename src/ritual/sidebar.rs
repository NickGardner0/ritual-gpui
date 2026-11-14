//! Production Ritual's labeled navigation, with conversations below the tracking destinations.
use super::{Destination, Page};
use crate::{app::{HEADER_H, View, Workspace}, theme::{self, Theme}, ui::{self, TipExt, TitleBarExt, design::RITUAL}};
use gpui::{prelude::FluentBuilder, *};
use ritual_core::{SpaceIndex, Thread};

pub(crate) const SIDEBAR_WIDTH: f32 = 240.;
const FIRST_NAV_ROW_Y: f32 = 87.; // Window-relative top of the Index pill in production Ritual.
const NAV: [(Destination, &str); 9] = [
    (Destination::Home, "ritual/navigation/index.svg"), (Destination::Metrics, "ritual/navigation/chart-no-axes-column.svg"),
    (Destination::Logs, "editor-icons/table.svg"), (Destination::Tasks, "icons/list.svg"),
    (Destination::Calendar, "editor-icons/calendar.svg"), (Destination::Reports, "icons/file-text.svg"),
    (Destination::Routines, "ritual/navigation/repeat-2.svg"), (Destination::Experiments, "ritual/navigation/flask-conical.svg"),
    (Destination::Integrations, "ritual/navigation/plug.svg"),
];

fn glyph(path: &'static str, size: f32, color: Hsla) -> Svg {
    svg().path(path).size(px(size)).flex_none().text_color(color)
}

fn tool(id: &'static str, label: &'static str, path: &'static str) -> Stateful<Div> {
    div().id(id).tab_index(0).no_drag().size(px(28.)).flex_none().flex().items_center().justify_center()
        .rounded(px(RITUAL.row_radius)).cursor_pointer().tip(label, None)
        .hover(|s| s.bg(Theme::sidebar_hover())).child(glyph(path, 16., Theme::icon()))
}

fn row_shape(id: String) -> Stateful<Div> {
    div().id(SharedString::from(id)).tab_index(0).min_w_0().h(px(34.)).px(px(16.))
        .flex().items_center().gap(px(11.)).rounded_full().cursor_pointer()
}

fn row(id: String, selected: bool) -> Stateful<Div> {
    row_shape(id)
        .when(selected, |d| d.bg(Theme::sidebar_selected().opacity(0.38)))
        .when(!selected, |d| d.hover(|s| s.bg(Theme::sidebar_hover().opacity(0.42))))
}

impl Workspace {
    fn destination_row(&self, dest: Destination, path: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let active = match self.view {
            View::Home | View::Doc | View::Folder => dest == Destination::Home,
            View::Settings | View::Trash => dest == Destination::Settings,
            View::Ritual(Page::Destination(current)) => dest == current,
            _ => false,
        };
        let hovered = self.navigation.as_ref().is_some_and(|nav| nav.hovered_destination == Some(dest));
        let ink = Theme::navigation_ink(active || hovered);
        row_shape(format!("nav-{}", dest.label())).px(px(17.))
            .font_family(super::appearance::navigation_font(cx)).text_size(px(14.)).font_weight(FontWeight::NORMAL)
            .text_color(ink).when(active, |d| d.bg(Theme::navigation_highlight()))
            .hover(|s| s.bg(Theme::navigation_highlight()))
            .child(glyph(path, 18., ink)).child(dest.label())
            .on_hover(cx.listener(move |ws, over: &bool, _, cx| {
                let hovered = &mut ws.navigation.as_mut().unwrap().hovered_destination;
                if *over { *hovered = Some(dest); } else if *hovered == Some(dest) { *hovered = None; }
                cx.notify();
            }))
            .on_click(cx.listener(move |ws, _, w, cx| ws.go_destination(dest, w, cx)))
    }

    pub(super) fn thread_row(&self, thread: &Thread, show_icon: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = thread.id;
        row(format!("thread-{id}"), self.view == View::Ritual(Page::Thread(id))).tip(&thread.title, None)
            .when(show_icon, |d| d.child(glyph("ritual/navigation/message-circle.svg", 15., Theme::text_caption())))
            .child(div().min_w_0().flex_1().truncate().child(thread.title.clone()))
            .on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Thread(id), w, cx)))
    }

    fn threads_section(&self, index: &SpaceIndex, cx: &mut Context<Self>) -> Div {
        let nav = self.navigation.as_ref().unwrap();
        let mut section = div().w_full().min_w_0().mt(px(20.)).text_size(px(13.))
            .child(div().pl(px(16.)).pr(px(6.)).h(px(32.)).flex().items_center()
                .child(div().flex_1().text_size(px(12.)).font_weight(FontWeight::MEDIUM)
                    .text_color(Theme::text_caption()).child("Threads"))
                .child(tool("create-space", "Create space", "icons/folder-plus.svg")
                    .on_click(cx.listener(|ws, _, w, cx| ws.go_ritual(Page::CreateSpace, w, cx))))
                .child(tool("create-thread", "New thread", "icons/plus.svg")
                    .on_click(cx.listener(|ws, _, w, cx| ws.go_ritual(Page::Draft(None), w, cx)))));
        if index.spaces.is_empty() && index.threads.is_empty() {
            return section.child(div().px(px(16.)).pt(px(3.)).text_color(Theme::text_caption()).child("No threads yet"));
        }
        section = section.children(index.threads.iter().filter(|t| t.space.is_none()).map(|t| self.thread_row(t, true, cx)));
        for space in &index.spaces {
            let id = space.id;
            let open = nav.expanded.contains(&id);
            section = section.child(div().w_full().flex().items_center().pr(px(7.))
                .child(row(format!("space-{id}"), self.view == View::Ritual(Page::Space(id)))
                    .flex_1().tip(&space.name, None).child(ui::icon("folder", 15., Theme::icon()))
                    .child(div().min_w_0().truncate().child(space.name.clone()))
                    .on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Space(id), w, cx))))
                .child(div().id(SharedString::from(format!("fold-{id}"))).tab_index(0).size(px(26.)).flex_none()
                    .flex().items_center().justify_center().rounded_full().cursor_pointer().hover(|s| s.bg(Theme::sidebar_hover()))
                    .tip(if open { "Collapse space" } else { "Expand space" }, None)
                    .child(ui::icon(if open { "chevron-down" } else { "chevron-right" }, 12., Theme::text_caption()))
                    .on_click(cx.listener(move |ws, _, _, cx| {
                        let expanded = &mut ws.navigation.as_mut().unwrap().expanded;
                        if !expanded.remove(&id) { expanded.insert(id); }
                        cx.notify();
                    }))));
            if open {
                let mut threads = index.threads.iter().filter(|thread| thread.space == Some(id)).peekable();
                let children = div().ml(px(24.));
                section = section.child(if threads.peek().is_none() {
                    children.child(row(format!("draft-{id}"), self.view == View::Ritual(Page::Draft(Some(id))))
                        .text_color(Theme::text_caption()).child("New thread")
                        .on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Draft(Some(id)), w, cx))))
                } else { children.children(threads.map(|t| self.thread_row(t, false, cx))) });
            }
        }
        section
    }

    pub(crate) fn ritual_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = matches!(self.view, View::Settings | View::Trash);
        let documents = self.navigation.as_ref().unwrap().documents_open;
        let head = div().h(px(HEADER_H)).w_full().flex_none().flex().items_center().title_bar()
            .pl(px(theme::TRAFFIC_LIGHTS_W)).pr(px(9.)).gap(px(2.))
            .child(tool("hide-sidebar", "Hide sidebar", "icons/panel-left-close.svg").on_click(cx.listener(|ws, _, _, cx| {
                ws.sidebar_open = false; cx.notify();
            })))
            .child(div().flex_1())
            .when(!settings, |d| d.child(tool("panel-library", if documents { "Show Ritual navigation" } else { "Show document tree" },
                if documents { "icons/arrow-left.svg" } else { "icons/library.svg" }).on_click(cx.listener(|ws, _, _, cx| {
                    let nav = ws.navigation.as_mut().unwrap(); nav.documents_open = !nav.documents_open;
                    ws.batch = None; cx.notify();
                }))))
            .child(tool("sidebar-search", "Search documents", "icons/search.svg").on_click(cx.listener(|ws, _, w, cx| ws.open_search(w, cx))))
            .child(tool("compose-thread", "New thread", "ritual/navigation/square-pen.svg")
                .on_click(cx.listener(|ws, _, w, cx| ws.go_ritual(Page::Draft(None), w, cx))));
        let body = if settings { self.render_settings_sidebar(window, cx) }
            else if documents { self.render_sidebar(window, cx) }
            else {
                let index = self.habits.as_ref().unwrap().read(cx).spaces.clone();
                div().id("ritual-navigation").size_full().overflow_y_scroll().child(div().px(px(9.)).pt(px(FIRST_NAV_ROW_Y - HEADER_H)).pb(px(12.))
                    .child(div().flex().flex_col().gap(px(5.))
                        .children(NAV.into_iter().map(|(dest, path)| self.destination_row(dest, path, cx))))
                    .child(self.threads_section(&index, cx))).into_any_element()
            };
        div().w(px(self.sidebar_w(window))).bg(Theme::bg_body()).h_full().flex_none().flex().flex_col()
            .font_family(theme::ui_font()).text_size(px(14.)).text_color(Theme::sidebar_text()).child(head)
            .child(div().w_full().min_h_0().flex_1().overflow_hidden().child(body))
            .when(!settings && self.batch.is_none(), |d| d.child(div().flex_none().px(px(9.)).py(px(14.))
                .child(self.destination_row(Destination::Settings, "icons/settings-gear.svg", cx))))
            .into_any_element()
    }
}
