use std::time::{Duration, Instant};

pub const INPUT_CHECK_PREFIX: &[u8] = &[
    0x4C, 0x8B, 0xDC, 0x48, 0x83, 0xEC, 0x78, 0x49, 0xC7, 0x43, 0xA8,
];
pub const SWITCH_HOLD_PREFIX: &[u8] = &[
    0x4C, 0x8B, 0xDC, 0x48, 0x81, 0xEC, 0x88, 0x00, 0x00, 0x00, 0x49, 0xC7, 0x43, 0x98,
];
pub const HUD_UPDATE_PREFIX: &[u8] = &[
    0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x56, 0x57,
];

pub const SPELL_REQUEST: usize = 0;
pub const ITEM_REQUEST: usize = 1;
pub const SPELL_REPEAT: usize = 2;
pub const ITEM_REPEAT: usize = 3;
pub const HOLD: usize = 4;

pub const INPUT_CHECKS: [(&str, &[u8]); 5] = [
    ("Switch Spell request", INPUT_CHECK_PREFIX),
    ("Switch Item request", INPUT_CHECK_PREFIX),
    ("Switch Spell repeat", INPUT_CHECK_PREFIX),
    ("Switch Item repeat", INPUT_CHECK_PREFIX),
    ("Switch Spell / Item hold", SWITCH_HOLD_PREFIX),
];

pub struct HookProfile {
    pub name: &'static str,
    pub input: [usize; INPUT_CHECKS.len()],
    pub hud: usize,
    pub sound: usize,
    pub sound_vtable: usize,
}

// RadialMenu recorded the Tarnished Edition addresses in a376711 and the 2.7.2.0
// addresses in 9f18b00. Select a complete set from the loaded executable.
pub const PROFILES: [HookProfile; 2] = [
    HookProfile {
        name: "1.17 (Tarnished Edition)",
        input: [0x759120, 0x7590B0, 0x759680, 0x7593D0, 0x759270],
        hud: 0x776530,
        sound: 0x815D50,
        sound_vtable: 0x2A9EDD0,
    },
    HookProfile {
        name: "1.17.2 (2.7.2.0)",
        input: [0x7595D0, 0x759560, 0x759B30, 0x759880, 0x759720],
        hud: 0x7769E0,
        sound: 0x816200,
        sound_vtable: 0x2AA2D40,
    },
];

impl HookProfile {
    pub fn signatures(&self) -> impl Iterator<Item = (&'static str, usize, &'static [u8])> {
        self.input
            .into_iter()
            .zip(INPUT_CHECKS)
            .map(|(rva, (name, prefix))| (name, rva, prefix))
            .chain([("Equipment HUD update", self.hud, HUD_UPDATE_PREFIX)])
    }
}

pub fn select_profile(
    mut matches: impl FnMut(usize, &[u8]) -> bool,
) -> Option<&'static HookProfile> {
    let mut candidates = PROFILES.iter().filter(|profile| {
        profile
            .signatures()
            .all(|(_, rva, prefix)| matches(rva, prefix))
    });
    let selected = candidates.next()?;
    if candidates.next().is_none() {
        Some(selected)
    } else {
        None
    }
}

pub struct ProbeRetry {
    last_probe: Option<Instant>,
    last_report: Option<Instant>,
}

impl ProbeRetry {
    pub const fn new() -> Self {
        Self {
            last_probe: None,
            last_report: None,
        }
    }

    pub fn poll(&mut self, now: Instant) -> Option<bool> {
        if self
            .last_probe
            .is_some_and(|last| now - last < Duration::from_secs(1))
        {
            return None;
        }
        let first = self.last_probe.replace(now).is_none();
        if first {
            self.last_report = Some(now);
        }
        Some(first)
    }

    pub fn should_report(&mut self, now: Instant, interval: Duration) -> bool {
        if self.last_report.is_some_and(|last| now - last < interval) {
            return false;
        }
        self.last_report = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARNISHED: [(usize, &[u8]); 6] = [
        (0x759120, INPUT_CHECK_PREFIX),
        (0x7590B0, INPUT_CHECK_PREFIX),
        (0x759680, INPUT_CHECK_PREFIX),
        (0x7593D0, INPUT_CHECK_PREFIX),
        (0x759270, SWITCH_HOLD_PREFIX),
        (0x776530, HUD_UPDATE_PREFIX),
    ];

    const LATER: [(usize, &[u8]); 6] = [
        (0x7595D0, INPUT_CHECK_PREFIX),
        (0x759560, INPUT_CHECK_PREFIX),
        (0x759B30, INPUT_CHECK_PREFIX),
        (0x759880, INPUT_CHECK_PREFIX),
        (0x759720, SWITCH_HOLD_PREFIX),
        (0x7769E0, HUD_UPDATE_PREFIX),
    ];

    #[test]
    fn an_initial_mismatch_does_not_disable_later_installation() {
        let start = Instant::now();
        let mut retry = ProbeRetry::new();
        assert_eq!(retry.poll(start), Some(true));
        assert!(select_profile(|_, _| false).is_none());
        assert_eq!(retry.poll(start + Duration::from_millis(500)), None);
        assert_eq!(retry.poll(start + Duration::from_secs(1)), Some(false));
        assert!(select_profile(|rva, bytes| LATER.contains(&(rva, bytes))).is_some());
        assert!(!retry.should_report(start + Duration::from_secs(8), Duration::from_secs(10)));
        assert!(retry.should_report(start + Duration::from_secs(10), Duration::from_secs(10)));
    }

    #[test]
    fn tarnished_edition_selects_its_own_input_and_sound_addresses() {
        let profile = select_profile(|rva, bytes| TARNISHED.contains(&(rva, bytes))).unwrap();
        assert_eq!((profile.sound, profile.sound_vtable), (0x815D50, 0x2A9EDD0));
    }

    #[test]
    fn later_executable_still_selects_its_original_addresses() {
        let profile = select_profile(|rva, bytes| LATER.contains(&(rva, bytes))).unwrap();
        assert_eq!((profile.sound, profile.sound_vtable), (0x816200, 0x2AA2D40));
    }

    #[test]
    fn partial_mixed_and_ambiguous_matches_are_rejected() {
        assert!(select_profile(|_, _| false).is_none());
        for excluded in 0..TARNISHED.len() {
            assert!(
                select_profile(|rva, bytes| {
                    TARNISHED[excluded].0 != rva && TARNISHED.contains(&(rva, bytes))
                })
                .is_none()
            );
        }
        assert!(
            select_profile(|rva, bytes| {
                TARNISHED[..5].contains(&(rva, bytes)) || LATER[5] == (rva, bytes)
            })
            .is_none()
        );
        assert!(
            select_profile(|rva, bytes| {
                TARNISHED.contains(&(rva, bytes)) || LATER.contains(&(rva, bytes))
            })
            .is_none()
        );
    }
}
