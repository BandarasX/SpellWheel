pub mod display_settings;
pub mod input_policy;
pub mod panel_model;
mod values;
pub mod visual_toml_parser;
pub use values::*;

use crate::debugging::{run_every, run_once};
use crate::keyboard::settings_shortcut;
use panel_model::{RuntimeSettings, binding, read_disk};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static RUNTIME: OnceLock<Mutex<RuntimeSettings>> = OnceLock::new();

fn runtime() -> &'static Mutex<RuntimeSettings> {
    RUNTIME.get_or_init(|| {
        let settings = load_disk().unwrap_or_else(|e| {
            tracing::error!(
                "Settings startup: {e}; using safe compatibility defaults without rewriting disk"
            );
            Settings::default()
        });
        settings_shortcut::configure(
            binding(&settings).unwrap_or(crate::keyboard::settings_binding::Binding::DEFAULT),
        );
        Mutex::new(RuntimeSettings::new(settings))
    })
}

fn load_disk() -> Result<Settings, String> {
    let text = read_disk(&crate::paths::settings())?.unwrap_or_else(panel_model::new_document);
    let (settings, warning) = panel_model::parse_startup(&text)?;
    if let Some(warning) = warning {
        tracing::error!("{warning}");
    }
    Ok(settings)
}

pub struct SettingsContext;
impl SettingsContext {
    pub fn begin_editing() -> Settings {
        let mut rt = runtime().lock().expect("settings runtime");
        rt.editing = true;
        rt.saved.clone()
    }
    pub fn end_editing() {
        let mut rt = runtime().lock().expect("settings runtime");
        rt.discard();
        rt.editing = false;
    }
    pub fn preview(settings: &Settings) {
        runtime()
            .lock()
            .expect("settings runtime")
            .preview(settings);
    }
    pub fn discard_preview() {
        runtime().lock().expect("settings runtime").discard();
    }
    pub fn committed(settings: &Settings) {
        runtime().lock().expect("settings runtime").commit(settings);
        settings_shortcut::configure(binding(settings).expect("saved binding validated"));
    }
    pub fn saved_runtime() -> Settings {
        runtime().lock().expect("settings runtime").saved.clone()
    }
}

impl Settings {
    pub fn read_or_default() -> Self {
        let rt = runtime();
        run_every!("settings disk refresh" every Duration::from_secs(1) => {
            let mut rt = rt.lock().expect("settings runtime");
            if !rt.editing {
                match load_disk() {
                    Ok(settings) => {
                        rt.refresh(&settings);
                        settings_shortcut::configure(binding(&settings).expect("validated binding"));
                    }
                    Err(e) => tracing::error!("Settings refresh ignored; last valid runtime retained: {e}"),
                }
            }
        });
        rt.lock().expect("settings runtime").effective.clone()
    }
    pub fn mods(&self) -> &[String] {
        match (self.mods.is_empty(), self.modded_spells.is_empty()) {
            (true, true) => &[],
            (true, false) => &self.modded_spells,
            (false, true) => &self.mods,
            (false, false) => {
                run_once!("Settings::mods" => {
                    tracing::warn!(
                        "Both 'mods' and 'modded_spells' aren't empty. Defaulting to 'mods'.\n\
                        'modded_spells' is old and 'mods' should be used instead"
                    );
                });
                &self.mods
            }
        }
    }
}
