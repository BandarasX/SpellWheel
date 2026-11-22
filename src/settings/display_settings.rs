use super::panel_model::{self, Draft, Mode, Tab};
use super::{Settings, SettingsContext};
use crate::keyboard::settings_binding::Binding;
use crate::keyboard::settings_shortcut::SettingsShortcut;
use imgui::{
    Condition, FontConfig, FontSource, ItemHoveredFlags, Key, MouseButton, SliderFlags, StyleColor,
    StyleVar, TreeNodeFlags, Ui,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use toml_edit::Value;

pub const PANEL_FONT_SIZE: f32 = 34.0;

pub fn panel_font_source() -> FontSource<'static> {
    FontSource::TtfData {
        data: include_bytes!("../../assets/AtkinsonHyperlegible-Regular.ttf"),
        size_pixels: PANEL_FONT_SIZE,
        config: Some(FontConfig {
            name: Some("Atkinson Hyperlegible".into()),
            ..FontConfig::default()
        }),
    }
}

// Scope the gold theme to the settings panel, including its popups and navigation state.
const PANEL_COLORS: [(StyleColor, [f32; 4]); 32] = [
    (StyleColor::Text, [0.96, 0.92, 0.83, 1.0]),
    (StyleColor::TextDisabled, [0.61, 0.56, 0.47, 1.0]),
    (StyleColor::WindowBg, [0.075, 0.064, 0.049, 1.0]),
    (StyleColor::PopupBg, [0.10, 0.083, 0.059, 1.0]),
    (StyleColor::Border, [0.48, 0.38, 0.22, 0.65]),
    (StyleColor::TitleBg, [0.17, 0.13, 0.074, 1.0]),
    (StyleColor::TitleBgActive, [0.27, 0.20, 0.10, 1.0]),
    (StyleColor::FrameBg, [0.18, 0.15, 0.10, 1.0]),
    (StyleColor::FrameBgHovered, [0.29, 0.22, 0.12, 1.0]),
    (StyleColor::FrameBgActive, [0.36, 0.27, 0.14, 1.0]),
    (StyleColor::Button, [0.29, 0.22, 0.12, 1.0]),
    (StyleColor::ButtonHovered, [0.39, 0.29, 0.15, 1.0]),
    (StyleColor::ButtonActive, [0.46, 0.35, 0.19, 1.0]),
    (StyleColor::Header, [0.27, 0.21, 0.12, 1.0]),
    (StyleColor::HeaderHovered, [0.37, 0.28, 0.15, 1.0]),
    (StyleColor::HeaderActive, [0.42, 0.32, 0.17, 1.0]),
    (StyleColor::Tab, [0.19, 0.15, 0.085, 1.0]),
    (StyleColor::TabHovered, [0.38, 0.29, 0.15, 1.0]),
    (StyleColor::TabActive, [0.33, 0.25, 0.13, 1.0]),
    (StyleColor::TabUnfocused, [0.15, 0.12, 0.07, 1.0]),
    (StyleColor::TabUnfocusedActive, [0.25, 0.19, 0.10, 1.0]),
    (StyleColor::CheckMark, [0.89, 0.74, 0.45, 1.0]),
    (StyleColor::SliderGrab, [0.72, 0.57, 0.32, 1.0]),
    (StyleColor::SliderGrabActive, [0.92, 0.77, 0.49, 1.0]),
    (StyleColor::NavHighlight, [0.93, 0.77, 0.44, 1.0]),
    (StyleColor::TextSelectedBg, [0.64, 0.49, 0.24, 0.60]),
    (StyleColor::ResizeGrip, [0.72, 0.57, 0.32, 0.50]),
    (StyleColor::ResizeGripHovered, [0.82, 0.67, 0.40, 0.85]),
    (StyleColor::ResizeGripActive, [0.92, 0.77, 0.49, 1.0]),
    (StyleColor::Separator, [0.48, 0.38, 0.22, 0.65]),
    (StyleColor::SeparatorHovered, [0.72, 0.57, 0.32, 0.85]),
    (StyleColor::SeparatorActive, [0.92, 0.77, 0.49, 1.0]),
];

