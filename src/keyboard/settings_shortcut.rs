//! Settings-only shortcut events. Other keyboard/controller listeners keep their existing semantics.

use super::settings_binding::{Binding, Keys};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub static ALT_F4_BINDING: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<ShortcutState> = Mutex::new(ShortcutState::new());
static EPOCH: AtomicU64 = AtomicU64::new(0);
static EPOCH_START: AtomicU64 = AtomicU64::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "settings-diagnostics")]
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct ShortcutState {
    binding: Binding,
    old_binding: Option<Binding>,
    armed: bool,
    chord_down: bool,
    generation: u64,
    #[cfg(feature = "settings-diagnostics")]
    last_sample: Option<u8>,
}

impl ShortcutState {
    const fn new() -> Self {
        Self {
            binding: Binding::DEFAULT,
            old_binding: None,
            armed: false,
            chord_down: false,
            generation: 0,
            #[cfg(feature = "settings-diagnostics")]
            last_sample: None,
        }
    }

    fn configure(&mut self, binding: Binding) -> bool {
        if binding == self.binding {
            return false;
        }
        self.old_binding = Some(self.binding);
        self.binding = binding;
        self.armed = false;
        self.chord_down = false;
        true
    }

    fn poll_keys(&mut self, focused: bool, keys: Keys) -> bool {
        if !focused {
            self.armed = false;
            self.chord_down = false;
            return false;
        }
        if !self.armed {
            self.armed = keys.released(self.binding)
                && self.old_binding.is_none_or(|old| keys.released(old));
            if self.armed {
                self.old_binding = None;
            }
            return false;
        }
        let down = keys.down(self.binding);
        // Releasing either member activates. Adding another modifier cancels the chord.
        let allowed = keys.modifiers & !self.binding.modifier.bit() == 0;
        let activated = self.chord_down && !down && allowed;
        self.chord_down = down;
        if !allowed {
            self.armed = false;
            self.chord_down = false;
        }
        if activated {
            self.generation = self.generation.wrapping_add(1);
        }
        activated
    }

    #[cfg(test)]
    fn poll(&mut self, focused: bool, shift: bool, f10: bool) -> bool {
        self.poll_keys(
            focused,
            Keys {
                functions: if f10 { 1 << 9 } else { 0 },
                modifiers: shift as u8,
            },
        )
    }
}

struct ActivationCursor {
    seen: u64,
}

impl ActivationCursor {
    fn apply(&mut self, generation: u64, shown: &mut bool) -> u64 {
        let count = generation.wrapping_sub(self.seen);
        self.seen = generation;
        // Multiple deliberate chords between draws have the same effect as processing each one.
        if count & 1 != 0 {
            *shown = !*shown;
        }
        count
    }
}

/// One editor's consumption cursor; reading an activation never clears another consumer's event.
pub struct SettingsShortcut {
    cursor: ActivationCursor,
    epoch: u64,
    #[cfg(feature = "settings-diagnostics")]
    id: u64,
}

