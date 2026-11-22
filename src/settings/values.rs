macro_rules! settings {
    ($($name:ident: $t:ty),*$(,)?) => {
        pub const KEYS: &[&str] = &[
            $(stringify!($name),)*
        ];

        #[derive(Clone, Debug, PartialEq)]
        pub struct Settings {
            $(
                pub $name: $t
            ),*
        }

        impl<'de> serde::Deserialize<'de> for Settings {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                use serde::de::{MapAccess, Visitor};
                use std::fmt;

                struct SettingsVisitor;

                impl<'de> Visitor<'de> for SettingsVisitor {
                    type Value = Settings;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        write!(f, "a settings map")
                    }

                    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Settings, A::Error> {
                        let mut this = Settings::default();

                        while let Some(key) = map.next_key::<String>()? {
                            match key.as_str() {
                                $(stringify!($name) => {
                                    this.$name = map.next_value()?;
                                })*
                                _ => { map.next_value::<serde::de::IgnoredAny>()?; }
                            }
                        }

                        Ok(this)
                    }
                }

                deserializer.deserialize_map(SettingsVisitor)
            }
        }

        impl Default for Settings {
            fn default() -> Self {
                Self {
                    $(
                        $name: $name()
                    ),*
                }
            }
        }
    };
}

settings!(
    spell_wheel_enabled: bool,
    item_wheel_enabled: bool,
    spells_key: String,
    quick_items_key: String,
    style: String,
    keyboard_mouse_inner_ring: bool,
    highlight_time: f32,
    highlight_expansion: f32,
    controller_wheel_open_delay: f32,
    center_on_close: bool,
    switch_instantly: bool,
    item_names: String,
    text_shadows: bool,
    icon_tint: [f32; 4],
    icon_background_tint: [f32; 4],
    name_tint: [f32; 4],
    font_scale_multiplier: f32,
    icon_scale_multiplier: f32,
    radius_multiplier: f32,
    mouse_min_radius: f32,
    controller_min_radius: f32,
    mods: Vec<String>,
    modded_spells: Vec<String>,
    await_xinput_hook: bool,
    debugging: bool,
    timing_offset: f32,
    force_rgba: bool,
    hdr_enabled: bool,
    hdr_ui_brightness: f32,
    hdr_ui_saturation: f32,
    settings_function_key: String,
    settings_modifier: String,
    wheel_details_wheels: DetailWheels,
    wheel_details_fp_cost: bool,
    wheel_details_stat_requirements: bool,
    wheel_details_item_quantity: bool,
    wheel_details_availability: bool,
    wheel_details_position: DetailPosition,
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailWheels {
    #[default]
    Both,
    Spell,
    Item,
    Off,
}

impl DetailWheels {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Spell => "spell",
            Self::Item => "item",
            Self::Off => "off",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailPosition {
    #[default]
    Below,
    Above,
}

impl DetailPosition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Below => "below",
            Self::Above => "above",
        }
    }
}

fn wheel_details_wheels() -> DetailWheels {
    DetailWheels::default()
}

const fn wheel_details_fp_cost() -> bool {
    true
}

const fn wheel_details_stat_requirements() -> bool {
    true
}

const fn wheel_details_item_quantity() -> bool {
    true
}

const fn wheel_details_availability() -> bool {
    true
}

fn wheel_details_position() -> DetailPosition {
    DetailPosition::default()
}

pub fn spell_wheel_enabled() -> bool {
    true
}

pub fn item_wheel_enabled() -> bool {
    true
}

pub fn spells_key() -> String {
    "INGAME".to_string()
}

pub fn quick_items_key() -> String {
    "INGAME".to_string()
}

pub const fn center_on_close() -> bool {
    true
}

pub fn style() -> String {
    "pretty".to_string()
}

pub const fn keyboard_mouse_inner_ring() -> bool {
    false
}

pub const fn highlight_time() -> f32 {
    0.15
}

pub const fn highlight_expansion() -> f32 {
    0.15
}

pub const fn controller_wheel_open_delay() -> f32 {
    0.5
}

pub const fn switch_instantly() -> bool {
    true
}

pub fn item_names() -> String {
    "show".to_string()
}

pub const fn text_shadows() -> bool {
    true
}

pub const fn icon_tint() -> [f32; 4] {
    [1.0; 4]
}

pub const fn icon_background_tint() -> [f32; 4] {
    [1.0; 4]
}

pub const fn name_tint() -> [f32; 4] {
    [1.0; 4]
}

pub const fn debugging() -> bool {
    false
}

pub const fn font_scale_multiplier() -> f32 {
    1.0
}

pub const fn icon_scale_multiplier() -> f32 {
    0.15
}

pub const fn radius_multiplier() -> f32 {
    0.3
}

pub fn mods() -> Vec<String> {
    Vec::new()
}

pub fn modded_spells() -> Vec<String> {
    Vec::new()
}

pub const fn await_xinput_hook() -> bool {
    false
}

pub const fn mouse_min_radius() -> f32 {
    0.3
}

pub const fn controller_min_radius() -> f32 {
    0.9
}

pub const fn timing_offset() -> f32 {
    0.0
}

pub const fn force_rgba() -> bool {
    false
}

pub const fn hdr_enabled() -> bool {
    false
}

pub const fn hdr_ui_brightness() -> f32 {
    200.0
}

pub const fn hdr_ui_saturation() -> f32 {
    1.0
}

#[derive(Clone, Copy, Debug)]
pub enum ItemNames {
    Show,
    Center,
    Hide,
}

impl<S: AsRef<str>> From<S> for ItemNames {
    fn from(value: S) -> Self {
        match value.as_ref().to_lowercase().as_str() {
            "show" => Self::Show,
            "center" => Self::Center,
            "hide" => Self::Hide,
            _ => Self::Show,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Simple,
    Pretty,
}

impl<S: AsRef<str>> From<S> for Style {
    fn from(value: S) -> Self {
        match value.as_ref().to_lowercase().as_str() {
            "simple" => Self::Simple,
            "pretty" => Self::Pretty,
            _ => Self::Pretty,
        }
    }
}

pub fn settings_function_key() -> String {
    "F10".into()
}
pub fn settings_modifier() -> String {
    "left_shift".into()
}
impl Settings {
    pub fn item_names(&self) -> ItemNames {
        self.item_names.as_str().into()
    }

    pub fn style(&self) -> Style {
        self.style.as_str().into()
    }

    // Returns the highlight time t normalized. Takes into account self.highlight_time = 0.
    pub fn normalized_highlight_time(&self, t: f32) -> f32 {
        if self.highlight_time == 0.0 {
            if t > 0.0 { 1.0 } else { 0.0 }
        } else {
            t / self.highlight_time
        }
    }
}