static IS_OPEN: AtomicBool = AtomicBool::new(false);
pub fn display_settings_open() -> bool {
    IS_OPEN.load(Ordering::Acquire)
}

/// This predicate runs on the window thread; activation still comes exclusively from the input task.
pub fn input_message_filter(message: u32, key: usize, _lparam: isize) -> bool {
    if display_settings_open() {
        return super::input_policy::blocks_message(true, false, 0, message, key);
    }
    let alt_f4 = crate::keyboard::settings_shortcut::ALT_F4_BINDING.load(Ordering::Acquire);
    if !alt_f4 || key != 0x73 || !matches!(message, 0x104 | 0x105) {
        return false;
    }
    let modifiers = crate::keyboard::settings_shortcut::modifier_mask();
    super::input_policy::blocks_message(false, alt_f4, modifiers, message, key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modal {
    Close,
    Reset,
    Conflict,
    Reload,
}
impl Modal {
    fn title(self) -> &'static str {
        match self {
            Self::Close => "Unsaved settings",
            Self::Reset => "Reset to Defaults",
            Self::Conflict => "Settings changed on disk",
            Self::Reload => "Reload disk settings",
        }
    }
}

pub struct DisplaySettings {
    pub reload_stage: u8,
    pub shortcut: SettingsShortcut,
    pub shown: bool,
    draft: Draft,
    modal: Option<Modal>,
    open_modal: bool,
    reset_current: bool,
    feedback: String,
    read_error: bool,
    disk_check: Instant,
    // Previous viewport, panel position and size, for keeping the window reachable on resize.
    window: Option<([f32; 2], [f32; 2], [f32; 2])>,
    #[cfg(test)]
    bounds: Vec<(String, [f32; 2], [f32; 2])>,
    #[cfg(test)]
    sections: Vec<(String, bool, bool)>,
}
impl DisplaySettings {
    pub fn new() -> Self {
        let runtime = Settings::read_or_default();
        Self {
            reload_stage: 0,
            shortcut: SettingsShortcut::new(),
            shown: false,
            draft: Draft::new(None, runtime),
            modal: None,
            open_modal: false,
            #[cfg(test)]
            bounds: Vec::new(),
            #[cfg(test)]
            sections: Vec::new(),
            reset_current: true,
            feedback: String::new(),
            read_error: false,
            disk_check: Instant::now(),
            window: None,
        }
    }
    fn open(&mut self) {
        let runtime = SettingsContext::begin_editing();
        self.read_error = match panel_model::read_disk(&crate::paths::settings()) {
            Ok(disk) => {
                self.draft.reload_disk(disk, runtime);
                false
            }
            Err(e) => {
                self.feedback = e;
                true
            }
        };
        if !self.read_error && self.draft.error.is_none() {
            SettingsContext::committed(&self.draft.last_valid);
        }
        self.shown = true;
    }
    fn close(&mut self) {
        self.shown = false;
        self.modal = None;
        self.draft.close_pending = false;
        SettingsContext::end_editing();
    }
    fn prompt(&mut self, modal: Modal) {
        self.modal = Some(modal);
        self.open_modal = true;
    }
    fn request_close(&mut self) {
        if self.draft.request_close() {
            self.close();
        } else {
            self.prompt(Modal::Close);
        }
    }
    pub fn update(&mut self) {
        let mut toggled = false;
        #[cfg(feature = "settings-diagnostics")]
        let before = self.shown;
        let activations = self.shortcut.update_visibility(&mut toggled);
        if activations != 0 && toggled && self.modal.is_none() {
            if self.shown {
                self.request_close();
            } else {
                self.open();
            }
        }
        #[cfg(feature = "settings-diagnostics")]
        if activations != 0 {
            hudhook::settings_diagnostics::event(format_args!(
                "editor_toggle reason=shortcut_generation listener={:x} before={} after={} activations={} generation={} dirty={} modal={}",
                self.shortcut.diagnostic_id(),
                before,
                self.shown,
                activations,
                self.shortcut.generation(),
                self.draft.dirty(),
                self.modal.is_some()
            ));
        }
        IS_OPEN.store(self.shown, Ordering::Release);
        if self.shown && self.disk_check.elapsed() >= Duration::from_secs(1) {
            self.disk_check = Instant::now();
            match panel_model::read_disk(&crate::paths::settings()) {
                Ok(disk) => self.draft.observe_disk(disk),
                Err(e) => self.feedback = e,
            }
        }
    }
    fn preview(&mut self) {
        if self.draft.validate() {
            SettingsContext::preview(&self.draft.last_valid);
        }
    }
    fn discard(&mut self) {
        if self.read_error {
            return;
        }
        self.draft.discard();
        SettingsContext::discard_preview();
        // Saved text can include valid visual values from a missing-file new-config draft.
        if self.draft.error.is_none() {
            SettingsContext::preview(&self.draft.last_valid);
        }
        self.feedback = "Draft discarded; last saved settings restored.".into();
    }
    fn save(&mut self, overwrite: bool) -> bool {
        if self.read_error {
            return false;
        }
        match self.draft.save(&crate::paths::settings(), overwrite) {
            Ok(settings) => {
                SettingsContext::committed(&settings);
                self.feedback = "Saved. Live settings applied.".into();
                true
            }
            Err(e) => {
                self.feedback = e;
                if self.draft.external_change {
                    self.prompt(Modal::Conflict);
                }
                false
            }
        }
    }
    fn set(&mut self, key: &str, value: impl Into<Value>) {
        if let Err(e) = self.draft.set(key, value.into()) {
            self.feedback = e;
        }
        if self.draft.error.is_none() {
            SettingsContext::preview(&self.draft.last_valid);
        }
    }
    fn button(&mut self, ui: &Ui, label: &str) -> bool {
        let clicked = ui.button(label);
        #[cfg(test)]
        self.bounds
            .push((label.into(), ui.item_rect_min(), ui.item_rect_max()));
        clicked
    }
    fn section(&mut self, ui: &Ui, label: &str, flags: TreeNodeFlags) -> bool {
        let expanded = ui.collapsing_header(label, flags);
        #[cfg(test)]
        self.sections
            .push((label.into(), expanded, ui.is_item_focused()));
        #[cfg(test)]
        self.bounds
            .push((label.into(), ui.item_rect_min(), ui.item_rect_max()));
        expanded
    }
    fn number(&mut self, ui: &Ui, label: &str, key: &str, value: f32, base: f32, help: &str) {
        let mut displayed = panel_model::to_percent(value, base);
        if ui
            .input_float(label, &mut displayed)
            .display_format("%.7g")
            .build()
        {
            self.set(
                key,
                panel_model::float_value(panel_model::from_percent(displayed, base)),
            );
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(help);
        }
    }
    fn percent_slider(
        &mut self,
        ui: &Ui,
        label: &str,
        key: &str,
        value: f32,
        base: f32,
        range: [f32; 2],
        help: &str,
    ) {
        let _id = ui.push_id(key);
        ui.text(label);
        let original = panel_model::to_percent(value, base);
        let mut percent = original;
        let width = ui.content_region_avail()[0];
        let entry_width = (ui.current_font_size() * 6.0).max(64.0);
        ui.set_next_item_width((width - entry_width - ui.clone_style().item_spacing[0]).max(1.0));
        let changed_slider = ui
            .slider_config("##slider", range[0], range[1])
            .display_format("")
            .flags(SliderFlags::NO_INPUT | SliderFlags::NO_ROUND_TO_FORMAT)
            .build(&mut percent);
        if changed_slider {
            // Native ImGui keyboard movement scales with the range. Use an adjacent
            // half-point instead, even when exact entry left the handle between ticks.
            let ticks = original * 2.0;
            let nearest_tick = ticks.round();
            let ticks = if (ticks - nearest_tick).abs() < 0.0001 {
                nearest_tick
            } else {
                ticks
            };
            percent =
                if !ui.is_mouse_down(MouseButton::Left) && ui.is_key_pressed(Key::LeftArrow) {
                    (ticks.ceil() - 1.0) / 2.0
                } else if !ui.is_mouse_down(MouseButton::Left) && ui.is_key_pressed(Key::RightArrow)
                {
                    (ticks.floor() + 1.0) / 2.0
                } else {
                    (percent * 2.0).round() / 2.0
                }
                .clamp(range[0], range[1]);
        }
        #[cfg(test)]
        self.bounds
            .push((key.into(), ui.item_rect_min(), ui.item_rect_max()));
        if ui.is_item_hovered() {
            ui.tooltip_text(help);
        }
        ui.same_line();
        ui.set_next_item_width(entry_width);
        let changed_entry = ui
            .input_float("##exact-percent", &mut percent)
            .display_format("%.2f")
            .build();
        #[cfg(test)]
        self.bounds.push((
            format!("{key} exact"),
            ui.item_rect_min(),
            ui.item_rect_max(),
        ));
        if ui.is_item_hovered() {
            ui.tooltip_text(
                "Enter an exact percentage, including values outside the slider range.",
            );
        }
        if changed_slider || changed_entry {
            self.set(
                key,
                panel_model::float_value(panel_model::from_percent(percent, base)),
            );
        }
    }
    fn choice(
        &mut self,
        ui: &Ui,
        label: &str,
        key: &str,
        current: &str,
        values: &[&str],
        labels: &[&str],
    ) {
        let mut index = values
            .iter()
            .position(|s| s.eq_ignore_ascii_case(current))
            .unwrap_or(0);
        if ui.combo_simple_string(label, &mut index, labels) {
            self.set(key, values[index]);
        }
    }
    fn appearance(&mut self, ui: &Ui, s: &Settings) {
        if self.section(ui, "Wheel layout", TreeNodeFlags::DEFAULT_OPEN) {
            self.choice(
                ui,
                "Style",
                "style",
                &s.style,
                &["pretty", "simple"],
                &["Pretty", "Simple"],
            );
            let mut show_ring = s.keyboard_mouse_inner_ring;
            const RING_LABEL: &str = "Show inner ring with keyboard and mouse";
            if ui.checkbox(RING_LABEL, &mut show_ring) {
                self.set("keyboard_mouse_inner_ring", show_ring);
            }
            #[cfg(test)]
            self.bounds
                .push((RING_LABEL.into(), ui.item_rect_min(), ui.item_rect_max()));
            self.percent_slider(
                ui,
                "Icon size (%)",
                "icon_scale_multiplier",
                s.icon_scale_multiplier,
                0.15,
                [50.0, 200.0],
                "100% = multiplier 0.15. Radius must exceed icon multiplier + 0.01.",
            );
            self.percent_slider(ui, "Wheel radius (%)", "radius_multiplier", s.radius_multiplier, 0.3, [50.0, 200.0], "100% = multiplier 0.3. Radius must exceed icon multiplier + 0.01. Supported up to 8.");
        }
        if self.section(ui, "Selection effect", TreeNodeFlags::empty()) {
            if s.style.eq_ignore_ascii_case("simple") {
                ui.text_wrapped("Pretty style only. Values are retained while Simple is active.");
            }
            let _disabled = ui.begin_disabled(s.style.eq_ignore_ascii_case("simple"));
            self.percent_slider(
                ui,
                "Enlargement (%)",
                "highlight_expansion",
                s.highlight_expansion,
                1.0,
                [0.0, 100.0],
                "15% = multiplier 0.15. Supported 0 to 800%.",
            );
            self.number(
                ui,
                "Transition duration (seconds)",
                "highlight_time",
                s.highlight_time,
                100.0,
                "0 is instant; supported 0 to 60 seconds.",
            );
        }
        if self.section(ui, "Labels and readability", TreeNodeFlags::empty()) {
            self.choice(
                ui,
                "Name placement",
                "item_names",
                &s.item_names,
                &["show", "center", "hide"],
                &["Each icon", "Selected in center", "Hidden"],
            );
            self.percent_slider(
                ui,
                "Font scale (%)",
                "font_scale_multiplier",
                s.font_scale_multiplier,
                1.0,
                [50.0, 200.0],
                "100% = multiplier 1.0. Supported 1 to 1600%.",
            );
            let mut shadows = s.text_shadows;
            if ui.checkbox("Text shadows", &mut shadows) {
                self.set("text_shadows", shadows);
            }
        }
        if self.section(ui, "Colors and opacity", TreeNodeFlags::empty()) {
            ui.text_wrapped(
                "RGBA tints, including opacity. Colors use the existing HDR output pipeline.",
            );
            for (label, key, mut color) in [
                ("Icon tint", "icon_tint", s.icon_tint),
                (
                    "Icon-background tint",
                    "icon_background_tint",
                    s.icon_background_tint,
                ),
                ("Label tint", "name_tint", s.name_tint),
            ] {
                let _disabled = ui.begin_disabled(
                    key == "icon_background_tint" && s.style.eq_ignore_ascii_case("simple"),
                );
                if ui
                    .color_edit4_config(label, &mut color)
                    .alpha_bar(true)
                    .flags(imgui::ColorEditFlags::FLOAT)
                    .build()
                {
                    self.set(
                        key,
                        color
                            .into_iter()
                            .map(|v| panel_model::float_value(v))
                            .collect::<toml_edit::Array>(),
                    );
                }
            }
            if s.style.eq_ignore_ascii_case("simple") {
                ui.text_wrapped("Icon-background tint is Pretty only; its value is retained.");
            }
        }
    }
    fn hdr(&mut self, ui: &Ui, s: &Settings) {
        if self.section(ui, "HDR output", TreeNodeFlags::DEFAULT_OPEN) {
            let mut enabled = s.hdr_enabled;
            if ui.checkbox("Enable HDR UI", &mut enabled) {
                self.set("hdr_enabled", enabled);
            }
            self.number(ui, "UI brightness (nits)", "hdr_ui_brightness", s.hdr_ui_brightness, 100.0, "50 to 2000 nits. Project reset: 400 nits. Existing shader constants preview without resource recreation.");
            ui.text_wrapped("Applies through the existing DX12 HDR path on supported HDR output. This does not change the game's HDR or screen mode.");
        }
        if self.section(ui, "Color", TreeNodeFlags::empty()) {
            self.percent_slider(
                ui,
                "Saturation (%)",
                "hdr_ui_saturation",
                s.hdr_ui_saturation,
                1.0,
                [0.0, 200.0],
                "0 to 200%; 100% = multiplier 1.0.",
            );
        }
    }
    fn interface(&mut self, ui: &Ui, s: &Settings) {
        if self.section(ui, "Open/close settings", TreeNodeFlags::DEFAULT_OPEN) {
            let labels: Vec<String> = (1..=12).map(|i| format!("F{i}")).collect();
            let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            self.choice(
                ui,
                "Function key",
                "settings_function_key",
                &s.settings_function_key,
                &refs,
                &refs,
            );
            self.choice(
                ui,
                "Modifier",
                "settings_modifier",
                &s.settings_modifier,
                &["none", "left_shift", "left_ctrl", "left_alt"],
                &["None", "Left Shift", "Left Ctrl", "Left Alt"],
            );
            if let Ok(b) = panel_model::binding(s) {
                ui.text(format!("Draft binding: {}", b.label()));
            }
            ui.text_wrapped("Becomes active after successful Save and release of the old/new keys. Requires the selected left modifier only. No modifier means no Shift/Ctrl/Alt held.");
        }
        if self.section(ui, "Wheel details", TreeNodeFlags::DEFAULT_OPEN) {
            self.choice(
                ui,
                "Show details on",
                "wheel_details_wheels",
                s.wheel_details_wheels.as_str(),
                &["both", "spell", "item", "off"],
                &["Both wheels", "Spell wheel only", "Item wheel only", "Off"],
            );
            for (label, key, mut enabled) in [
                ("FP cost", "wheel_details_fp_cost", s.wheel_details_fp_cost),
                (
                    "Stat requirements",
                    "wheel_details_stat_requirements",
                    s.wheel_details_stat_requirements,
                ),
                (
                    "Item quantity",
                    "wheel_details_item_quantity",
                    s.wheel_details_item_quantity,
                ),
                (
                    "Availability messages",
                    "wheel_details_availability",
                    s.wheel_details_availability,
                ),
            ] {
                if ui.checkbox(label, &mut enabled) {
                    self.set(key, enabled);
                }
            }
            self.choice(
                ui,
                "Card position",
                "wheel_details_position",
                s.wheel_details_position.as_str(),
                &["below", "above"],
                &["Below", "Above"],
            );
        }
    }
    fn content(&mut self, ui: &Ui) {
        if self.read_error {
            ui.text_wrapped("Cannot read settings. Runtime values are retained. Resolve the file error, then use Reload disk before editing.");
            return;
        }
        if self.draft.mode == Mode::Advanced {
            ui.text_wrapped("Visual changes preview when valid. Live settings apply on Save.");
            ui.text_wrapped("Changes to mods, modded_spells, force_rgba, timing_offset, or await_xinput_hook require a game restart.");
            let saved_runtime = SettingsContext::saved_runtime();
            let icon_settings_changed = self.draft.last_valid.mods != saved_runtime.mods
                || self.draft.last_valid.modded_spells != saved_runtime.modded_spells
                || self.draft.last_valid.force_rgba != saved_runtime.force_rgba;
            let reload_safe = !self.draft.dirty()
                && !self.draft.external_change
                && self.draft.error.is_none()
                && !icon_settings_changed;
            {
                let _disabled = ui.begin_disabled(!reload_safe || self.reload_stage != 0);
                if self.button(ui, "Reload icons") {
                    self.reload_stage = 1;
                    self.feedback = "Reloading icons from saved runtime paths.".into();
                }
                if ui.is_item_hovered_with_flags(ItemHoveredFlags::ALLOW_WHEN_DISABLED) {
                    ui.tooltip_text(
                        "Reloads wheel icons from the active mod paths. Does not save your edits.",
                    );
                }
            }
            if icon_settings_changed {
                ui.text_wrapped(
                    "Save and restart to apply changes to mod paths or texture encoding.",
                );
            } else if !reload_safe {
                ui.text_wrapped(
                    "Reload icons requires valid saved settings with no external changes pending.",
                );
            }
            let size = ui.content_region_avail();
            let _editor_bg = ui.push_style_color(StyleColor::FrameBg, [0.14, 0.112, 0.078, 1.0]);
            if ui
                .input_text_multiline("##settings-toml", &mut self.draft.text, size)
                .build()
            {
                self.preview();
            }
        } else if let Some(_tabs) = ui.tab_bar("Settings tabs") {
            // ImGui remembers the selected tab and independent header state. No draft is rebuilt.
            for tab in Tab::ALL {
                if let Some(_tab) = ui.tab_item(tab.label()) {
                    self.draft.tab = tab;
                    if let Ok(s) = toml::from_str::<Settings>(&self.draft.text) {
                        let _width =
                            ui.push_item_width((ui.content_region_avail()[0] * 0.48).max(100.0));
                        match tab {
                            Tab::Appearance => self.appearance(ui, &s),
                            Tab::Hdr => self.hdr(ui, &s),
                            Tab::Interface => self.interface(ui, &s),
                        }
                    }
                }
            }
        }
    }
    pub fn draw(&mut self, ui: &Ui) {
        if !self.shown {
            return;
        }
        #[cfg(test)]
        {
            self.bounds.clear();
            self.sections.clear();
        }
        let viewport = ui.io().display_size;
        let [sw, sh] = viewport;
        if sw <= 0.0 || sh <= 0.0 {
            return;
        }
        let max_size = [(sw - 16.0).max(1.0), (sh - 16.0).max(1.0)];
        let min_size = [360.0_f32.min(max_size[0]), 310.0_f32.min(max_size[1])];
        let scale = (sh / 1440.0).min(1.0);
        let initial_size = [
            (620.0 * scale).clamp(min_size[0], max_size[0]),
            (560.0 * scale).clamp(min_size[1], max_size[1]),
        ];
        let _padding = ui.push_style_var(StyleVar::WindowPadding([12.0, 10.0]));
        let _colors = PANEL_COLORS.map(|(color, rgba)| ui.push_style_color(color, rgba));
        let mut window_open = true;
        let mut window = ui
            .window("Spell Wheel settings")
            .opened(&mut window_open)
            .size_constraints(min_size, max_size)
            .save_settings(false)
            .scroll_bar(false)
            .scrollable(false)
            .collapsible(false);
        match self.window {
            None => {
                let left = (28.0 * scale).max(8.0).min((sw - initial_size[0]).max(0.0));
                window = window
                    .position([left, (sh - initial_size[1]) * 0.5], Condition::Always)
                    .size(initial_size, Condition::Always);
            }
            Some((previous_viewport, position, size)) if previous_viewport != viewport => {
                let width = size[0].clamp(min_size[0], max_size[0]);
                let height = size[1].clamp(min_size[1], max_size[1]);
                window = window
                    .position(
                        [
                            position[0].clamp(0.0, (sw - width).max(0.0)),
                            position[1].clamp(0.0, (sh - height).max(0.0)),
                        ],
                        Condition::Always,
                    )
                    .size([width, height], Condition::Always);
            }
            _ => {}
        }
        window.build(|| {
            self.window = Some((viewport, ui.window_pos(), ui.window_size()));
            ui.text(format!(
                "{} mode  |  {}",
                if self.draft.mode == Mode::Advanced {
                    "Advanced"
                } else {
                    "Graphical"
                },
                if self.read_error {
                    "Read error"
                } else if self.draft.dirty() {
                    "Unsaved changes *"
                } else if self.draft.baseline.is_none() {
                    "New configuration (not written)"
                } else {
                    "Saved"
                }
            ));
            let active =
                panel_model::binding(&SettingsContext::saved_runtime()).unwrap_or(Binding::DEFAULT);
            ui.text(format!("Active shortcut: {}", active.label()));
            if self.draft.external_change {
                ui.text_colored(
                    [1.0, 0.75, 0.3, 1.0],
                    "Disk changed. Save will ask before overwriting.",
                );
            }
            ui.separator();
            let line = ui.text_line_height_with_spacing();
            let footer_height = line * 2.0
                + ui.frame_height_with_spacing() * 2.0
                + ui.clone_style().item_spacing[1] * 4.0
                + 4.0;
            let body_height = (ui.content_region_avail()[1] - footer_height).max(line);
            ui.child_window("Settings content")
                .size([0.0, body_height])
                .build(|| self.content(ui));
            ui.separator();
            ui.child_window("Settings feedback")
                .size([0.0, line * 2.0])
                .build(|| {
                    if let Some(e) = &self.draft.error {
                        ui.text_colored(
                            [1.0, 0.58, 0.48, 1.0],
                            "Invalid draft: last valid visual preview retained.",
                        );
                        ui.text_wrapped(e);
                    } else {
                        ui.text_wrapped(&self.feedback);
                    }
                });
            // Reset is separated from Save | Discard | mode. Two rows keep all actions reachable.
            if self.button(ui, "Reset to Defaults...") {
                if !self.read_error && self.draft.validate() {
                    self.reset_current = self.draft.mode == Mode::Graphical;
                    self.prompt(Modal::Reset);
                } else {
                    self.feedback =
                        "Fix invalid TOML or explicitly Discard before resetting.".into();
                }
            }
            ui.same_line();
            if self.button(ui, "Reload disk...") {
                self.prompt(Modal::Reload);
            }
            ui.separator();
            {
                let _disabled = ui.begin_disabled(self.read_error || self.draft.error.is_some());
                if self.button(ui, "Save") {
                    self.save(false);
                }
            }
            ui.same_line();
            if self.button(ui, "Discard") {
                self.discard();
            }
            ui.same_line();
            if self.button(
                ui,
                if self.draft.mode == Mode::Advanced {
                    "Graphical mode"
                } else {
                    "Advanced mode"
                },
            ) {
                self.draft.switch_mode();
            }
            self.draw_modal(ui);
        });
        if !window_open && self.modal.is_none() {
            self.request_close();
        }
        IS_OPEN.store(self.shown, Ordering::Release);
    }
    fn draw_modal(&mut self, ui: &Ui) {
        let Some(modal) = self.modal else {
            return;
        };
        if self.open_modal {
            ui.open_popup(modal.title());
            self.open_modal = false;
        }
        if let Some(_popup) = ui
            .modal_popup_config(modal.title())
            .always_auto_resize(true)
            .begin_popup()
        {
            match modal {
                Modal::Close => {
                    ui.text("Save or discard changes before closing?");
                    {
                        let _disabled =
                            ui.begin_disabled(self.read_error || self.draft.error.is_some());
                        if self.button(ui, "Save and close") {
                            if self.save(false) {
                                self.close();
                                ui.close_current_popup();
                            } else if self.modal != Some(Modal::Close) {
                                ui.close_current_popup();
                            }
                        }
                    }
                    if self.button(ui, "Discard and close") {
                        self.discard();
                        self.close();
                        ui.close_current_popup();
                    }
                    if self.button(ui, "Continue editing") {
                        self.draft.close_pending = false;
                        self.modal = None;
                        ui.close_current_popup();
                    }
                }
                Modal::Reset => {
                    if self.draft.mode == Mode::Graphical {
                        ui.radio_button("Current tab", &mut self.reset_current, true);
                    }
                    ui.radio_button("All panel settings", &mut self.reset_current, false);
                    if !self.reset_current {
                        ui.text("Includes HDR enabled / 400 nits and Left Shift + F10.");
                    }
                    ui.text("Staged only. Save persists; Discard undoes. Unmanaged fields are retained.");
                    if self.button(ui, "Confirm reset") {
                        let scope = if self.reset_current {
                            Some(self.draft.tab)
                        } else {
                            None
                        };
                        match self.draft.reset(scope) {
                            Ok(()) => self.preview(),
                            Err(e) => self.feedback = e,
                        }
                        self.modal = None;
                        ui.close_current_popup();
                    }
                    if self.button(ui, "Cancel") {
                        self.modal = None;
                        ui.close_current_popup();
                    }
                }
                Modal::Conflict => {
                    ui.text(
                        "The file changed externally. Keep editing, reload disk, or overwrite it?",
                    );
                    if self.button(ui, "Overwrite disk") {
                        if self.save(true) {
                            if self.draft.close_pending {
                                self.close();
                            } else {
                                self.modal = None;
                            }
                        } else {
                            self.modal = None;
                        }
                        ui.close_current_popup();
                    }
                    if self.button(ui, "Reload disk (discard draft)") {
                        self.reload_disk();
                        self.modal = None;
                        ui.close_current_popup();
                    }
                    if self.button(ui, "Keep editing") {
                        self.modal = None;
                        self.draft.close_pending = false;
                        ui.close_current_popup();
                    }
                }
                Modal::Reload => {
                    ui.text(
                        "Reload disk settings and discard the current draft? This does not save.",
                    );
                    if self.button(ui, "Reload disk") {
                        self.reload_disk();
                        self.modal = None;
                        ui.close_current_popup();
                    }
                    if self.button(ui, "Cancel") {
                        self.modal = None;
                        ui.close_current_popup();
                    }
                }
            }
        }
    }
    fn reload_disk(&mut self) {
        match panel_model::read_disk(&crate::paths::settings()) {
            Ok(disk) => {
                self.read_error = false;
                self.draft
                    .reload_disk(disk, SettingsContext::saved_runtime());
                if self.draft.error.is_none() {
                    // Explicit reload is an accepted new saved baseline; never a write.
                    SettingsContext::committed(&self.draft.last_valid);
                    self.feedback = "Reloaded disk. Live settings applied.".into();
                }
            }
            Err(e) => self.feedback = e,
        }
    }
}
