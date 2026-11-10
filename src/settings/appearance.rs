//! Ritual's application appearance controls; document stylesheet editing lives in settings.rs.
use super::*;
use crate::input::{TextField, TextFieldEvent};
use workspace_theme::{Mode, Options, Selection};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group { Theme, Icons }

#[derive(Clone, Copy)]
enum Field { Fixed(Group), Light(Group), Dark(Group), DocumentFont, DocumentSize, UiFont }

impl Group {
    fn selection(self, options: &Options) -> &Selection {
        match self { Self::Theme => &options.colors, Self::Icons => &options.icons }
    }
    fn selection_mut(self, options: &mut Options) -> &mut Selection {
        match self { Self::Theme => &mut options.colors, Self::Icons => &mut options.icons }
    }
}

impl Field {
    fn value(self, options: &Options) -> String {
        match self {
            Self::Fixed(group) => group.selection(options).fixed.clone().unwrap_or_default(),
            Self::Light(group) => group.selection(options).light.clone(),
            Self::Dark(group) => group.selection(options).dark.clone(),
            Self::DocumentFont => options.document_font.clone(),
            Self::DocumentSize => options.document_size.map(|n| n.to_string()).unwrap_or_default(),
            Self::UiFont => options.ui_font.clone(),
        }
    }

    fn write(self, value: String, options: &mut Options) {
        match self {
            Self::Fixed(group) => group.selection_mut(options).fixed = Some(value),
            Self::Light(group) => group.selection_mut(options).light = value,
            Self::Dark(group) => group.selection_mut(options).dark = value,
            Self::DocumentFont => options.document_font = value,
            Self::DocumentSize => options.document_size = value.parse().ok(),
            Self::UiFont => options.ui_font = value,
        }
    }

    fn label(self, value: &str) -> String {
        if value.is_empty() {
            if matches!(self, Self::UiFont) { tr("系统字体", "System font").into() }
            else { tr("使用文档主题", "Use document theme").into() }
        } else if matches!(self, Self::DocumentSize) { format!("{value} pt") }
        else { value.into() }
    }

    fn choices(self, cx: &App) -> Vec<String> {
        match self {
            Self::DocumentFont | Self::UiFont => std::iter::once(String::new()).chain(workspace_theme::fonts(cx).iter().cloned()).collect(),
            Self::DocumentSize => std::iter::once(String::new()).chain((8..=40).map(|n| n.to_string())).collect(),
            Self::Fixed(Group::Icons) | Self::Light(Group::Icons) | Self::Dark(Group::Icons) =>
                vec![workspace_theme::RITUAL_ICONS.into(), workspace_theme::ZED_ICONS.into()],
            _ => [workspace_theme::RITUAL_LIGHT.into(), workspace_theme::RITUAL_DARK.into()].into_iter()
                .chain(workspace_theme::PALETTES.iter().map(|p| p.name.clone())).collect(),
        }
    }
}

struct Chosen(Option<String>);
pub(crate) struct Picker {
    query: Entity<TextField>,
    names: Vec<String>,
    labels: Vec<String>,
    filtered: Vec<usize>,
    active: usize,
    selected: String,
    anchor: Bounds<Pixels>,
    scroll: UniformListScrollHandle,
    _input: Subscription,
}
impl EventEmitter<Chosen> for Picker {}

