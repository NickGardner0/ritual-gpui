use crate::ui::design::RITUAL;
use super::*;
use crate::input;
use crate::{theme::{self, Theme}, ui::{self, TipExt}};
use gpui::prelude::FluentBuilder;

const ACTION_SIZE: f32 = 32.;

fn glyph(name: &'static str, size: f32, color: Hsla) -> Svg {
    match name {
        "plus" | "arrow-up" | "audio-lines" | "maximize-2" => svg().path(format!("ritual/composer/{name}.svg")).size(px(size)).text_color(color),
        "paperclip" => svg().path("editor-icons/paperclip.svg").size(px(size)).text_color(color),
        _ => ui::icon(name, size, color),
    }
}

fn tab_stop(button: Stateful<Div>, index: isize) -> Stateful<Div> {
    button.tab_index(index).focus(|style| style.border_1().border_color(Theme::primary()))
}

fn control(id: &'static str, icon: &'static str, hint: &'static str) -> Stateful<Div> {
    tab_stop(div().id(id), 2).size(px(RITUAL.control_height)).flex_none().flex().items_center().justify_center().rounded(px(RITUAL.control_radius))
        .cursor_pointer().hover(|s| s.bg(Theme::fill_hover_strong())).tip(hint,None)
        .child(glyph(icon, 15., Theme::text_caption()))
}

impl Composer {
    fn input_panel(&self, colors: &ComposerColors, cx: &mut Context<Self>) -> Div {
        let draft = &self.drafts().input;
        let input = self.input.read(cx);
        let has_text = !input.text.trim().is_empty();
        let multiline = input.text.contains('\n') || input.text.chars().count() > 120;
        let modes = [Mode::Log, Mode::Chat].map(|mode| {
            let active = self.mode == mode;
            tab_stop(div().id(mode.label()), 2).h(px(28.)).px(px(14.)).border_1().border_color(transparent_black())
                .rounded_full().flex().items_center().justify_center().cursor_pointer().text_size(px(13.))
                .text_color(if active { colors.text } else { colors.secondary })
                .when(active, |button| button.bg(colors.mode_selected))
                .when(!active, |button| button.hover(|s| s.text_color(colors.text)))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(cx.listener(move |composer, _, window, cx| composer.switch_mode(mode, window, cx)))
                .child(mode.label())
        });

        let add = tab_stop(div().id("composer-context"), 2).size(px(ACTION_SIZE)).rounded_full().flex_none()
            .flex().items_center().justify_center().cursor_pointer().tip("Add context", None)
            .bg(if self.context_open { colors.text.opacity(0.11) } else { colors.action_fill })
            .hover(|s| s.bg(colors.text.opacity(0.11)))
            .child(glyph("plus", 18., colors.icon))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(|composer, _, window, cx| {
                composer.context_open = !composer.context_open;
                composer.focus_input(window, cx);
                cx.notify();
            }));
        let voice = div().id("composer-voice-preview").size(px(ACTION_SIZE)).rounded_full().flex_none()
            .flex().items_center().justify_center().bg(colors.action_fill)
            .tip("Voice input is unavailable in this preview", None).child(glyph("audio-lines", 18., colors.icon));
        let send = div().id("composer-preview").size(px(ACTION_SIZE)).rounded_full().flex_none()
            .flex().items_center().justify_center().bg(if has_text { colors.text } else { colors.send_disabled })
            .tip("Preview draft", Some("↵")).child(glyph("arrow-up", 16., colors.on_send))
            .when(has_text, |button| tab_stop(button, 3).cursor_pointer()
                .on_click(cx.listener(|composer, _, window, cx| composer.preview(window, cx))));

        let toolbar = div().flex().flex_wrap().items_center().justify_between().gap(px(8.))
            .child(div().flex().items_center().gap(px(8.)).child(add)
                .child(div().h(px(ACTION_SIZE)).flex().items_center().rounded_full().p(px(2.))
                    .bg(colors.mode_track).children(modes)))
            .child(div().flex().flex_1().min_w(px(2. * ACTION_SIZE + 8.)).items_center().justify_end().gap(px(8.)).child(voice).child(send));

