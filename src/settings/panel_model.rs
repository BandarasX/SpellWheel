//! Shared document/draft state. This module is also compiled by the host regression harness.
use super::Settings;
use crate::keyboard::settings_binding::Binding;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use toml_edit::{DocumentMut, Item, Value, value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Appearance,
    Hdr,
    Interface,
}
impl Tab {
    pub const ALL: [Self; 3] = [Self::Appearance, Self::Hdr, Self::Interface];
    pub fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Hdr => "HDR",
            Self::Interface => "Interface",
        }
    }
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Appearance => &[
                "style",
                "keyboard_mouse_inner_ring",
                "icon_scale_multiplier",
                "radius_multiplier",
                "highlight_expansion",
                "highlight_time",
                "item_names",
                "font_scale_multiplier",
                "text_shadows",
                "icon_tint",
                "icon_background_tint",
                "name_tint",
            ],
            Self::Hdr => &["hdr_enabled", "hdr_ui_brightness", "hdr_ui_saturation"],
            Self::Interface => &[
                "settings_function_key",
                "settings_modifier",
                "wheel_details_wheels",
                "wheel_details_show_name",
                "wheel_details_fp_cost",
                "wheel_details_stat_requirements",
                "wheel_details_item_quantity",
                "wheel_details_availability",
                "wheel_details_position",
                "wheel_details_background_transparency",
                "wheel_details_gold_border",
            ],
        }
    }
}

/// Compatibility defaults remain in Settings::default. Only resets and a missing NEW file
/// use this provider; an existing document with omitted HDR keys retains legacy fallbacks.
pub fn project_defaults() -> Settings {
    Settings {
        hdr_enabled: true,
        hdr_ui_brightness: 400.0,
        ..Settings::default()
    }
}

fn managed_document(s: &Settings) -> DocumentMut {
    let mut d = DocumentMut::new();
    macro_rules! scalar { ($($key:ident),*) => { $(d[stringify!($key)] = value(s.$key.clone());)* }; }
    scalar!(
        style,
        keyboard_mouse_inner_ring,
        item_names,
        text_shadows,
        hdr_enabled,
        settings_function_key,
        settings_modifier,
        wheel_details_show_name,
        wheel_details_fp_cost,
        wheel_details_stat_requirements,
        wheel_details_item_quantity,
        wheel_details_availability,
        wheel_details_gold_border
    );
    d["wheel_details_wheels"] = value(s.wheel_details_wheels.as_str());
    d["wheel_details_position"] = value(s.wheel_details_position.as_str());
    macro_rules! number { ($($key:ident),*) => { $(d[stringify!($key)] = Item::Value(float_value(s.$key));)* }; }
    number!(
        icon_scale_multiplier,
        radius_multiplier,
        highlight_expansion,
        highlight_time,
        font_scale_multiplier,
        hdr_ui_brightness,
        hdr_ui_saturation,
        wheel_details_background_transparency
    );
    for (key, color) in [
        ("icon_tint", s.icon_tint),
        ("icon_background_tint", s.icon_background_tint),
        ("name_tint", s.name_tint),
    ] {
        d[key] = value(
            color
                .into_iter()
                .map(|v| float_value(v))
                .collect::<toml_edit::Array>(),
        );
    }
    d
}

pub fn new_document() -> String {
    managed_document(&project_defaults()).to_string()
}

pub fn binding(s: &Settings) -> Result<Binding, String> {
    Binding::parse(&s.settings_function_key, &s.settings_modifier)
}

pub fn parse(text: &str) -> Result<Settings, String> {
    // serde checks every known key, including unmanaged keys; unknown keys are retained.
    let s: Settings = toml::from_str(text).map_err(|e| e.to_string())?;
    validate(&s)?;
    Ok(s)
}

