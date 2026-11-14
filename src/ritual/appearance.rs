//! Application colors follow Zed's named theme or light/dark pair. Document CSS is separate.
use gpui::{App, Hsla, rgba};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, sync::{LazyLock, OnceLock}};

pub(crate) const RITUAL_LIGHT: &str = "Ritual Light";
pub(crate) const RITUAL_DARK: &str = "Ritual Dark";
pub(crate) const RITUAL_ICONS: &str = "Ritual";
pub(crate) const ZED_ICONS: &str = "Zed";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode { #[default] System, Light, Dark }

impl Mode {
    fn dark(self, system_dark: bool) -> bool {
        match self { Self::System => system_dark, Self::Light => false, Self::Dark => true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Selection {
    pub fixed: Option<String>,
    pub mode: Mode,
    pub light: String,
    pub dark: String,
}

impl Selection {
    pub(crate) fn name(&self, system_dark: bool) -> &str {
        self.fixed.as_deref().unwrap_or_else(|| if self.mode.dark(system_dark) { &self.dark } else { &self.light })
    }

    pub(crate) fn set_fixed(&mut self, fixed: bool, system_dark: bool) {
        self.fixed = fixed.then(|| self.name(system_dark).to_owned());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Options {
    pub colors: Selection,
    pub icons: Selection,
    pub document_font: String,
    pub document_size: Option<u8>,
    pub ui_font: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            colors: Selection { fixed: None, mode: Mode::System, light: RITUAL_LIGHT.into(), dark: RITUAL_DARK.into() },
            icons: Selection { fixed: Some(RITUAL_ICONS.into()), mode: Mode::System, light: RITUAL_ICONS.into(), dark: RITUAL_ICONS.into() },
            document_font: String::new(), document_size: None, ui_font: String::new(),
        }
    }
}

pub(crate) struct Palette {
    pub name: String,
    pub dark: bool,
    pub text: Hsla, pub muted: Hsla, pub disabled: Hsla, pub placeholder: Hsla,
    pub accent: Hsla, pub icon: Hsla, pub border: Hsla, pub focus: Hsla, pub error: Hsla,
    pub page: Hsla, pub ink: Hsla, pub surface: Hsla, pub panel: Hsla, pub elevated: Hsla,
    pub hover: Hsla, pub pressed: Hsla, pub selected: Hsla,
}

pub(crate) static PALETTES: LazyLock<Vec<Palette>> = LazyLock::new(|| {
    #[derive(Deserialize)]
    struct Preset { name: String, appearance: String, style: std::collections::BTreeMap<String, String> }
    let presets: Vec<Preset> = serde_json::from_str(include_str!("../../assets/ritual/themes/palettes.json")).expect("bundled palettes");
    presets.into_iter().map(|p| {
        let color = |key: &str| rgba(u32::from_str_radix(p.style[key].trim_start_matches('#'), 16).expect("RGBA theme color")).into();
        Palette { name: p.name, dark: p.appearance == "dark", text: color("text"), muted: color("text.muted"),
            disabled: color("text.disabled"), placeholder: color("text.placeholder"), accent: color("text.accent"), icon: color("icon"),
            border: color("border"), focus: color("border.focused"), error: color("error"), page: color("editor.background"),
            ink: color("editor.foreground"), surface: color("surface.background"), panel: color("panel.background"),
            elevated: color("elevated_surface.background"), hover: color("element.hover"), pressed: color("element.active"),
            selected: color("element.selected") }
    }).collect()
});

pub(crate) fn palette_named(name: &str) -> Option<&'static Palette> { PALETTES.iter().find(|theme| theme.name == name) }

#[derive(Clone, Copy, Default, PartialEq)]
struct Active {
    palette_name: Option<&'static str>,
    zed_icons: bool,
    ui_font: Option<&'static str>,
    document_font: Option<&'static str>,
    document_size: Option<u8>,
}

thread_local! {
    static ACTIVE: Cell<Active> = Cell::new(Active::default());
    static PALETTE: Cell<Option<&'static Palette>> = const { Cell::new(None) };
}

static FONTS: OnceLock<Vec<String>> = OnceLock::new();
pub(crate) fn fonts(cx: &App) -> &'static [String] {
    FONTS.get_or_init(|| cx.text_system().all_font_names().into_iter().filter(|s| !s.starts_with('.')).collect())
}
fn installed(name: &str, cx: &App) -> Option<&'static str> {
    if name.is_empty() { return None; }
    fonts(cx).iter().find(|f| f.as_str() == name).map(String::as_str)
}

pub(super) fn production_font(cx: &App) -> &'static str {
    static FAMILY: OnceLock<&'static str> = OnceLock::new();
    FAMILY.get_or_init(|| ["FK Grotesk Neue", "FK Grotesk Neue Trial"].into_iter()
        .find_map(|name| installed(name, cx)).unwrap_or(crate::theme::UI_FONT))
}

pub(super) fn navigation_font(cx: &App) -> &'static str {
    ACTIVE.get().ui_font.unwrap_or_else(|| production_font(cx))
}

