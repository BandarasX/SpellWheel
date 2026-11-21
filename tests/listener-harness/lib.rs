//! Host-side characterization of the actual listener source with controlled key/focus polling.
#![allow(dead_code)]

mod debugging {
    pub fn is_debugging() -> bool {
        false
    }
    pub fn add_to_screen_debug(_: String) {}
}

mod keyboard {
    pub(crate) use crate::{settings_binding, settings_shortcut};
    use std::sync::atomic::{AtomicU8, Ordering};
    pub static INPUT: AtomicU8 = AtomicU8::new(0);
    pub static EXTRA_FUNCTIONS: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
    pub static EXTRA_MODIFIERS: AtomicU8 = AtomicU8::new(0);
    pub fn is_key_pressed(key: i32) -> bool {
        let state = INPUT.load(Ordering::Relaxed);
        let bit = match key {
            0xA0 => 1,
            0x79 => 2,
            _ => 0,
        };
        state & bit != 0
            || (if (0x70..=0x7b).contains(&key) {
                EXTRA_FUNCTIONS.load(Ordering::Relaxed) & (1 << (key - 0x70)) != 0
            } else if let Some(index) = [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5]
                .iter()
                .position(|code| *code == key)
            {
                EXTRA_MODIFIERS.load(Ordering::Relaxed) & (1 << index) != 0
            } else {
                false
            })
    }
    pub fn is_pressed(key: i32) -> bool {
        crate::io::is_game_focused() && is_key_pressed(key)
    }
}

mod io {
    pub fn is_game_focused() -> bool {
        crate::keyboard::INPUT.load(std::sync::atomic::Ordering::Relaxed) & 4 != 0
    }
}

#[path = "../../src/keyboard/settings_binding.rs"]
pub(crate) mod settings_binding;

#[path = "../../src/keyboard/settings_shortcut.rs"]
pub(crate) mod settings_shortcut;

#[path = "../../src/keyboard/listener.rs"]
mod listener;

#[cfg(test)]
mod tests {
    use super::*;
    use listener::{Listener, setup_listener, tick_listeners};
    use std::sync::{Mutex, atomic::Ordering};
    pub(crate) static SERIAL: Mutex<()> = Mutex::new(());

    fn poll(keys: u8, focused: bool) {
        keyboard::INPUT.store(keys | if focused { 4 } else { 0 }, Ordering::Relaxed);
        tick_listeners();
    }

    fn release_listener() -> listener::ListenerStatus {
        poll(0, true);
        setup_listener(Listener::Released {
            keys: Box::new([0xA0, 0x79]),
        })
    }

    #[test]
    fn one_release_can_be_observed_by_two_consumers_until_next_poll() {
        let _lock = SERIAL.lock().unwrap();
        let status = release_listener();
        poll(3, true);
        for _ in 0..10 {
            poll(3, true);
            assert!(!status.is_active());
        }
        poll(1, true);
        assert!(status.is_active());
        assert!(
            status.is_active(),
            "a render read does not consume release status"
        );
        poll(0, true);
        assert!(!status.is_active());
    }

    #[test]
    fn either_modifier_release_order_activates_once_in_producer() {
        let _lock = SERIAL.lock().unwrap();
        for remaining in [1, 2] {
            let status = release_listener();
            poll(3, true);
            poll(remaining, true);
            assert!(status.is_active());
            poll(0, true);
            assert!(!status.is_active(), "second key-up does not activate again");
        }
    }

    #[test]
    fn release_is_lost_if_no_consumer_reads_before_next_poll() {
        let _lock = SERIAL.lock().unwrap();
        let status = release_listener();
        poll(3, true);
        poll(0, true);
        poll(0, true);
        assert!(!status.is_active());
    }

    #[test]
    fn focus_loss_while_chord_is_held_looks_like_release() {
        let _lock = SERIAL.lock().unwrap();
        let status = release_listener();
        poll(3, true);
        poll(3, false);
        assert!(status.is_active());
        poll(3, false);
        assert!(!status.is_active());
    }

    #[test]
    fn game_poll_to_editor_integration_handles_nested_draws_and_focus() {
        let _lock = SERIAL.lock().unwrap();
        poll(0, true);
        let mut shortcut = settings_shortcut::SettingsShortcut::new();
        let mut shown = false;
        poll(1, true);
        poll(3, true);
        poll(1, true);
        assert_eq!(shortcut.update_visibility(&mut shown), 1);
        assert!(shown);
        assert_eq!(shortcut.update_visibility(&mut shown), 0);
        assert!(shown);
        poll(0, true);
        poll(3, true);
        poll(2, true);
        poll(0, true); // No render until after the producer has polled again.
        assert_eq!(shortcut.update_visibility(&mut shown), 1);
        assert!(!shown);
        poll(3, true);
        poll(3, false);
        poll(3, true);
        poll(1, true);
        poll(0, true);
        assert_eq!(shortcut.update_visibility(&mut shown), 0);
        assert!(!shown);
    }

