use std::cell::Cell;
use std::mem;
use std::sync::{Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

use fromsoftware_shared::Program;
use fromsoftware_shared::game_version::GameVersion;
use hudhook::windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use hudhook::windows::Win32::System::Threading::GetCurrentProcess;
use retour::GenericDetour;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::io::bindings::{get_item_bindings, get_spell_bindings};
use crate::io::controller_state::{TapReplay, Wheel};
use crate::settings::Settings;
use crate::settings::display_settings::display_settings_open;

use super::controller;
use super::hook_profiles::{
    HOLD, HUD_UPDATE_PREFIX, HookProfile, INPUT_CHECKS, ITEM_REPEAT, ITEM_REQUEST, PROFILES,
    ProbeRetry, SPELL_REPEAT, SPELL_REQUEST, select_profile,
};

type InputCheck = unsafe extern "system" fn(usize, usize, usize, usize) -> bool;
type HudUpdate = unsafe extern "system" fn(usize, usize, usize);

const HUD_SPELL_VISIBLE_OFFSET: usize = 0x4D;

static PROBE_RETRY: Mutex<ProbeRetry> = Mutex::new(ProbeRetry::new());
static PROFILE: OnceLock<&'static HookProfile> = OnceLock::new();
static FIRST_INPUT: [Once; INPUT_CHECKS.len()] = [const { Once::new() }; INPUT_CHECKS.len()];
static FIRST_HUD: Once = Once::new();
static INPUT_HOOKS: [OnceLock<GenericDetour<InputCheck>>; INPUT_CHECKS.len()] =
    [const { OnceLock::new() }; INPUT_CHECKS.len()];
static HUD_HOOK: OnceLock<GenericDetour<HudUpdate>> = OnceLock::new();

thread_local! {
    static HUD_REPLAY: Cell<Option<TapReplay>> = const { Cell::new(None) };
}

fn take_replay(wheel: Wheel) -> bool {
    HUD_REPLAY.with(|scope| {
        let Some(mut replay) = scope.get() else {
            return false;
        };
        let requested = replay.request(wheel);
        scope.set(Some(replay));
        requested
    })
}

unsafe extern "system" fn input_hook<const I: usize>(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
) -> bool {
    FIRST_INPUT[I]
        .call_once(|| tracing::info!(hook = INPUT_CHECKS[I].0, "Controller input hook called"));
    if display_settings_open() {
        return false;
    }
    let original = INPUT_HOOKS[I].get().unwrap();
    let wheel = match (I, b as u32) {
        (SPELL_REQUEST | SPELL_REPEAT, _) | (HOLD, 13) => Wheel::Spells,
        (ITEM_REQUEST | ITEM_REPEAT, _) | (HOLD, 14) => Wheel::QuickItems,
        _ => return original.call(a, b, c, d),
    };

    if matches!(I, SPELL_REQUEST | ITEM_REQUEST)
        && take_replay(wheel)
        && controller::can_replay_tap()
    {
        original.call(a, b, c, d);
        return true;
    }

    let settings = Settings::read_or_default();
    let (enabled, key, bindings) = match wheel {
        Wheel::Spells => (
            settings.spell_wheel_enabled,
            &settings.spells_key,
            get_spell_bindings(),
        ),
        Wheel::QuickItems => (
            settings.item_wheel_enabled,
            &settings.quick_items_key,
            get_item_bindings(),
        ),
    };
    let pressed = bindings.keyboard_mouse_status();
    let native_keyboard_mouse = pressed.scroll()
        || ((pressed.key() || pressed.mouse()) && !key.eq_ignore_ascii_case("ingame"));
    if !enabled
        || native_keyboard_mouse
        || controller::native_passthrough(wheel, bindings.pad, controller_hooks_ready(wheel))
    {
        return original.call(a, b, c, d);
    }
    false
}

unsafe fn spell_visible(hud_context: usize) -> Option<bool> {
    if hud_context == 0 {
        return None;
    }
    let state = usize::from_ne_bytes(read_game_value(hud_context)?);
    if state == 0 {
        return None;
    }
    read_game_value(state + HUD_SPELL_VISIBLE_OFFSET).map(|[visible]| visible != 0)
}

unsafe extern "system" fn hud_update(hud_context: usize, b: usize, c: usize) {
    FIRST_HUD.call_once(|| tracing::info!("Equipment HUD hook called"));
    let original = HUD_HOOK.get().unwrap();
    if let Some(outer) = HUD_REPLAY.with(|scope| scope.replace(Some(TapReplay::default()))) {
        original.call(hud_context, b, c);
        HUD_REPLAY.with(|scope| scope.set(Some(outer)));
        return;
    }

    let settings = Settings::read_or_default();
    let mut taps = controller::take_taps(&settings);
    let spell_hud = if taps[Wheel::Spells.index()] {
        spell_visible(hud_context)
    } else {
        None
    };
    if taps[Wheel::Spells.index()] && spell_hud.is_none() {
        tracing::debug!("Dropped controller spell tap because HUD visibility is unavailable");
        taps[Wheel::Spells.index()] = false;
    }
    HUD_REPLAY.with(|scope| scope.set(Some(TapReplay::new(taps, spell_hud.unwrap_or(false)))));
    original.call(hud_context, b, c);
    let unused = HUD_REPLAY
        .with(|scope| scope.replace(None))
        .unwrap_or_default();
    if settings.debugging && taps.into_iter().any(|tap| tap) {
        tracing::info!(
            ?taps,
            ?spell_hud,
            ?unused,
            "Controller tap during equipment HUD update"
        );
    }
}

const INPUT_FNS: [InputCheck; INPUT_CHECKS.len()] = [
    input_hook::<SPELL_REQUEST>,
    input_hook::<ITEM_REQUEST>,
    input_hook::<SPELL_REPEAT>,
    input_hook::<ITEM_REPEAT>,
    input_hook::<HOLD>,
];

pub(super) fn controller_hooks_ready(wheel: Wheel) -> bool {
    let pair = match wheel {
        Wheel::Spells => [SPELL_REQUEST, SPELL_REPEAT],
        Wheel::QuickItems => [ITEM_REQUEST, ITEM_REPEAT],
    };
    HUD_HOOK.get().is_some_and(GenericDetour::is_enabled)
        && [pair[0], pair[1], HOLD]
            .into_iter()
            .all(|i| INPUT_HOOKS[i].get().is_some_and(GenericDetour::is_enabled))
}

pub(super) fn module_base() -> usize {
    unsafe { GetModuleHandleW(std::ptr::null()) as usize }
}

fn read_game_memory(address: usize, data: &mut [u8]) -> Result<(), String> {
    let mut bytes_read = 0;
    unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const _,
            data.as_mut_ptr().cast(),
            data.len(),
            Some(&mut bytes_read),
        )
    }
    .map_err(|err| err.to_string())?;
    if bytes_read == data.len() {
        Ok(())
    } else {
        Err(format!("read {bytes_read} of {} bytes", data.len()))
    }
}