impl SettingsShortcut {
    pub fn new() -> Self {
        let _state = STATE.lock().expect("settings shortcut state");
        Self {
            cursor: ActivationCursor {
                seen: GENERATION.load(Ordering::Acquire),
            },
            epoch: EPOCH.load(Ordering::Acquire),
            #[cfg(feature = "settings-diagnostics")]
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Apply pending activations once, even when several render callbacks run between input polls.
    pub fn update_visibility(&mut self, shown: &mut bool) -> u64 {
        let _state = STATE.lock().expect("settings shortcut state");
        let epoch = EPOCH.load(Ordering::Acquire);
        let generation = GENERATION.load(Ordering::Acquire);
        if epoch != self.epoch {
            self.epoch = epoch;
            // Drop pre-rebind events, retain valid new activations even if no render ran since Save.
            self.cursor.seen = EPOCH_START.load(Ordering::Acquire);
        }
        self.cursor.apply(generation, shown)
    }

    #[cfg(feature = "settings-diagnostics")]
    pub fn diagnostic_id(&self) -> u64 {
        self.id
    }

    #[cfg(feature = "settings-diagnostics")]
    pub fn generation(&self) -> u64 {
        self.cursor.seen
    }
}

/// Called only by the game input task; publication persists through later polls until consumed.
pub(crate) fn poll_keys(focused: bool, keys: Keys) {
    let mut state = STATE
        .lock()
        .expect("Couldn't acquire settings shortcut state");
    let activated = state.poll_keys(focused, keys);
    if activated {
        GENERATION.store(state.generation, Ordering::Release);
    }
    #[cfg(feature = "settings-diagnostics")]
    {
        use hudhook::settings_diagnostics as diag;
        let mask = (keys.modifiers != 0) as u8 | ((keys.function(state.binding) as u8) << 1);
        let sample = mask | ((focused as u8) << 2);
        if activated {
            diag::arm();
        }
        if state.last_sample != Some(sample) || activated {
            diag::event(format_args!(
                "chord_poll source=settings_shortcut revision={} game_tick={} mask={} focused={} armed={} publish={} generation={}",
                crate::keyboard::listener::diagnostic_revision(),
                crate::get_tick(),
                mask,
                focused,
                state.armed,
                activated,
                state.generation
            ));
            state.last_sample = Some(sample);
        }
    }
}

/// Commit access only after disk persistence. Epoch invalidates queued pre-save activations.
pub fn configure(binding: Binding) {
    let mut state = STATE.lock().expect("settings shortcut state");
    if state.configure(binding) {
        ALT_F4_BINDING.store(
            binding.function == 4 && binding.modifier == super::settings_binding::Modifier::LeftAlt,
            Ordering::Release,
        );
        EPOCH_START.store(state.generation, Ordering::Release);
        EPOCH.fetch_add(1, Ordering::Release);
    }
}

/// Sample raw keys once on the existing input task, never in a Present callback.
pub(crate) fn sample(focused: bool) {
    let mut keys = Keys::default();
    for index in 0..12 {
        if crate::keyboard::is_key_pressed(0x70 + index) {
            keys.functions |= 1 << index;
        }
    }
    keys.modifiers = modifier_mask();
    poll_keys(focused, keys);
}

/// Raw side-specific modifier state for synchronous native system-key suppression.
pub(crate) fn modifier_mask() -> u8 {
    let mut modifiers = 0;
    for (index, code) in [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5].into_iter().enumerate() {
        if crate::keyboard::is_key_pressed(code) {
            modifiers |= 1 << index;
        }
    }
    modifiers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> (ShortcutState, ActivationCursor, bool) {
        let mut state = ShortcutState::new();
        assert!(!state.poll(true, false, false));
        (state, ActivationCursor { seen: 0 }, false)
    }

    #[test]
    fn captured_nested_present_schedule_opens_once_and_later_chord_closes() {
        let (mut input, mut editor, mut shown) = ready();
        input.poll(true, true, false);
        input.poll(true, true, true);
        assert!(input.poll(true, true, false)); // User's observed F10-first release.
        assert_eq!(editor.apply(input.generation, &mut shown), 1); // Outer Present.
        assert!(shown);
        assert_eq!(editor.apply(input.generation, &mut shown), 0); // Nested Present before poll.
        assert!(shown);
        for _ in 0..100 {
            input.poll(true, true, false);
            assert_eq!(editor.apply(input.generation, &mut shown), 0);
            assert!(shown);
        }
        input.poll(true, false, false);
        input.poll(true, true, true);
        assert!(input.poll(true, false, true)); // Shift-first release also closes normally.
        assert_eq!(editor.apply(input.generation, &mut shown), 1);
        assert!(!shown);
    }

    #[test]
    fn held_keys_and_second_key_release_do_not_repeat() {
        for remaining in [(true, false), (false, true)] {
            let (mut input, mut editor, mut shown) = ready();
            for _ in 0..100 {
                assert!(!input.poll(true, true, true));
            }
            assert_eq!(editor.apply(input.generation, &mut shown), 0);
            assert!(!shown);
            assert!(input.poll(true, remaining.0, remaining.1));
            assert!(!input.poll(true, false, false));
            assert_eq!(editor.apply(input.generation, &mut shown), 1);
            assert!(shown);
        }
    }

    #[test]
    fn focus_loss_discards_chord_and_requires_all_keys_up_to_rearm() {
        let (mut input, mut editor, mut shown) = ready();
        input.poll(true, true, true);
        assert!(!input.poll(false, false, false));
        assert!(!input.poll(true, true, true));
        assert!(!input.poll(true, true, false));
        assert!(!input.poll(true, true, true));
        assert!(!input.poll(true, false, false));
        assert_eq!(editor.apply(input.generation, &mut shown), 0);
        assert!(!shown);
        input.poll(true, true, true);
        assert!(input.poll(true, false, false));
        assert_eq!(editor.apply(input.generation, &mut shown), 1);
        assert!(shown);
    }

    #[test]
    fn startup_with_held_keys_does_not_activate() {
        let mut input = ShortcutState::new();
        assert!(!input.poll(true, true, true));
        assert!(!input.poll(true, false, false));
        assert_eq!(input.generation, 0);
        input.poll(true, true, true);
        assert!(input.poll(true, true, false));
    }

    #[test]
    fn events_survive_missing_draws_and_multiple_chords_keep_their_parity() {
        let (mut input, mut editor, mut shown) = ready();
        for _ in 0..2 {
            input.poll(true, true, true);
            input.poll(true, false, false);
        }
        for _ in 0..100 {
            input.poll(true, false, false);
        }
        assert_eq!(editor.apply(input.generation, &mut shown), 2);
        assert!(!shown);
        input.poll(true, true, true);
        input.poll(true, false, false);
        for _ in 0..100 {
            input.poll(true, false, false);
        }
        assert_eq!(editor.apply(input.generation, &mut shown), 1);
        assert!(shown);
    }

    #[test]
    fn independent_editors_do_not_consume_each_others_event() {
        let (mut input, mut first, mut first_shown) = ready();
        let mut second = ActivationCursor {
            seen: input.generation,
        };
        let mut second_shown = false;
        input.poll(true, true, true);
        input.poll(true, false, false);
        assert_eq!(first.apply(input.generation, &mut first_shown), 1);
        assert_eq!(second.apply(input.generation, &mut second_shown), 1);
        assert!(first_shown && second_shown);
    }

    #[test]
    fn generation_wrap_preserves_one_activation() {
        let mut input = ShortcutState::new();
        input.poll(true, false, false);
        input.generation = u64::MAX;
        let mut editor = ActivationCursor { seen: u64::MAX };
        let mut shown = false;
        input.poll(true, true, true);
        input.poll(true, false, false);
        assert_eq!(editor.apply(input.generation, &mut shown), 1);
        assert!(shown);
    }
    fn keys(binding: Binding, function: bool, modifiers: u8) -> Keys {
        Keys {
            functions: if function {
                1 << (binding.function - 1)
            } else {
                0
            },
            modifiers,
        }
    }

    #[test]
    fn every_supported_binding_handles_hold_repeat_release_orders_focus_and_nested_draws() {
        use super::super::settings_binding::Modifier;
        for function in 1..=12 {
            for modifier in Modifier::ALL {
                for release_function_first in [false, true] {
                    let binding = Binding { function, modifier };
                    let mut state = ShortcutState::new();
                    state.configure(binding);
                    let up = Keys::default();
                    let down = keys(binding, true, modifier.bit());
                    state.poll_keys(true, down); // Startup or rebind while held cannot activate.
                    state.poll_keys(true, up);
                    assert_eq!(state.generation, 0);
                    state.poll_keys(true, down);
                    for _ in 0..12 {
                        assert!(!state.poll_keys(true, down));
                    }
                    let released = if release_function_first {
                        keys(binding, false, modifier.bit())
                    } else {
                        keys(binding, true, 0)
                    };
                    if modifier == Modifier::None {
                        assert!(state.poll_keys(true, up));
                    } else {
                        assert!(state.poll_keys(true, released));
                        assert!(!state.poll_keys(true, up));
                    }
                    let mut cursor = ActivationCursor { seen: 0 };
                    let mut shown = false;
                    assert_eq!(cursor.apply(state.generation, &mut shown), 1);
                    assert!(shown);
                    assert_eq!(cursor.apply(state.generation, &mut shown), 0);
                    assert!(shown);
                    state.poll_keys(true, down);
                    state.poll_keys(false, up);
                    state.poll_keys(true, down);
                    state.poll_keys(true, up);
                    assert_eq!(state.generation, 1);
                    state.poll_keys(true, down);
                    state.poll_keys(true, up);
                    assert_eq!(cursor.apply(state.generation, &mut shown), 1);
                    assert!(!shown);
                }
            }
        }
    }

    #[test]
    fn wrong_side_extra_modifiers_and_interrupted_chords_do_not_activate() {
        use super::super::settings_binding::Modifier;
        for modifier in Modifier::ALL {
            let binding = Binding {
                function: 10,
                modifier,
            };
            for wrong in [1, 2, 4, 8, 16, 32, 3, 5, 63] {
                if wrong == modifier.bit() {
                    continue;
                }
                let mut state = ShortcutState::new();
                state.configure(binding);
                state.poll_keys(true, Keys::default());
                state.poll_keys(true, keys(binding, true, wrong));
                state.poll_keys(true, keys(binding, false, wrong));
                state.poll_keys(true, Keys::default());
                assert_eq!(state.generation, 0);
            }
            let mut state = ShortcutState::new();
            state.configure(binding);
            state.poll_keys(true, Keys::default());
            state.poll_keys(true, keys(binding, true, modifier.bit()));
            state.poll_keys(true, keys(binding, true, 63)); // Adding an extra modifier cancels.
            state.poll_keys(true, Keys::default());
            assert_eq!(state.generation, 0);
        }
    }

    #[test]
    fn rebind_requires_old_and_new_keys_released_and_does_not_publish_stale_chord() {
        use super::super::settings_binding::Modifier;
        let (mut state, _, _) = ready();
        state.poll(true, true, true);
        let new = Binding {
            function: 2,
            modifier: Modifier::None,
        };
        assert!(state.configure(new));
        assert!(
            !state.configure(new),
            "reapplying saved binding must not rearm every second"
        );
        let held_old = Keys {
            functions: 1 << 9,
            modifiers: 1,
        };
        state.poll_keys(true, held_old);
        state.poll_keys(true, keys(new, true, 0));
        state.poll_keys(true, Keys::default());
        assert_eq!(state.generation, 0);
        state.poll_keys(true, keys(new, true, 0));
        assert!(state.poll_keys(true, Keys::default()));
        assert_eq!(state.generation, 1);
    }

    #[test]
    fn binding_schema_rejects_unsupported_keys_and_multiple_modifiers() {
        for key in ["F0", "F13", "F24", "f10", "F01", "TAB", "10", ""] {
            assert!(Binding::parse(key, "left_shift").is_err());
        }
        for modifier in [
            "right_shift",
            "shift",
            "left_shift+left_ctrl",
            "",
            "LEFT_SHIFT",
        ] {
            assert!(Binding::parse("F10", modifier).is_err());
        }
    }

    #[test]
    fn deliberate_function_key_repress_while_modifier_stays_down_is_a_second_chord() {
        let (mut state, mut cursor, mut shown) = ready();
        for _ in 0..2 {
            state.poll(true, true, true);
            assert!(state.poll(true, true, false));
            for _ in 0..8 {
                assert!(!state.poll(true, true, false));
            }
        }
        assert_eq!(cursor.apply(state.generation, &mut shown), 2);
        assert!(!shown);
    }
}