    #[test]
    fn actual_poll_editor_save_rebind_and_failed_save_do_not_replay_queued_events() {
        let _lock = SERIAL.lock().unwrap();
        use settings_binding::{Binding, Modifier};
        settings_shortcut::configure(Binding::DEFAULT);
        poll(0, true);
        let mut editor = settings_shortcut::SettingsShortcut::new();
        let mut shown = true;
        poll(3, true);
        poll(0, true); // Published but not yet consumed.
        let mut draft =
            crate::panel_model::Draft::new(Some(String::new()), crate::Settings::default());
        draft
            .set("settings_function_key", toml_edit::Value::from("F2"))
            .unwrap();
        draft
            .set("settings_modifier", toml_edit::Value::from("none"))
            .unwrap();
        let failed = std::env::temp_dir().join(format!(
            "spellwheel-no-parent-{}/settings.toml",
            std::process::id()
        ));
        assert!(draft.save(&failed, true).is_err());
        assert_eq!(
            editor.update_visibility(&mut shown),
            1,
            "failed Save preserves the original event/binding"
        );
        assert!(!shown);
        poll(3, true);
        poll(0, true); // Another old queued activation.
        let file =
            std::env::temp_dir().join(format!("spellwheel-rebind-{}.toml", std::process::id()));
        std::fs::write(&file, "").unwrap();
        let saved = draft.save(&file, false).unwrap();
        settings_shortcut::configure(crate::panel_model::binding(&saved).unwrap());
        // No rendering from Save through the next deliberate chord: pre-save events must be
        // discarded without throwing away the first legitimate new-binding activation.
        assert!(!shown);
        keyboard::EXTRA_FUNCTIONS.store(1 << 1, Ordering::Relaxed);
        poll(0, true);
        keyboard::EXTRA_FUNCTIONS.store(0, Ordering::Relaxed);
        poll(0, true);
        keyboard::EXTRA_FUNCTIONS.store(1 << 1, Ordering::Relaxed);
        poll(0, true);
        keyboard::EXTRA_FUNCTIONS.store(0, Ordering::Relaxed);
        poll(0, true);
        assert_eq!(editor.update_visibility(&mut shown), 1);
        assert!(shown);
        assert_eq!(editor.update_visibility(&mut shown), 0);
        assert!(shown);
        assert_eq!(
            crate::panel_model::binding(&saved).unwrap(),
            Binding {
                function: 2,
                modifier: Modifier::None
            }
        );
        settings_shortcut::configure(Binding::DEFAULT);
        poll(0, true);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn other_listener_pressed_held_and_released_behavior_is_unchanged() {
        let _lock = SERIAL.lock().unwrap();
        poll(0, true);
        let pressed = setup_listener(Listener::Pressed {
            keys: Box::new([0xA0, 0x79]),
        });
        let held = setup_listener(Listener::Held {
            keys: Box::new([0xA0, 0x79]),
        });
        let released = setup_listener(Listener::Released {
            keys: Box::new([0xA0, 0x79]),
        });
        poll(3, true);
        assert!(pressed.is_active() && held.is_active());
        assert!(!released.is_active());
        poll(3, true);
        assert!(!pressed.is_active());
        assert!(held.is_active());
        assert!(!released.is_active());
        poll(1, true);
        assert!(!pressed.is_active() && !held.is_active());
        assert!(released.is_active());
        poll(0, true);
        assert!(!released.is_active());
    }
}

#[path = "../../src/settings/panel_model.rs"]
pub mod panel_model;
#[path = "../../src/settings/values.rs"]
mod settings_values;
pub use settings_values::Settings;

#[cfg(test)]
mod panel_tests;

#[path = "../../src/settings/input_policy.rs"]
pub mod input_policy;

// Platform adapters for testing the actual panel widget/controller with a real ImGui context.
// The document, validation, atomic save and shortcut code remain production modules.
#[cfg(test)]
mod settings {
    pub use crate::{Settings, panel_model};
    use std::sync::{LazyLock, Mutex};
    static RUNTIME: LazyLock<Mutex<panel_model::RuntimeSettings>> =
        LazyLock::new(|| Mutex::new(panel_model::RuntimeSettings::new(Settings::default())));
    pub struct SettingsContext;
    impl SettingsContext {
        pub fn begin_editing() -> Settings {
            let mut r = RUNTIME.lock().unwrap();
            r.editing = true;
            r.saved.clone()
        }
        pub fn end_editing() {
            let mut r = RUNTIME.lock().unwrap();
            r.discard();
            r.editing = false;
        }
        pub fn saved_runtime() -> Settings {
            RUNTIME.lock().unwrap().saved.clone()
        }
        pub fn preview(s: &Settings) {
            RUNTIME.lock().unwrap().preview(s);
        }
        pub fn discard_preview() {
            RUNTIME.lock().unwrap().discard();
        }
        pub fn committed(s: &Settings) {
            RUNTIME.lock().unwrap().commit(s);
            crate::settings_shortcut::configure(panel_model::binding(s).unwrap());
        }
    }
    impl Settings {
        pub fn read_or_default() -> Self {
            RUNTIME.lock().unwrap().effective.clone()
        }
    }
}
#[cfg(test)]
pub use settings::SettingsContext;
#[cfg(test)]
mod paths {
    pub fn settings() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("spellwheel-ui-tests-{}.toml", std::process::id()))
    }
}
#[cfg(test)]
mod display_settings {
    include!("../../src/settings/display_settings.rs");
    mod tests {
        include!("panel_ui_tests.rs");
    }
}

#[cfg(test)]
#[path = "../../src/io/controller_state.rs"]
mod controller_state;

#[cfg(test)]
#[path = "../../src/io/hook_profiles.rs"]
mod hook_profiles;