pub(super) fn read_game_value<const N: usize>(address: usize) -> Option<[u8; N]> {
    let mut data = [0; N];
    read_game_memory(address, &mut data).ok()?;
    Some(data)
}

pub(super) fn matches_prefix(address: usize, prefix: &[u8]) -> bool {
    let mut data = [0; 16];
    let data = &mut data[..prefix.len()];
    read_game_memory(address, data).is_ok() && data == prefix
}

#[derive(Debug)]
struct ExecutableVersion {
    product_version: String,
    language: u16,
}

impl GameVersion for ExecutableVersion {
    const NAME: &'static str = "elden ring";

    fn from_lang_version(language: u16, version: &str) -> Option<Self> {
        Some(Self {
            product_version: version.to_owned(),
            language,
        })
    }
}

fn log_build() {
    let file_size = std::fs::metadata(crate::paths::game())
        .ok()
        .map(|file| file.len());
    if let Program::Mapping(image) = Program::current() {
        match ExecutableVersion::detect(&image) {
            Ok(version) => tracing::info!(
                product_version = %version.product_version,
                language = version.language,
                ?file_size,
                candidate = option_env!("SPELLWHEEL_DIAGNOSTIC_REVISION").unwrap_or("unrecorded"),
                "Executable version for controller hooks"
            ),
            Err(err) => tracing::warn!(%err, "Could not identify the executable version"),
        }
    }
}

