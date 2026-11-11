use super::{Destination, Page, view::display_amount};
use crate::{app::Workspace, input::TextField, theme::Theme, trash::page_header_with, ui::{self, TipExt, design::RITUAL}};
use gpui::{prelude::FluentBuilder, *};

fn field(input: &Entity<TextField>, height: f32, window: &Window, cx: &App) -> Stateful<Div> {
    let focus = input.read(cx).focus_handle(cx);
    div().id(("conversation-field", input.entity_id())).w_full().h(px(height)).flex_none().flex().flex_col().justify_center()
        .px(px(12.)).py(px(6.)).overflow_hidden().rounded(px(RITUAL.control_radius)).border_1()
        .border_color(if focus.is_focused(window) { Theme::primary() } else { Theme::line_border() })
        .bg(Theme::bg_body()).cursor(CursorStyle::IBeam).child(input.clone())
        .on_click(move |_, w, _| focus.focus(w))
}

impl Workspace {
    pub(crate) fn ritual_page(&self, page: Page, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let nav = self.navigation.as_ref().unwrap();
        let habits = self.habits.as_ref().unwrap().clone();
        let h = habits.read(cx);
        let index = h.spaces.clone();
        let ready = h.db.is_some() && !h.saving;
        let db_error = if h.db.is_none() { h.error.clone() } else { None };
        let scope_name = |id| index.spaces.iter().find(|s| s.id == id).map(|s| s.name.clone());
        let thread = match page { Page::Thread(id) => index.threads.iter().find(|t| t.id == id), _ => None };
        let (crumb, title) = match page {
            Page::Destination(dest) => (None, dest.label().to_owned()),
            Page::CreateSpace => (Some("Spaces".into()), "Create Space".into()),
            Page::Space(id) => (Some("Spaces".into()), scope_name(id).unwrap_or_default()),
            Page::Thread(_) | Page::Draft(_) => {
                let scope = match page { Page::Draft(id) => id, _ => thread.and_then(|t| t.space) };
                (Some(scope.and_then(scope_name).unwrap_or_else(|| "Threads".into())),
                    thread.map_or_else(|| "New thread".into(), |t| t.title.clone()))
            }
        };
        let heading = div().flex().items_center().gap(px(8.)).min_w_0().text_size(px(14.))
            .when_some(crumb, |d, crumb| d.child(div().max_w(px(160.)).truncate().text_color(Theme::text_caption()).child(crumb))
                .child(ui::icon("chevron-right", 10., Theme::text_caption())))
            .child(div().truncate().font_weight(FontWeight::MEDIUM).child(title));
        let header = page_header_with(self, heading, 22., cx).border_b_1().border_color(Theme::line_border());
        let mut body = div().flex_1().min_h_0().w_full().flex().flex_col().items_center()
            .px(px(22.)).pt(px(24.)).pb(px(24.)).text_size(px(14.));
        let column = || div().w_full().max_w(px(640.)).flex().flex_col().gap(px(12.));
        body = match page {
            Page::CreateSpace => {
                let enabled = ready && !nav.name.read(cx).text.trim().is_empty();
                body.child(column().max_w(px(460.)).mt(px(22.))
                    .child(div().text_size(px(24.)).font_weight(FontWeight::SEMIBOLD).child("Create Space"))
                    .child(div().mt(px(10.)).child("Name")).child(field(&nav.name, 38., window, cx))
                    .child(div().mt(px(6.)).child("Goal (optional)")).child(field(&nav.goal, 82., window, cx))
                    .child(div().mt(px(8.)).flex().items_center().gap(px(8.))
                        .child(ui::primary_button("save-space", "Create Space").tab_index(3).when(!enabled, |d| d.opacity(0.45))
                            .when(enabled, |d| d.on_click(cx.listener(|ws, _, w, cx| ws.save_conversation(w, cx)))))
                        .child(ui::secondary_button("cancel-space", "Cancel").tab_index(4)
                            .on_click(cx.listener(|ws, _, w, cx| ws.cancel_create_space(w, cx))))))
            }
            Page::Space(id) => {
                let space = index.spaces.iter().find(|s| s.id == id);
                body.child(div().id("space-contents").w_full().h_full().overflow_y_scroll().child(column().mx_auto()
                    .when_some(space, |d, s| d.child(div().text_size(px(26.)).font_weight(FontWeight::SEMIBOLD).child(s.name.clone()))
                        .when(!s.goal.is_empty(), |d| d.child(div().text_color(Theme::text_caption()).child(s.goal.clone()))))
                    .child(div().mt(px(14.)).flex().items_center().justify_between().child("Threads")
                        .child(ui::secondary_button("space-new-thread", "New thread")
                            .on_click(cx.listener(move |ws, _, w, cx| ws.go_ritual(Page::Draft(Some(id)), w, cx)))))
                    .children(index.threads.iter().filter(|t| t.space == Some(id)).map(|t| self.thread_row(t, true, cx)))))
            }
            Page::Draft(_) | Page::Thread(_) => {
                let enabled = ready && !nav.loading && !nav.message.read(cx).text.trim().is_empty();
                let mut history = column().mx_auto();
                if nav.loading { history = history.child("Loading conversation…"); }
                for message in &nav.messages {
                    history = history.child(div().ml_auto().max_w_full().rounded(px(RITUAL.menu_radius))
                        .bg(Theme::bg_overlay()).px(px(14.)).py(px(9.)).child(message.clone()));
                }
                let message_focus = nav.message.read(cx).focus_handle(cx);
                body.child(div().id("thread-history").flex_1().min_h_0().w_full().overflow_y_scroll()
                    .track_scroll(&nav.history_scroll).child(history))
                    .child(column().mt(px(14.)).gap(px(6.)).child(div().p(px(12.)).rounded(px(RITUAL.menu_radius)).bg(Theme::bg_overlay())
                        .child(div().id("thread-input").h(px(48.)).w_full().px(px(7.)).pt(px(3.)).overflow_hidden()
                            .cursor(CursorStyle::IBeam).child(nav.message.clone()).on_click(move |_, w, _| message_focus.focus(w)))
                        .child(div().mt(px(8.)).flex().items_center().justify_between()
                            .child(div().text_size(px(12.)).text_color(Theme::text_caption()).child("Local thread · AI offline"))
                            .child(div().id("save-message").size(px(32.)).flex().items_center().justify_center().rounded_full()
                                .tip("Save message", Some("↵")).bg(Theme::text()).when(!enabled, |d| d.opacity(0.35))
                                .child(svg().path("ritual/composer/arrow-up.svg").size(px(16.)).text_color(Theme::bg_body()))
                                .when(enabled, |d| d.tab_index(2).cursor_pointer()
                                    .on_click(cx.listener(|ws, _, w, cx| ws.save_conversation(w, cx))))))))
            }
            Page::Destination(Destination::Metrics) => {
                let mut rows = column();
                let h = habits.read(cx);
                for row in &h.habits {
                    rows = rows.child(div().flex().items_center().gap(px(12.))
                        .child(div().flex_1().truncate().child(row.habit.name.clone()))
                        .child(format!("{} {}", display_amount(row.value()), row.habit.unit)));
                }
                if h.habits.is_empty() { rows = rows.child("Tracked habits appear here."); }
                body.child(div().id("metric-list").h_full().w_full().overflow_y_scroll().child(rows.mx_auto()))
            }
            Page::Destination(Destination::Logs) => {
                let mut rows = column();
                if nav.loading { rows = rows.child("Loading logs…"); }
                else if nav.logs.is_empty() && nav.error.is_none() { rows = rows.child("No habit entries yet."); }
                else { for (name, unit, entry) in &nav.logs {
                    rows = rows.child(div().flex().flex_wrap().items_center().gap(px(12.)).border_b_1().border_color(Theme::line_border()).pb(px(9.))
                        .child(div().flex_1().child(name.clone())).child(format!("{} {}", display_amount(entry.amount), unit))
                        .child(div().text_color(Theme::text_caption()).text_size(px(12.)).child(entry.display_time())));
                } }
                body.child(div().id("log-history").h_full().w_full().overflow_y_scroll().child(rows.mx_auto()))
            }
            Page::Destination(dest) => body.child(column().child(div().text_color(Theme::text_caption())
                .child(format!("{} isn’t available in Ritual Dev yet.", dest.label())))),
        };
        if let Some(err) = nav.error.as_ref().or(db_error.as_ref()) {
            body = body.child(div().mt(px(8.)).text_color(Theme::danger()).child(err.clone()));
        }
        div().flex_1().min_w_0().h_full().flex().flex_col().child(header).child(body).into_any_element()
    }
}