        // Desktop's idle panel is 104pt: 12 top + 50 writing area + 32 actions + 10 bottom.
        div().relative().w_full().flex_none().flex().flex_col().rounded(px(RITUAL.dialog_radius)).bg(colors.surface)
            .border_1().border_color(Theme::line_border())
            .pl(px(15.)).pr(px(11.)).pt(px(11.)).pb(px(9.))
            .when(self.context_open, |panel| panel.child(self.context_list(cx)))
            .when(draft.habit.is_some() || draft.attachment.is_some(), |panel| panel.child(
                div().flex().flex_wrap().gap(px(4.)).pb(px(6.))
                    .when_some(draft.habit.as_ref(), |row, (_, name)| row.child(chip("draft-habit", "list-todo", name.clone())
                        .on_click(cx.listener(|composer, _, window, cx| {
                            composer.drafts_mut().input.habit = None;
                            composer.focus_input(window, cx);
                            cx.notify();
                        }))))
                    .when_some(draft.attachment.as_ref(), |row, path| row.child(chip("draft-file", "paperclip", filename(path))
                        .on_click(cx.listener(|composer, _, window, cx| {
                            composer.drafts_mut().input.attachment = None;
                            composer.focus_input(window, cx);
                            cx.notify();
                        })))) ))
            .child(div().id("composer-input-area").w_full().h(px(if multiline { 70. } else { 50. }))
                .pl(px(4.)).pr(px(8.)).pt(px(2.)).flex().overflow_y_scroll()
                .on_click(cx.listener(|composer, _, window, cx| composer.focus_input(window, cx))).child(self.input.clone()))
            .child(toolbar)
    }

    fn context_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut items = div().id("composer-context-list").max_h(px(142.)).overflow_y_scroll().pb(px(5.))
            .child(tab_stop(crate::ui::option_row("attach-file"), 2)
                .child(glyph("paperclip",RITUAL.menu_icon,Theme::icon())).child("Attach a file…")
                .on_click(cx.listener(|p,_,w,cx|p.attach(w,cx))));
        if !self.expanded {
            items = items.child(tab_stop(crate::ui::option_row("composer-expand"), 2)
                .child(glyph("maximize-2", RITUAL.menu_icon, Theme::icon())).child("Expand composer")
                .on_click(cx.listener(|composer, _, window, cx| composer.expand(window, cx))));
        }
        let rows = &self.habits.read(cx).habits;
        if !rows.is_empty() {
            items = items.child(div().mx(px(6.)).mt(px(4.)).mb(px(2.)).text_size(px(11.)).text_color(Theme::text_caption()).child("Tracked habits"));
        }
        for h in rows {
            let name = h.habit.name.clone();
            let id = h.habit.id;
            items = items.child(tab_stop(crate::ui::option_row(SharedString::from(id.to_string())), 2)
                .child(glyph("list-todo",RITUAL.menu_icon,Theme::icon()))
                .child(name.clone()).on_click(cx.listener(move |p,_,w,cx| {
                    p.drafts_mut().input.habit=Some((id,name.clone())); p.context_open=false; p.focus_input(w,cx); cx.notify();
                })));
        }
        items.mb(px(6.)).border_b_1().border_color(Theme::line_border())
    }

    fn thread(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let drafts = self.drafts();
        let mut list = div().id("composer-drafts").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.scroll).px(px(18.)).py(px(12.));
        let can_edit = self.can_replace_input();
        if drafts.previews.is_empty() {
            let suggestions: Vec<_> = self.habits.read(cx).habits.iter().take(3).map(|h| {
                if self.mode == Mode::Log { format!("2 {} of {}", h.habit.unit, h.habit.name) }
                else { format!("Summarize my {} entries.", h.habit.name) }
            }).collect();
            if !suggestions.is_empty() {
                list = list.child(div().pb(px(5.)).text_size(px(11.)).text_color(Theme::text_caption()).child("Try a draft"));
            }
            for (i,suggestion) in suggestions.into_iter().enumerate() {
                list = list.child(div().id(("try-draft",i)).px(px(6.)).h(px(RITUAL.menu_height)).flex().items_center().gap(px(RITUAL.icon_gap))
                    .rounded(px(RITUAL.row_radius)).when(can_edit, |d|tab_stop(d, 2).cursor_pointer().hover(|s|s.bg(Theme::fill_hover()))
                        .on_click(cx.listener({
                            let suggestion = suggestion.clone();
                            move |composer, _, window, cx| composer.use_suggestion(&suggestion, window, cx)
                        }))).child(glyph("pen-line",13.,Theme::text_placeholder())).child(suggestion));
            }
        }
        for (index, draft) in drafts.previews.iter().enumerate() {
            let label = if self.mode == Mode::Chat { "Chat draft" } else { "Log draft" };
            let heading = div().flex().items_center().justify_between()
                .child(div().text_size(px(11.)).text_color(Theme::text_caption()).child(label))
                .when(can_edit, |row| {
                    row.child(tab_stop(div().id(("edit-draft", index)), 2).px(px(6.)).rounded(px(4.)).text_size(px(12.))
                        .cursor_pointer().hover(|s| s.bg(Theme::fill_hover())).child("Edit")
                        .on_click(cx.listener(move |p, _, w, cx| p.edit(index, w, cx))))
                });
            let item = div().py(px(8.)).border_b_1().border_color(Theme::line_border()).child(heading)
                .child(div().w_full().pt(px(4.)).text_size(px(14.)).child(draft.text.clone()))
                .when_some(draft.habit.as_ref(), |d, (_, name)| d.child(metadata("list-todo", name.clone())))
                .when_some(draft.attachment.as_ref(), |d, file| d.child(metadata("paperclip", filename(file))));
            list = list.child(item);
        }
        list
    }
}