pub(crate) fn resolve(colors: &Selection, system_dark: bool) -> (Option<&'static Palette>, bool) {
    let name = colors.name(system_dark);
    let palette = palette_named(name);
    let dark = match name { RITUAL_LIGHT => false, RITUAL_DARK => true, _ => palette.map_or(colors.mode.dark(system_dark), |p| p.dark) };
    (palette, dark)
}

pub(crate) fn apply(options: &Options, system_dark: bool, cx: &App) -> bool {
    let (palette, dark) = resolve(&options.colors, system_dark);
    let active = Active { palette_name: palette.map(|p| p.name.as_str()), zed_icons: options.icons.name(system_dark) == ZED_ICONS,
        ui_font: installed(&options.ui_font, cx), document_font: installed(&options.document_font, cx),
        document_size: options.document_size.filter(|n| (8..=40).contains(n)) };
    PALETTE.set(palette);
    let changed = ACTIVE.replace(active) != active;
    crate::theme::set_dark(dark) || changed
}

pub(crate) fn palette() -> Option<&'static Palette> {
    if crate::theme::exporting_light() { None } else { PALETTE.get() }
}
pub(crate) fn zed_icons() -> bool { ACTIVE.get().zed_icons }
pub(crate) fn ui_font() -> &'static str { ACTIVE.get().ui_font.unwrap_or(crate::theme::UI_FONT) }
pub(crate) fn document_font() -> Option<&'static str> {
    ACTIVE.get().document_font
}
pub(crate) fn document_size() -> Option<f32> {
    ACTIVE.get().document_size.map(f32::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn icon_pair_and_static_color_are_independent() {
        let mut options = Options::default();
        options.colors.fixed = Some("One Dark".into());
        options.icons = Selection { fixed: None, mode: Mode::System, light: RITUAL_ICONS.into(), dark: ZED_ICONS.into() };
        assert!(resolve(&options.colors, false).1);
        assert_eq!(options.icons.name(false), RITUAL_ICONS);
        assert_eq!(options.icons.name(true), ZED_ICONS);
        options.colors.fixed = Some("A missing theme".into());
        assert!(resolve(&options.colors, true).0.is_none());
        assert!(resolve(&options.colors, true).1);
        assert_eq!(PALETTES.len(), 11);
        for palette in PALETTES.iter() {
            assert_eq!(palette.page.a, 1.);
            assert_eq!(palette.text.a, 1.);
        }
    }

    #[test]
    fn switching_modes_resolves_os_and_palette_independently() {
        let mut options = Options::default();
        options.colors.light = "One Light".into();
        options.colors.dark = "Ayu Mirage".into();
        assert_eq!(resolve(&options.colors, false).0.unwrap().name, "One Light");
        assert!(resolve(&options.colors, true).1);
        options.colors.set_fixed(true, true);
        assert_eq!(resolve(&options.colors, false).0.unwrap().name, "Ayu Mirage");
        options.colors.set_fixed(false, false);
        options.colors.mode = Mode::Light;
        assert!(!resolve(&options.colors, true).1);
        options.colors.light = "Gruvbox Dark Hard".into();
        assert!(resolve(&options.colors, false).1);
        let saved = serde_json::to_string(&options).unwrap();
        assert_eq!(serde_json::from_str::<Options>(&saved).unwrap(), options);
    }
}
