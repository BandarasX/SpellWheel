//! Settings-only shortcut events. Other keyboard/controller listeners keep their existing semantics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub(crate) const SHIFT_KEY: i32 = 0xA0;
pub(crate) const F10_KEY: i32 = 0x79;

static STATE: Mutex<ShortcutState> = Mutex::new(ShortcutState::new());
static GENERATION: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "settings-diagnostics")]
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

struct ShortcutState {
    armed: bool,
    chord_down: bool,
    generation: u64,
    #[cfg(feature = "settings-diagnostics")]
    last_sample: Option<u8>,
}

impl ShortcutState {
    const fn new() -> Self {
        Self {
            armed: false,
            chord_down: false,
            generation: 0,
            #[cfg(feature = "settings-diagnostics")]
            last_sample: None,
        }
    }

    fn poll(&mut self, focused: bool, shift: bool, f10: bool) -> bool {
        if !focused {
            self.armed = false;
            self.chord_down = false;
            return false;
        }
        // At startup and after focus loss, held keys cannot complete an old chord.
        if !self.armed {
            self.armed = !shift && !f10;
            return false;
        }
        let both_down = shift && f10;
        let activated = self.chord_down && !both_down;
        self.chord_down = both_down;
        if activated {
            self.generation = self.generation.wrapping_add(1);
        }
        activated
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
    #[cfg(feature = "settings-diagnostics")]
    id: u64,
}

impl SettingsShortcut {
    pub fn new() -> Self {
        Self {
            cursor: ActivationCursor { seen: GENERATION.load(Ordering::Acquire) },
            #[cfg(feature = "settings-diagnostics")]
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Apply pending activations once, even when several render callbacks run between input polls.
    pub fn update_visibility(&mut self, shown: &mut bool) -> u64 {
        self.cursor.apply(GENERATION.load(Ordering::Acquire), shown)
    }

    #[cfg(feature = "settings-diagnostics")]
    pub fn diagnostic_id(&self) -> u64 { self.id }

    #[cfg(feature = "settings-diagnostics")]
    pub fn generation(&self) -> u64 { self.cursor.seen }
}

/// Called only by the game input task; publication persists through later polls until consumed.
pub(crate) fn poll(focused: bool, shift: bool, f10: bool) {
    let mut state = STATE.lock().expect("Couldn't acquire settings shortcut state");
    let activated = state.poll(focused, shift, f10);
    if activated {
        GENERATION.store(state.generation, Ordering::Release);
    }
    #[cfg(feature = "settings-diagnostics")]
    {
        use hudhook::settings_diagnostics as diag;
        let mask = (shift as u8) | ((f10 as u8) << 1);
        let sample = mask | ((focused as u8) << 2);
        if activated { diag::arm(); }
        if state.last_sample != Some(sample) || activated {
            diag::event(format_args!("chord_poll source=settings_shortcut revision={} game_tick={} mask={} focused={} armed={} publish={} generation={}",
                crate::keyboard::listener::diagnostic_revision(), crate::get_tick(), mask, focused, state.armed, activated, state.generation));
            state.last_sample = Some(sample);
        }
    }
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
            for _ in 0..100 { assert!(!input.poll(true, true, true)); }
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
        for _ in 0..100 { input.poll(true, false, false); }
        assert_eq!(editor.apply(input.generation, &mut shown), 2);
        assert!(!shown);
        input.poll(true, true, true);
        input.poll(true, false, false);
        for _ in 0..100 { input.poll(true, false, false); }
        assert_eq!(editor.apply(input.generation, &mut shown), 1);
        assert!(shown);
    }

    #[test]
    fn independent_editors_do_not_consume_each_others_event() {
        let (mut input, mut first, mut first_shown) = ready();
        let mut second = ActivationCursor { seen: input.generation };
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
}