fn log_unmatched(base: usize) {
    for profile in &PROFILES {
        for (name, rva, expected) in profile.signatures() {
            let mut actual = [0; 16];
            let actual = &mut actual[..expected.len()];
            match read_game_memory(base + rva, actual) {
                Ok(()) if actual == expected => {}
                Ok(()) => tracing::warn!(
                    profile = profile.name,
                    hook = name,
                    rva = format_args!("{rva:#x}"),
                    expected = ?expected,
                    actual = ?actual,
                    "Controller hook byte mismatch"
                ),
                Err(err) => tracing::warn!(
                    profile = profile.name,
                    hook = name,
                    rva = format_args!("{rva:#x}"),
                    %err,
                    "Could not read controller hook bytes"
                ),
            }
        }
    }
    tracing::warn!("Controller hook profile unavailable; will retry");
}

pub(super) fn profile() -> Option<&'static HookProfile> {
    PROFILE.get().copied()
}

pub fn install_io_hooks() {
    if PROFILE.get().is_some() {
        return;
    }
    let mut retry = PROBE_RETRY.lock().unwrap();
    if PROFILE.get().is_some() {
        return;
    }
    let Some(first_probe) = retry.poll(Instant::now()) else {
        return;
    };
    let base = module_base();
    if first_probe {
        log_build();
    }
    let Some(profile) = select_profile(|rva, prefix| matches_prefix(base + rva, prefix)) else {
        if first_probe || retry.should_report(Instant::now(), Duration::from_secs(10)) {
            log_unmatched(base);
        }
        return;
    };
    unsafe {
        let _ = PROFILE.set(profile);
        tracing::info!(
            profile = profile.name,
            "Matched all six controller hook signatures"
        );

        for (i, (name, rva, prefix)) in profile.signatures().take(INPUT_CHECKS.len()).enumerate() {
            let address = base + rva;
            if !matches_prefix(address, prefix) {
                tracing::warn!("Couldn't hook {name}: prologue changed at RVA {rva:#x}");
                continue;
            }
            let target = mem::transmute::<usize, InputCheck>(address);
            match GenericDetour::new(target, INPUT_FNS[i]) {
                Ok(detour) => {
                    let detour = INPUT_HOOKS[i].get_or_init(|| detour);
                    if let Err(err) = detour.enable() {
                        tracing::warn!("Couldn't enable {name} hook: {err}");
                    }
                }
                Err(err) => tracing::warn!("Couldn't create {name} hook: {err}"),
            }
        }

        let address = base + profile.hud;
        if !matches_prefix(address, HUD_UPDATE_PREFIX) {
            tracing::warn!(
                "Equipment HUD update prologue mismatch; controller wheels use native input"
            );
            return;
        }
        let target = mem::transmute::<usize, HudUpdate>(address);
        match GenericDetour::new(target, hud_update as HudUpdate) {
            Ok(detour) => {
                let detour = HUD_HOOK.get_or_init(|| detour);
                if let Err(err) = detour.enable() {
                    tracing::warn!("Couldn't enable equipment HUD hook: {err}");
                }
            }
            Err(err) => tracing::warn!("Couldn't create equipment HUD hook: {err}"),
        }
        tracing::info!(
            spells = controller_hooks_ready(Wheel::Spells),
            quick_items = controller_hooks_ready(Wheel::QuickItems),
            "Controller switch hooks ready",
        );
    }
}

pub fn uninstall_io_hooks() {
    unsafe {
        if let Some(hook) = HUD_HOOK.get() {
            let _ = hook.disable();
        }
        for hook in INPUT_HOOKS.iter().filter_map(OnceLock::get) {
            let _ = hook.disable();
        }
    }
}