/// Recover invalid hotkey values without changing any other parsed setting or the disk text.
/// Panel validation remains strict so the original invalid fields can be repaired in Advanced.
pub fn parse_startup(text: &str) -> Result<(Settings, Option<String>), String> {
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e: toml_edit::TomlError| e.to_string())?;
    let key = doc.get("settings_function_key").map(|v| v.as_str());
    let modifier = doc.get("settings_modifier").map(|v| v.as_str());
    let binding = Binding::parse(
        key.unwrap_or(Some("F10")).unwrap_or(""),
        modifier.unwrap_or(Some("left_shift")).unwrap_or(""),
    );
    let warning = binding.err().map(|e| format!("{e}; recovery binding is Left Shift + F10. Fix TOML with the game closed or in Advanced mode."));
    if warning.is_some() {
        doc.remove("settings_function_key");
        doc.remove("settings_modifier");
    }
    Ok((parse(&doc.to_string())?, warning))
}

pub fn validate(s: &Settings) -> Result<(), String> {
    binding(s)?;
    if !["pretty", "simple"]
        .iter()
        .any(|value| s.style.eq_ignore_ascii_case(value))
    {
        return Err("style must be pretty or simple".into());
    }
    if !["show", "center", "hide"]
        .iter()
        .any(|value| s.item_names.eq_ignore_ascii_case(value))
    {
        return Err("item_names must be show, center, or hide".into());
    }
    // Geometry/CPU budgets keep f32 screen-space math finite at supported desktop sizes.
    // The selector subtracts icon size AND 2 * (screen_min / 200) from wheel radius.
    for (key, v, min, max) in [
        ("icon_scale_multiplier", s.icon_scale_multiplier, 0.001, 4.0),
        ("radius_multiplier", s.radius_multiplier, 0.001, 8.0),
        ("font_scale_multiplier", s.font_scale_multiplier, 0.01, 16.0),
        ("highlight_expansion", s.highlight_expansion, 0.0, 8.0),
        ("highlight_time", s.highlight_time, 0.0, 60.0),
        ("hdr_ui_brightness", s.hdr_ui_brightness, 50.0, 2000.0),
        ("hdr_ui_saturation", s.hdr_ui_saturation, 0.0, 2.0),
        (
            "wheel_details_background_transparency",
            s.wheel_details_background_transparency,
            0.0,
            1.0,
        ),
        (
            "controller_wheel_open_delay",
            s.controller_wheel_open_delay,
            0.0,
            f32::MAX,
        ),
        ("mouse_min_radius", s.mouse_min_radius, 0.0, f32::MAX),
        (
            "controller_min_radius",
            s.controller_min_radius,
            0.0,
            f32::MAX,
        ),
        ("timing_offset", s.timing_offset, 0.0, f32::MAX),
    ] {
        if !v.is_finite() || v < min || v > max {
            return Err(format!("{key} must be finite and between {min} and {max}"));
        }
    }
    if std::time::Duration::try_from_secs_f32(s.timing_offset).is_err() {
        return Err("timing_offset must be representable as a Duration".into());
    }
    if !(s.mouse_min_radius * s.radius_multiplier * 16384.0)
        .powi(2)
        .is_finite()
        || !s.controller_min_radius.powi(2).is_finite()
    {
        return Err("selection radii must keep squared renderer distances finite at a 16384-pixel screen dimension".into());
    }
    if s.radius_multiplier <= s.icon_scale_multiplier + 0.01 {
        return Err(
            "radius_multiplier must exceed icon_scale_multiplier + 0.01 (selector clearance)"
                .into(),
        );
    }
    for (key, c) in [
        ("icon_tint", s.icon_tint),
        ("icon_background_tint", s.icon_background_tint),
        ("name_tint", s.name_tint),
    ] {
        if c.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) {
            return Err(format!(
                "{key} RGBA components must be finite and between 0 and 1"
            ));
        }
    }
    Ok(())
}

