use std::sync::{Once, OnceLock};

use super::controller_state::Wheel;
use super::hook::{module_base, profile, read_game_value};

// The game uses this same menu event for both equipment cycles.
// This entry point takes only a sound event and never receives equipment data.
const SOUND_PREFIX: [u8; 12] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x83, 0x39, 0x00, 0x48, 0x8B, 0xD9,
];
const EQUIPMENT_CHANGE_EVENT_ID: i32 = 0x2710;

#[repr(C)]
struct MenuSoundEvent {
    id: i32,
    target: i32,
    category: i32,
    padding: i32,
    vtable: usize,
}

type PlaySound = unsafe extern "system" fn(*mut MenuSoundEvent);
static SOUND: OnceLock<(PlaySound, usize)> = OnceLock::new();
static FIRST_PLAYBACK: [Once; Wheel::ALL.len()] = [const { Once::new() }; Wheel::ALL.len()];

fn resolve() -> Option<(PlaySound, usize)> {
    if let Some(&sound) = SOUND.get() {
        return Some(sound);
    }

    let profile = profile()?;
    let base = module_base();
    let address = base + profile.sound;
    let actual = read_game_value(address);
    if actual != Some(SOUND_PREFIX) {
        tracing::warn!(
            profile = profile.name,
            rva = format_args!("{:#x}", profile.sound),
            expected = %format_args!("{SOUND_PREFIX:02x?}"),
            actual = %format_args!("{actual:02x?}"),
            "Equipment sound routine signature unavailable"
        );
        return None;
    }

    // Each profile names the event table used by its sound routine. The table
    // must be readable, but entries are not required to point below it in the image.
    let vtable = base + profile.sound_vtable;
    if read_game_value::<{ size_of::<usize>() }>(vtable).is_none() {
        tracing::warn!(
            profile = profile.name,
            rva = format_args!("{:#x}", profile.sound_vtable),
            "Could not read equipment sound event table"
        );
        return None;
    }

    let play = unsafe { std::mem::transmute::<usize, PlaySound>(address) };
    Some(*SOUND.get_or_init(|| (play, vtable)))
}

pub fn play(wheel: Wheel) {
    let Some((play, vtable)) = resolve() else {
        return;
    };

    let mut event = MenuSoundEvent {
        id: EQUIPMENT_CHANGE_EVENT_ID,
        target: -1,
        category: 3,
        padding: 0,
        vtable,
    };
    unsafe { play(&mut event) };
    FIRST_PLAYBACK[wheel.index()].call_once(|| {
        tracing::info!(
            ?wheel,
            "Called the native equipment sound routine on wheel close"
        );
    });
}
