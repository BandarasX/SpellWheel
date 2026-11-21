use super::*;

const DELAY: Duration = Duration::from_millis(250);
const SPELL: u32 = 1;
const ITEM: u32 = 2;
const REMAP: u32 = 1 << 17;

const START: Slot = Slot {
    index: 1,
    param_id: 100,
};
const NEXT: Slot = Slot {
    index: 5,
    param_id: 100,
};
const INVALID: Slot = Slot {
    index: 6,
    param_id: 200,
};

struct Pad {
    state: ControllerState,
    base: Instant,
}

impl Pad {
    fn new() -> Self {
        let mut pad = Self {
            state: ControllerState::new(),
            base: Instant::now(),
        };
        pad.state.set_bindings([SPELL, ITEM]);
        pad.sample(0, 0);
        pad
    }

    fn sample(&mut self, buttons: u32, ms: u64) {
        self.observe(Some(buttons), ms, Context::Gameplay);
    }

    fn observe(&mut self, buttons: Option<u32>, ms: u64, context: Context) {
        self.state.observe_buttons(
            buttons.map(|buttons| (buttons, self.base + Duration::from_millis(ms))),
            DELAY,
            context,
        );
    }

    fn open(&mut self, wheel: Wheel) -> VisibleWheel {
        let button = if wheel == Wheel::Spells { SPELL } else { ITEM };
        self.sample(button, 1);
        self.sample(button, 251);
        self.state.open(wheel, || Some(START)).unwrap()
    }
}

#[test]
fn early_release_queues_one_tap_per_press_and_never_opens() {
    let mut pad = Pad::new();
    pad.sample(SPELL, 1);
    pad.sample(SPELL, 249);
    assert!(!pad.state.take_tap(Wheel::Spells));
    assert!(pad.state.open(Wheel::Spells, || None).is_none());
    pad.sample(0, 250);
    pad.sample(0, 250);
    assert!(pad.state.take_tap(Wheel::Spells));
    assert!(!pad.state.take_tap(Wheel::Spells));
    assert!(!pad.state.take_tap(Wheel::QuickItems));
    assert!(pad.state.open(Wheel::Spells, || None).is_none());
}

#[test]
fn rapid_repress_preserves_each_release_for_a_separate_hud_update() {
    let mut pad = Pad::new();
    pad.sample(SPELL, 1);
    pad.sample(0, 2);
    pad.sample(SPELL, 3);
    pad.sample(0, 4);
    assert!(pad.state.take_tap(Wheel::Spells));
    assert!(pad.state.take_tap(Wheel::Spells));
    assert!(!pad.state.take_tap(Wheel::Spells));
}

#[test]
fn release_at_or_after_threshold_without_a_held_sample_never_replays_as_tap() {
    for release in [251, 252] {
        let mut pad = Pad::new();
        pad.sample(ITEM, 1);
        pad.sample(0, release);
        assert!(!pad.state.take_tap(Wheel::QuickItems));
        assert!(pad.state.open(Wheel::QuickItems, || None).is_none());
    }
}

#[test]
fn hold_at_threshold_stays_consumed_through_repeated_samples_and_release() {
    let mut pad = Pad::new();
    let session = pad.open(Wheel::QuickItems);
    for ms in [251, 750, 1500] {
        pad.sample(ITEM, ms);
        assert_eq!(pad.state.visible(), Some(session));
        assert!(pad.state.open(Wheel::QuickItems, || None).is_none());
        assert!(!pad.state.take_tap(Wheel::QuickItems));
    }
    pad.sample(0, 1501);
    assert_eq!(pad.state.visible(), None);
    assert!(!pad.state.take_tap(Wheel::QuickItems));
    assert_eq!(
        pad.state
            .update_selection(false, |_, _| panic!("No highlight")),
        None
    );
    pad.sample(ITEM, 1502);
    pad.sample(0, 1503);
    assert!(pad.state.take_tap(Wheel::QuickItems));
}