/// Copy visual settings for preview while retaining the active input, hook, and path values.
pub fn visual_preview(base: &Settings, draft: &Settings) -> Settings {
    let mut out = base.clone();
    macro_rules! copy { ($($key:ident),*) => { $(out.$key = draft.$key.clone();)* }; }
    copy!(
        style,
        keyboard_mouse_inner_ring,
        icon_scale_multiplier,
        radius_multiplier,
        highlight_expansion,
        highlight_time,
        item_names,
        font_scale_multiplier,
        text_shadows,
        icon_tint,
        icon_background_tint,
        name_tint,
        hdr_enabled,
        hdr_ui_brightness,
        hdr_ui_saturation,
        wheel_details_wheels,
        wheel_details_show_name,
        wheel_details_fp_cost,
        wheel_details_stat_requirements,
        wheel_details_item_quantity,
        wheel_details_availability,
        wheel_details_position,
        wheel_details_background_transparency,
        wheel_details_gold_border
    );
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Graphical,
    Advanced,
}

pub struct Draft {
    pub text: String,
    pub saved: String,
    pub baseline: Option<String>,
    pub mode: Mode,
    pub tab: Tab,
    pub error: Option<String>,
    pub last_valid: Settings,
    saved_preview: Settings,
    pub external_change: bool,
    pub close_pending: bool,
}
impl Draft {
    pub fn new(disk: Option<String>, runtime: Settings) -> Self {
        let text = disk.clone().unwrap_or_else(new_document);
        let parsed = parse(&text);
        Self {
            mode: if parsed.is_ok() {
                Mode::Graphical
            } else {
                Mode::Advanced
            },
            error: parsed.as_ref().err().cloned(),
            saved_preview: parsed.clone().unwrap_or(runtime.clone()),
            last_valid: parsed.unwrap_or(runtime),
            saved: text.clone(),
            text,
            baseline: disk,
            tab: Tab::Appearance,
            external_change: false,
            close_pending: false,
        }
    }
    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }
    pub fn validate(&mut self) -> bool {
        match parse(&self.text) {
            Ok(s) => {
                self.last_valid = s;
                self.error = None;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    pub fn switch_mode(&mut self) -> bool {
        if self.mode == Mode::Advanced && !self.validate() {
            return false;
        }
        self.mode = if self.mode == Mode::Graphical {
            Mode::Advanced
        } else {
            Mode::Graphical
        };
        true
    }
    pub fn set(&mut self, key: &str, v: Value) -> Result<(), String> {
        let mut doc: DocumentMut = self
            .text
            .parse()
            .map_err(|e: toml_edit::TomlError| e.to_string())?;
        set_preserving(&mut doc, key, Item::Value(v));
        self.text = doc.to_string();
        self.validate();
        Ok(())
    }
    pub fn reset(&mut self, scope: Option<Tab>) -> Result<(), String> {
        // No silent repair or text replacement, even when the syntax is valid but schema isn't.
        if !self.validate() {
            return Err("Fix invalid TOML or explicitly Discard before resetting".into());
        }
        let mut doc: DocumentMut = self
            .text
            .parse()
            .map_err(|e: toml_edit::TomlError| e.to_string())?;
        let defaults = managed_document(&project_defaults());
        for tab in Tab::ALL {
            if scope.is_some_and(|scope| scope != tab) {
                continue;
            }
            for &key in tab.keys() {
                set_preserving(&mut doc, key, defaults[key].clone());
            }
        }
        self.text = doc.to_string();
        self.validate();
        Ok(())
    }
    pub fn discard(&mut self) {
        self.text = self.saved.clone();
        self.last_valid = self.saved_preview.clone();
        self.validate();
        // An invalid startup file has no valid saved preview. Keep runtime/last valid values.
        if self.error.is_some() {
            self.mode = Mode::Advanced;
        }
        self.close_pending = false;
    }
    pub fn request_close(&mut self) -> bool {
        self.close_pending = self.dirty();
        !self.close_pending
    }
    pub fn observe_disk(&mut self, disk: Option<String>) {
        self.external_change = disk != self.baseline;
    }
    pub fn reload_disk(&mut self, disk: Option<String>, runtime: Settings) {
        let tab = self.tab;
        let mode = self.mode;
        *self = Self::new(disk, runtime);
        self.tab = tab;
        if self.error.is_none() {
            self.mode = mode;
        }
    }
    pub fn save(&mut self, path: &Path, overwrite: bool) -> Result<Settings, String> {
        let settings = parse(&self.text)?;
        let observed = read_disk(path)?;
        if !overwrite && observed != self.baseline {
            self.external_change = true;
            return Err(
                "File changed on disk. Choose Reload disk or explicitly Overwrite disk.".into(),
            );
        }
        atomic_save(path, &self.text, &observed)?;
        self.saved = self.text.clone();
        self.baseline = Some(self.text.clone());
        self.last_valid = settings.clone();
        self.saved_preview = settings.clone();
        self.error = None;
        self.external_change = false;
        Ok(settings)
    }
}

fn set_preserving(doc: &mut DocumentMut, key: &str, mut item: Item) {
    if let (Some(old), Some(new)) = (doc.get(key).and_then(Item::as_value), item.as_value_mut()) {
        *new.decor_mut() = old.decor().clone();
    }
    doc[key] = item;
}

pub fn read_disk(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read {}: {e}", path.display())),
    }
}

static TEMP_ID: AtomicU64 = AtomicU64::new(0);
pub fn atomic_save(path: &Path, text: &str, expected: &Option<String>) -> Result<(), String> {
    let name = path
        .file_name()
        .ok_or("Settings path has no filename")?
        .to_string_lossy();
    let temp = path.with_file_name(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file); // Windows must release the temporary handle before replacement.
        if &read_disk(path)? != expected {
            return Err("Settings changed during Save; original retained. Retry after reviewing disk changes.".into());
        }
        // std::fs::rename uses MoveFileExW(REPLACE_EXISTING) on Windows, same filesystem.
        fs::rename(&temp, path).map_err(|e| format!("Cannot replace settings file: {e}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Periodic disk refresh must never undo an active editor's preview.
pub struct RuntimeSettings {
    pub effective: Settings,
    pub saved: Settings,
    pub editing: bool,
}
impl RuntimeSettings {
    pub fn new(settings: Settings) -> Self {
        Self {
            effective: settings.clone(),
            saved: settings,
            editing: false,
        }
    }
    pub fn refresh(&mut self, disk: &Settings) {
        if !self.editing {
            self.commit(disk);
        }
    }
    pub fn preview(&mut self, draft: &Settings) {
        self.effective = visual_preview(&self.saved, draft);
    }
    pub fn commit(&mut self, settings: &Settings) {
        self.saved = visual_preview(&self.saved, settings);
        // These values are read during play. Preserve the startup hook, timing,
        // mod paths and texture encoding until the next launch.
        self.saved.spell_wheel_enabled = settings.spell_wheel_enabled;
        self.saved.item_wheel_enabled = settings.item_wheel_enabled;
        self.saved.spells_key.clone_from(&settings.spells_key);
        self.saved
            .quick_items_key
            .clone_from(&settings.quick_items_key);
        self.saved.center_on_close = settings.center_on_close;
        self.saved.switch_instantly = settings.switch_instantly;
        self.saved.mouse_min_radius = settings.mouse_min_radius;
        self.saved.controller_min_radius = settings.controller_min_radius;
        self.saved.debugging = settings.debugging;
        self.saved.controller_wheel_open_delay = settings.controller_wheel_open_delay;
        self.saved.settings_function_key = settings.settings_function_key.clone();
        self.saved.settings_modifier = settings.settings_modifier.clone();
        self.effective = self.saved.clone();
    }
    pub fn discard(&mut self) {
        self.effective = self.saved.clone();
    }
}

/// Display conversion: base multiplier is 100%; base=100 gives unscaled units.
pub fn to_percent(value: f32, base: f32) -> f32 {
    value / base * 100.0
}
pub fn from_percent(percent: f32, base: f32) -> f32 {
    percent / 100.0 * base
}

/// Use the shortest round-tripping f32 spelling instead of exposing widened binary tails in TOML.
pub fn float_value(value: f32) -> Value {
    Value::from(
        value
            .to_string()
            .parse::<f64>()
            .expect("f32 numeric spelling"),
    )
}