impl Picker {
    fn new(field: Field, selected: String, anchor: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hint = match field {
            Field::DocumentFont | Field::UiFont => tr("搜索字体…", "Search fonts…"),
            Field::DocumentSize => tr("搜索字号…", "Search sizes…"),
            _ => tr("搜索主题…", "Search themes…"),
        };
        let names = field.choices(cx);
        let labels = names.iter().map(|n| field.label(n)).collect();
        let query = cx.new(|cx| TextField::new(hint, cx));
        let active = names.iter().position(|n| n == &selected).unwrap_or(0);
        let filtered = (0..names.len()).collect();
        let scroll = UniformListScrollHandle::new();
        scroll.scroll_to_item(active, ScrollStrategy::Top);
        query.read(cx).focus_handle(cx).focus(window);
        let input = cx.subscribe(&query, |p, input, event, cx| {
            match event {
                TextFieldEvent::Changed => {
                    let needle = input.read(cx).text.trim().to_lowercase();
                    p.filtered = p.labels.iter().enumerate().filter(|(_, s)| s.to_lowercase().contains(&needle)).map(|(i, _)| i).collect();
                    p.active = 0;
                    p.scroll.scroll_to_item(0, ScrollStrategy::Top);
                }
                TextFieldEvent::Up | TextFieldEvent::Down => {
                    p.active = if *event == TextFieldEvent::Up { p.active.saturating_sub(1) }
                        else { (p.active + 1).min(p.filtered.len().saturating_sub(1)) };
                    p.scroll.scroll_to_item(p.active, ScrollStrategy::Center);
                }
                TextFieldEvent::Submit => {
                    if let Some(i) = p.filtered.get(p.active) { cx.emit(Chosen(Some(p.names[*i].clone()))); }
                }
                TextFieldEvent::Cancel => cx.emit(Chosen(None)),
                _ => {}
            }
            cx.notify();
        });
        Self { query, names, labels, active, filtered, selected, anchor, scroll, _input: input }
    }
}

impl Render for Picker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let len = self.filtered.len();
        let height = px((len.clamp(1, 8) * 28) as f32);
        let contents = ui::popover().id("workspace-theme-options").w(px(244.)).flex().flex_col().occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().flex().h(px(36.)).items_center().gap(px(6.)).px(px(10.)).border_b_1().border_color(Theme::line_border())
                .child(ui::icon("search", 14., Theme::icon())).child(self.query.clone()))
            .when(len == 0, |d| d.child(div().px(px(12.)).py(px(9.)).text_color(Theme::text_caption()).child(tr("没有匹配项", "No matches"))))
            .when(len > 0, |d| d.child(uniform_list("workspace-theme-rows", len, cx.processor(|p, range: std::ops::Range<usize>, _, cx| {
                range.map(|index| {
                    let i = p.filtered[index];
                    let name = p.names[i].clone();
                    div().id(("workspace-theme-row", i)).w_full().h(px(28.)).px(px(4.)).px(px(8.)).flex().items_center().gap(px(8.))
                        .rounded(px(4.)).cursor_pointer().when(p.active == index, |d| d.bg(Theme::fill_hover_strong()))
                        .hover(|s| s.bg(Theme::fill_hover())).text_size(px(13.)).text_color(Theme::text())
                        .when_some(workspace_theme::palette_named(&name), |d, palette| d.child(div().size(px(12.)).rounded(px(3.))
                            .border_1().border_color(palette.accent).bg(palette.page)))
                        .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().child(p.labels[i].clone()))
                        .when(name == p.selected, |d| d.child(ui::icon("check", 13., Theme::icon())))
                        .on_mouse_down(MouseButton::Left, |_, w, _| w.prevent_default())
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(Chosen(Some(name.clone())))))
                }).collect()
            })).w_full().h(height).track_scroll(self.scroll.clone())));
        let above = self.anchor.bottom() + height + px(48.) > window.viewport_size().height && self.anchor.top() > height + px(40.);
        let (corner, y) = if above { (Corner::BottomRight, self.anchor.top() - px(3.)) }
            else { (Corner::TopRight, self.anchor.bottom() + px(3.)) };
        anchored().anchor(corner).position(point(self.anchor.right(), y)).snap_to_window_with_margin(px(6.)).child(contents)
    }
}

impl Workspace {
    fn set_workspace_option(&mut self, change: impl FnOnce(&mut Options), cx: &mut Context<Self>) {
        self.set_pref(|p| {
            let mut options = p.workspace_options();
            change(&mut options);
            p.ritual_appearance = Some(options);
        }, cx);
    }