#[test]
fn disabled_and_remapped_wheels_follow_the_current_binding_and_rearm_after_changes() {
    let mut pad = Pad::new();
    pad.sample(SPELL, 1);
    pad.state.set_bindings([REMAP, 0]);
    pad.sample(SPELL | ITEM | REMAP, 2);
    pad.sample(0, 3);
    assert!(!pad.state.take_tap(Wheel::Spells));
    assert!(!pad.state.take_tap(Wheel::QuickItems));

    pad.sample(SPELL | ITEM, 4);
    pad.sample(SPELL | ITEM, 300);
    assert!(pad.state.open(Wheel::Spells, || None).is_none());
    assert!(pad.state.open(Wheel::QuickItems, || None).is_none());
    pad.sample(REMAP, 301);
    pad.sample(0, 302);
    assert!(pad.state.take_tap(Wheel::Spells));
    assert!(!pad.state.take_tap(Wheel::QuickItems));
}

#[test]
fn zoom_owns_press_through_release_and_drops_unreplayed_taps() {
    let mut pad = Pad::new();
    pad.observe(Some(SPELL), 1, Context::Zoom);
    pad.sample(SPELL, 300);
    assert!(pad.state.zoom_owns(Wheel::Spells));
    assert!(pad.state.open(Wheel::Spells, || None).is_none());
    pad.sample(0, 301);
    pad.sample(0, 301);
    assert!(pad.state.zoom_owns(Wheel::Spells));
    pad.sample(0, 302);
    assert!(!pad.state.zoom_owns(Wheel::Spells));
    assert!(!pad.state.take_tap(Wheel::Spells));

    pad.sample(SPELL, 302);
    pad.observe(Some(SPELL), 303, Context::Zoom);
    pad.sample(0, 304);
    assert!(!pad.state.take_tap(Wheel::Spells));

    pad.sample(SPELL, 305);
    pad.sample(0, 306);
    pad.observe(Some(0), 306, Context::Zoom);
    assert!(!pad.state.take_tap(Wheel::Spells));

    pad.sample(SPELL, 307);
    pad.sample(0, 308);
    assert!(pad.state.take_tap(Wheel::Spells));
}

#[test]
fn focus_panel_and_disconnect_discard_pending_taps_and_require_up_before_rearming() {
    for (buttons, context) in [
        (Some(ITEM), Context::Interrupted),
        (None, Context::Gameplay),
    ] {
        let mut pad = Pad::new();
        pad.sample(ITEM, 1);
        pad.sample(0, 2);
        pad.sample(ITEM, 3);
        pad.observe(buttons, 4, context);
        pad.sample(ITEM, 300);
        assert!(pad.state.open(Wheel::QuickItems, || None).is_none());
        pad.sample(0, 301);
        assert!(!pad.state.take_tap(Wheel::QuickItems));
        pad.sample(ITEM, 302);
        pad.sample(0, 303);
        assert!(pad.state.take_tap(Wheel::QuickItems));
    }
}

#[test]
fn stable_slot_changes_sound_once_after_the_actual_commit_in_both_modes() {
    for (wheel, instant) in Wheel::ALL
        .into_iter()
        .flat_map(|wheel| [false, true].map(|instant| (wheel, instant)))
    {
        let mut pad = Pad::new();
        let session = pad.open(wheel);
        pad.state.highlight(session, NEXT);
        let mut current = START;
        let mut writes = 0;
        let mut access = |actual_wheel, slot| {
            assert_eq!(actual_wheel, wheel);
            if let Some(slot) = slot {
                current = slot;
                writes += 1;
            }
            Some(current)
        };
        assert_eq!(pad.state.update_selection(instant, &mut access), None);
        assert_eq!(
            access(wheel, None),
            Some(if instant { NEXT } else { START })
        );
        pad.sample(0, 252);
        pad.state.highlight(session, INVALID);
        assert_eq!(
            pad.state.update_selection(instant, &mut access),
            Some(wheel)
        );
        assert_eq!(pad.state.update_selection(instant, &mut access), None);
        assert_eq!(current, NEXT);
        assert_eq!(writes, 1);
    }
}