fn chip(id: &'static str, icon: &'static str, name: String) -> Stateful<Div> {
    tab_stop(div().id(id), 2).h(px(22.)).max_w_full().px(px(6.)).rounded(px(4.)).bg(Theme::fill_hover_strong())
        .flex().gap(px(4.)).items_center().text_size(px(12.)).cursor_pointer().tip("Remove from draft",None)
        .child(glyph(icon,12.,Theme::text_caption())).child(div().min_w_0().overflow_hidden().whitespace_nowrap().child(name))
        .child(glyph("x",10.,Theme::text_caption()))
}
fn metadata(icon: &'static str, name: String) -> Div {
    div().mt(px(3.)).flex().items_center().gap(px(4.)).text_size(px(12.)).text_color(Theme::text_caption())
        .child(glyph(icon,12.,Theme::text_caption())).child(name)
}

impl Render for Composer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = ComposerColors::current();
        let input = self.input.read(cx);
        let family = input.font.unwrap_or(theme::ui_font());
        if input.color != colors.text || input.placeholder_color != colors.muted {
            self.input.update(cx, |field, cx| {
                field.color = colors.text;
                field.placeholder_color = colors.muted;
                cx.notify();
            });
        }
        let base = div().id("ritual-composer").track_focus(&self.focus).tab_group().key_context("RitualComposer")
            .on_action(cx.listener(|p, _: &input::Cancel, window, cx| p.escape(window, cx)))
            .on_action(cx.listener(|p, _: &input::Tab, window, cx| p.advance_focus(false, window, cx)))
            .on_action(cx.listener(|p, _: &input::ShiftTab, window, cx| p.advance_focus(true, window, cx)))
            .w_full().max_w(px(660.)).min_w_0()
            .font_family(family).text_size(px(13.)).text_color(Theme::text())
            .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation());
        if !self.expanded { return base.child(self.input_panel(&colors, cx)); }
        let height = (f32::from(w.viewport_size().height)-32.).min(360.);
        base.mx(px(16.)).h(px(height)).rounded(px(RITUAL.dialog_radius)).flex().flex_col().occlude().overflow_hidden()
            .border_1().border_color(Theme::line_border()).bg(Theme::bg_body()).shadow(theme::shadow_dialog())
            .child(div().h(px(40.)).flex_none().px(px(12.)).border_b_1().border_color(Theme::line_border())
                .flex().items_center().gap(px(8.)).child(glyph("pen-line",16.,Theme::icon()))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(if self.mode == Mode::Chat {"Ritual Chat"} else {"Log composer"}))
                .child(div().flex_1()).child(div().text_size(px(11.)).text_color(Theme::text_caption()).child("Session preview"))
                .child(control("close-composer","x","Close composer").on_click(cx.listener(|p,_,w,cx| p.collapse(w,cx)))))
            .child(self.thread(cx))
            .child(div().flex_none().p(px(10.)).pt(px(4.)).child(self.input_panel(&colors, cx)))
    }
}
