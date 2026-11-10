//! Workspace geometry in logical pixels. Palettes supply colors separately in `theme`.
//! Use `ui::option_row` for choices, `ui::icon_button` for single-icon actions,
//! `ui::popover` for menus and `ui::dialog` for modal surfaces. Layout and hit
//! testing read the same measurements. Inline document tools size themselves
//! to the content they edit.
use std::sync::OnceLock;

pub(crate) struct Metrics {
    pub control_height: f32,
    pub small_control_height: f32,
    pub toolbar_target: f32,
    pub inline_target: f32,
    pub control_radius: f32,
    pub nav_height: f32,
    pub nav_gap: f32,
    pub nav_radius: f32,
    pub nav_icon: f32,
    pub menu_height: f32,
    pub row_radius: f32,
    pub row_padding: f32,
    pub icon_gap: f32,
    pub menu_icon: f32,
    pub menu_radius: f32,
    pub dialog_radius: f32,
}

pub(crate) const RITUAL: Metrics = Metrics {
    control_height: 36., small_control_height: 34., toolbar_target: 36., inline_target: 28., control_radius: 12.,
    nav_height: 36., nav_gap: 2., nav_radius: 11., nav_icon: 20.,
    menu_height: 36., row_radius: 10., row_padding: 12., icon_gap: 10., menu_icon: 18.,
    menu_radius: 16., dialog_radius: 22.,
};

const QUIET: Metrics = Metrics {
    control_height: 30., small_control_height: 28., toolbar_target: 32., inline_target: 22., control_radius: 9.,
    nav_height: 30., nav_gap: 1., nav_radius: 9., nav_icon: 18.,
    menu_height: 32., row_radius: 6., row_padding: 8., icon_gap: 8., menu_icon: 16.,
    menu_radius: 12., dialog_radius: 16.,
};

static WORKSPACE: OnceLock<&Metrics> = OnceLock::new();

pub(crate) fn use_ritual() { let _ = WORKSPACE.set(&RITUAL); }
pub(crate) fn metrics() -> &'static Metrics { WORKSPACE.get().copied().unwrap_or(&QUIET) }