#[test]
fn unchanged_returned_invalid_and_rejected_selections_stay_silent() {
    for instant in [false, true] {
        for (final_highlight, final_index) in [(START, START), (INVALID, NEXT), (NEXT, START)] {
            let mut pad = Pad::new();
            let session = pad.open(Wheel::Spells);
            let mut current = START;
            pad.state.highlight(session, NEXT);
            assert_eq!(
                pad.state.update_selection(instant, |_, requested| {
                    current = requested.unwrap_or(current);
                    Some(current)
                }),
                None
            );

            pad.state.highlight(session, final_highlight);
            pad.sample(0, 252);
            assert_eq!(
                pad.state.update_selection(instant, |_, requested| {
                    match requested {
                        Some(INVALID) => None,
                        Some(slot) => {
                            current = if slot == NEXT { final_index } else { slot };
                            Some(current)
                        }
                        None => Some(final_index),
                    }
                }),
                None
            );
        }
    }
}

#[test]
fn zoom_or_disconnect_cancels_a_visible_wheel_and_keeps_only_completed_instant_writes() {
    for (buttons, context) in [
        (Some(SPELL), Context::Zoom),
        (None, Context::Gameplay),
        (Some(SPELL), Context::Interrupted),
    ] {
        for instant in [false, true] {
            let mut pad = Pad::new();
            let session = pad.open(Wheel::Spells);
            pad.state.highlight(session, NEXT);
            let mut current = START;
            pad.state.update_selection(instant, |_, slot| {
                current = slot.unwrap_or(current);
                Some(current)
            });
            pad.state.highlight(session, INVALID);
            pad.observe(buttons, 252, context);
            assert_eq!(pad.state.visible(), None);
            assert_eq!(
                pad.state
                    .update_selection(instant, |_, _| panic!("Cancelled wheel must not commit")),
                None
            );
            assert_eq!(current, if instant { NEXT } else { START });
            pad.sample(SPELL, 300);
            pad.sample(0, 301);
            assert!(!pad.state.take_tap(Wheel::Spells));
        }
    }
}

#[test]
fn rendered_selection_from_a_closed_generation_cannot_affect_the_next_one() {
    let mut pad = Pad::new();
    let old = pad.open(Wheel::Spells);
    pad.sample(0, 252);
    pad.state
        .update_selection(false, |_, _| panic!("No selection"));
    pad.sample(SPELL, 253);
    pad.sample(SPELL, 503);
    let new = pad.state.open(Wheel::Spells, || Some(START)).unwrap();
    assert_ne!(old, new);
    pad.state.highlight(old, NEXT);
    pad.sample(0, 504);
    assert_eq!(
        pad.state
            .update_selection(false, |_, _| panic!("Stale render")),
        None
    );
}

#[test]
fn each_hud_pass_limits_replay_to_the_native_checks_for_one_cycle() {
    for visible in [false, true] {
        let mut replay = TapReplay::new([true, true], visible);
        assert!(replay.request(Wheel::QuickItems));
        assert!(!replay.request(Wheel::QuickItems));
        assert!(replay.request(Wheel::Spells));
        assert_eq!(replay.request(Wheel::Spells), visible);
        assert!(!replay.request(Wheel::Spells));
        assert_eq!(replay, TapReplay::default());
    }
}

#[test]
fn angular_hysteresis_scales_with_slot_count_and_handles_the_seam() {
    for count in [2, 5, 10, 24] {
        let width = TAU / count as f32;
        assert!(retains_highlight(width * 0.54, 0.0, count));
        assert!(!retains_highlight(width * 0.59, 0.0, count));
        assert!(retains_highlight(-width * 0.54, 0.0, count));
        assert!(!retains_highlight(-width * 0.59, 0.0, count));

        let center = PI - width * 0.25;
        assert!(retains_highlight(-PI + width * 0.29, center, count));
        assert!(!retains_highlight(-PI + width * 0.34, center, count));
    }
    assert!(retains_highlight(PI, 0.0, 1));
}
