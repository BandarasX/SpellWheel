//! Host-side characterization of the actual listener source with controlled key/focus polling.
#![allow(dead_code)]

mod debugging {
    pub fn is_debugging() -> bool { false }
    pub fn add_to_screen_debug(_: String) {}
}

mod keyboard {
    pub(crate) use crate::settings_shortcut;
    use std::sync::atomic::{AtomicU8, Ordering};
    pub static INPUT: AtomicU8 = AtomicU8::new(0);
    pub fn is_key_pressed(key: i32) -> bool {
        let state = INPUT.load(Ordering::Relaxed);
        let bit = match key { 0xA0 => 1, 0x79 => 2, _ => 0 };
        state & bit != 0
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

#[path = "../../src/keyboard/settings_shortcut.rs"]
pub(crate) mod settings_shortcut;

#[path = "../../src/keyboard/listener.rs"]
mod listener;

#[cfg(test)]
mod tests {
    use super::*;
    use listener::{Listener, setup_listener, tick_listeners};
    use std::sync::{Mutex, atomic::Ordering};
    static SERIAL: Mutex<()> = Mutex::new(());

    fn poll(keys: u8, focused: bool) {
        keyboard::INPUT.store(keys | if focused { 4 } else { 0 }, Ordering::Relaxed);
        tick_listeners();
    }

    fn release_listener() -> listener::ListenerStatus {
        poll(0, true);
        setup_listener(Listener::Released { keys: Box::new([0xA0, 0x79]) })
    }

    #[test]
    fn one_release_can_be_observed_by_two_consumers_until_next_poll() {
        let _lock = SERIAL.lock().unwrap();
        let status = release_listener();
        poll(3, true);
        for _ in 0..10 { poll(3, true); assert!(!status.is_active()); }
        poll(1, true);
        assert!(status.is_active());
        assert!(status.is_active(), "a render read does not consume release status");
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
    fn other_listener_pressed_held_and_released_behavior_is_unchanged() {
        let _lock = SERIAL.lock().unwrap();
        poll(0, true);
        let pressed = setup_listener(Listener::Pressed { keys: Box::new([0xA0, 0x79]) });
        let held = setup_listener(Listener::Held { keys: Box::new([0xA0, 0x79]) });
        let released = setup_listener(Listener::Released { keys: Box::new([0xA0, 0x79]) });
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