    fn appearance_choice(&self, field: Field, title: &'static str, explanation: Option<SharedString>, options: &Options,
                         cx: &mut Context<Self>) -> AnyElement {
        let value = field.value(options);
        let anchor = std::rc::Rc::new(std::cell::Cell::new(None));
        let bounds = anchor.clone();
        let control = popup_button(SharedString::from(format!("ritual-select-{title}")), field.label(&value), false)
            .relative().child(canvas(move |b, _, _| bounds.set(Some(b)), |_, _, _, _| {}).absolute().inset_0())
            .on_click(cx.listener(move |ws, _, window, cx| {
                let Some(bounds) = anchor.get() else { return };
                ws.theme_menu = None;
                let picker = cx.new(|cx| Picker::new(field, value.clone(), bounds, window, cx));
                cx.subscribe_in(&picker, window, move |ws, _, result: &Chosen, w, cx| {
                    ws.workspace_theme_picker = None;
                    if let Some(name) = &result.0 { ws.set_workspace_option(|options| field.write(name.clone(), options), cx); }
                    ws.focus.focus(w);
                    cx.notify();
                }).detach();
                ws.workspace_theme_picker = Some(picker);
                cx.notify();
            }));
        row(title, explanation, control).into_any_element()
    }

    fn selection_rows(&self, kind: Group, options: &Options, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let selection = kind.selection(options);
        let fixed = selection.fixed.is_some();
        let title = match kind { Group::Theme => tr("主题模式", "Theme Mode"), Group::Icons => tr("图标主题模式", "Icon Theme Mode") };
        let id = if kind == Group::Theme { "color" } else { "icon" };
        let mut rows = vec![row(title, desc(tr("固定使用一种主题，或为浅色和深色分别选择。", "Use one theme or choose a pair for light and dark appearances.")),
            segmented(if kind == Group::Theme { "theme-mode" } else { "icon-mode" },
                &[tr("静态", "Static"), tr("动态", "Dynamic")], usize::from(!fixed), move |w, index, cx| {
                    let dark = system_dark(cx);
                    w.set_workspace_option(|o| kind.selection_mut(o).set_fixed(index == 0, dark), cx);
                }, cx)).into_any_element()];
        if fixed {
            rows.push(self.appearance_choice(Field::Fixed(kind), if kind == Group::Theme { tr("主题名称", "Theme Name") } else { tr("图标主题名称", "Icon Theme Name") }, None, options, cx));
        } else {
            let mode = match selection.mode { Mode::System => 0, Mode::Light => 1, Mode::Dark => 2 };
            rows.push(row(tr("模式", "Mode"), None, segmented(id, &[tr("跟随系统", "System"), tr("浅色", "Light"), tr("深色", "Dark")], mode,
                move |w, index, cx| w.set_workspace_option(|o| kind.selection_mut(o).mode = [Mode::System, Mode::Light, Mode::Dark][index], cx), cx)).into_any_element());
            let (light, dark) = if kind == Group::Theme { (tr("浅色主题", "Light Theme"), tr("深色主题", "Dark Theme")) }
                else { (tr("浅色图标主题", "Light Icon Theme"), tr("深色图标主题", "Dark Icon Theme")) };
            rows.push(self.appearance_choice(Field::Light(kind), light, None, options, cx));
            rows.push(self.appearance_choice(Field::Dark(kind), dark, None, options, cx));
        }
        rows
    }

    pub(super) fn workspace_appearance_groups(&self, cx: &mut Context<Self>) -> Div {
        let options = prefs().workspace_options();
        div().w_full().flex().flex_col().gap(px(18.))
            .child(group(Some(tr("工作区主题", "Workspace theme")), self.selection_rows(Group::Theme, &options, cx)))
            .child(group(Some(tr("文件与文件夹图标", "File and folder icons")), self.selection_rows(Group::Icons, &options, cx)))
            .child(group(Some(tr("字体", "Fonts")), vec![
                self.appearance_choice(Field::UiFont, tr("界面字体", "Interface font"), None, &options, cx),
                self.appearance_choice(Field::DocumentFont, tr("文档正文字体", "Document body font"), None, &options, cx),
                self.appearance_choice(Field::DocumentSize, tr("正文字号", "Body font size"), None, &options, cx),
            ]))
    }
}
