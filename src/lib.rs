#![allow(unsafe_op_in_unsafe_fn)]

mod rendering;
#[cfg(feature = "extra-memory-slots-support")]
mod expanded_memory_slots;
mod debugging;
mod keyboard;
mod icons;
mod settings;
mod wheel_details;
mod wheel_details_data;
pub mod items;
pub mod paths;
pub mod gamepad;
pub mod xinput_hook;
pub mod await_seamless;
pub mod mouse;
pub mod hwindow;
pub mod dynamic_icons;
pub mod util;
pub mod io;
pub mod font;
pub mod glyphs;

use crate::await_seamless::{await_seamless, is_seamless_coop_active};
use crate::debugging::{add_to_screen_debug, commit_screen_debug, is_debugging, run_every, run_once};
use crate::io::controller;
use crate::io::controller_state::{Slot, Wheel};
use crate::glyphs::font_manager::FontManager;
use crate::icons::icon_manager::IconManager;
use crate::rendering::{remove_hudhook, try_init_rendering};
use eldenring::cs::{CSFeManHudState, CSFeManImp, CSMenuManImp, CSTaskGroupIndex, CSTaskImp, EquipGameData, GameDataMan, Magic, SoloParam, SoloParamRepository, WorldChrManDbg};
use eldenring::fd4::FD4TaskData;
use eldenring::util::system::wait_for_system_init;
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use hudhook::hooks::dx12::get_hwnd;
use hudhook::windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use io::hook::{install_io_hooks, uninstall_io_hooks};
use lazy_static::lazy_static;
use rendering::render_hook::ItemWheelData;
use rendering::wheel_renderer::WheelType;
use std::fs::File;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tracing_subscriber::fmt;
#[cfg(feature = "extra-memory-slots-support")]
use crate::expanded_memory_slots::SelectionResult;
use crate::items::Item;
use crate::keyboard::hook::{install_keyboard_and_mouse_hooks, reset_scroll};
use crate::settings::Settings;
use crate::settings::display_settings::display_settings_open;
use crate::wheel_details_data::{SpellFp, WheelDetailsData, details_enabled, read_item_availability};
use crate::xinput_hook::{install_xinput_hook, remove_xinput_hook, set_suppress_camera};
use keyboard::listener::tick_listeners;

static HMODULE: OnceLock<usize> = OnceLock::new();

pub fn hmodule() -> usize {
    *HMODULE.get().expect("Could not get HMODULE")
}

static HOOKED_HWND: OnceLock<usize> = OnceLock::new();

pub fn set_hwnd() {
    if let Err(hwnd) = HOOKED_HWND.set(*get_hwnd().expect("Couldn't find HWND")) {
        tracing::error!("HWND already initialized: {hwnd}");
    }
}

pub fn hwnd() -> windows::Win32::Foundation::HWND {
    unsafe {
        mem::transmute(*HOOKED_HWND.get().expect("Could not get HWND"))
    }
}

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
/// # Safety
///
/// This is exposed this way such that windows LoadLibrary API can call it. Do not call this yourself.
pub unsafe extern "C" fn DllMain(hmodule: usize, reason: u32) -> bool {
    match reason {
        DLL_PROCESS_ATTACH => { std::thread::spawn(move || init(hmodule)); },
        DLL_PROCESS_DETACH => deinit(),
        _ => {}
    }

    true
}

fn init(hmodule: usize) {
    HMODULE.set(hmodule).expect("Could not set HMODULE");
    // Fix for Seamless crash
    std::thread::sleep(Duration::from_secs_f32(Settings::read_or_default().timing_offset));
    if is_seamless_coop_active() {
        await_seamless();
    }

    fmt().with_writer(File::create(paths::log()).expect("Could not create log file"))
        .with_ansi(false)
        .init();
    tracing::info!("Log file created");
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        tracing::error!("Encountered an error:\n{msg}");
    }));

    guard!(
        start();
    );
}

fn deinit() {
    tracing::info!("DLL detach called");
    remove_hudhook();
    remove_xinput_hook();
    uninstall_io_hooks();
}

lazy_static!(
    static ref TICK: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
);

pub fn get_tick() -> u64 {
    TICK.load(Ordering::Relaxed)
}

#[macro_export]
macro_rules! guard {
    ($($t:tt)*) => {
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            $($t)*
        }));
        if let Err(err) = out {
            tracing::error!("Encountered an error:\n{}", panic_message::panic_message(&err));
        }
    };
}

fn start() {
    #[cfg(feature = "settings-diagnostics")]
    {
        let marker = paths::dll_folder().join("spellwheel-settings-diagnostics.enable");
        if marker.is_file() {
            let log = paths::spellwheel().join(format!("settings-diagnostics-{}-{:x}.log", std::process::id(), hmodule()));
            match hudhook::settings_diagnostics::enable(&log) {
                Ok(()) => {
                    hudhook::settings_diagnostics::event(format_args!(
                        "module_start base={:x} dll={:?} build={} settings={:?}", hmodule(), paths::dll(),
                        option_env!("SPELLWHEEL_DIAGNOSTIC_REVISION").unwrap_or("unrecorded"), paths::settings()));
                    tracing::info!("Settings diagnostics enabled: {log:?}");
                }
                Err(err) => tracing::error!("Could not enable settings diagnostics: {err}"),
            }
        }
    }
    tracing::info!("DLL Path: {:?}", paths::dll());
    tracing::info!("Game Path: {:?}", paths::game());
    tracing::info!("Awaiting system init");
    wait_for_system_init(&Program::current(), Duration::MAX)
        .expect("Could not await system init.");

    if !Settings::read_or_default().await_xinput_hook {
        install_xinput_hook();
    }

    tracing::info!("Initializing IconManager and FontManager asynchronously");
    std::thread::spawn(IconManager::init);
    std::thread::spawn(FontManager::init);

    tracing::info!("Init complete");
    let tasks = unsafe { CSTaskImp::instance() }.expect("Could not get CSTaskImp");
    tracing::info!("Running task");
    tasks.run_recurring(
        tick,
        CSTaskGroupIndex::MenuMan
    );
}

lazy_static!(
    static ref SELECTED_SPELL_INDEX: AtomicI32 = AtomicI32::new(-1);
    static ref SELECTED_QUICK_ITEM_INDEX: AtomicI32 = AtomicI32::new(-1);
);

pub fn set_selected_spell_index(idx: i32) {
    SELECTED_SPELL_INDEX.store(idx, Ordering::Relaxed);
}

pub fn set_selected_quick_item_index(idx: i32) {
    SELECTED_QUICK_ITEM_INDEX.store(idx, Ordering::Relaxed);
}

// One may ask why I'm creating atomic structs in lazy statics.
// Thour't a cunning fella, I assure ye! Thy suspicions are not misplaced.
// Though alas, thou art mistaken.
// For ye see, were I not to do this, one would find that:
//
// IN_MENUS.store(false, Ordering::Release);
// assert!(!in_menus());
//
// fails.
lazy_static!(
    static ref IN_MENUS: AtomicBool = AtomicBool::new(true);
);

pub fn in_menus() -> bool {
    IN_MENUS.load(Ordering::Acquire)
}

pub fn set_in_menus(world_chr_man_dbg: &WorldChrManDbg, fe_man: &CSFeManImp) {
    let not_in_game = world_chr_man_dbg.player_session_holder.is_none();
    let paused = !matches!(fe_man.hud_state, CSFeManHudState::Default);
    if is_debugging() {
        add_to_screen_debug(format!("pre-game menu: {not_in_game}"));
        add_to_screen_debug(format!("in-game menu: {paused}"));
    }
    IN_MENUS.store(
        not_in_game || paused,
        Ordering::Release
    );
}

lazy_static!(
    static ref PREV_WHEEL_TYPE: Arc<Mutex<WheelType>> = Arc::new(Mutex::new(WheelType::None));
    static ref PREV_DISPLAY_SETTINGS_OPEN: AtomicBool = AtomicBool::new(false);
);
fn select_spell(equipment: &mut EquipGameData, index: i32) -> bool {
    let magic = &mut equipment.equip_magic_data;
    #[cfg(feature = "extra-memory-slots-support")]
    match expanded_memory_slots::select_spell(magic.as_ptr().cast(), index) {
        SelectionResult::Applied => return true,
        SelectionResult::Rejected => {
            tracing::warn!("Expanded Memory Slots rejected spell index {index}");
            return false;
        }
        SelectionResult::Unavailable => {}
    }
    magic.selected_slot = index;
    true
}

fn active_slot(equipment: &EquipGameData, wheel: Wheel, slots: &[(usize, u32)]) -> Option<Slot> {
    let index = match wheel {
        Wheel::Spells => {
            let magic = &equipment.equip_magic_data;
            #[cfg(feature = "extra-memory-slots-support")]
            let index = expanded_memory_slots::selected_slot(magic.as_ptr().cast(), magic.selected_slot)?;
            #[cfg(not(feature = "extra-memory-slots-support"))]
            let index = magic.selected_slot;
            index
        }
        Wheel::QuickItems => equipment.equip_item_data.selected_quick_slot,
    };
    slots.iter().find(|(slot, _)| *slot == index as usize)
        .map(|&(_, param_id)| Slot { index, param_id })
}

fn interrupt_gameplay() {
    controller::interrupt();
    ItemWheelData::mutate(|data| data.details = WheelDetailsData::default());
    set_suppress_camera(false);
    SELECTED_SPELL_INDEX.store(-1, Ordering::Relaxed);
    SELECTED_QUICK_ITEM_INDEX.store(-1, Ordering::Relaxed);
}

fn tick(_fd4: &FD4TaskData) {
    guard!(
        run_once!("entered tick function" => {
            tracing::info!("Entered tick function");
        });
        if is_debugging() {
            run_every!("D tick" every Duration::from_secs(1) => {
                tracing::info!("In tick function");
            });
        }
        let Some(world_chr_man_dbg) = unsafe { WorldChrManDbg::instance() }.ok() else {
            interrupt_gameplay();
            return;
        };

        let Some(fe_man) = unsafe { CSFeManImp::instance() }.ok() else {
            interrupt_gameplay();
            return;
        };
        set_in_menus(world_chr_man_dbg, fe_man);

        let Some(game_data_man) = unsafe { GameDataMan::instance_mut() }.ok() else {
            interrupt_gameplay();
            return;
        };

        let Some(param_repo) = unsafe { SoloParamRepository::instance_mut() }.ok() else {
            interrupt_gameplay();
            return;
        };

        let Some(menu_man) = unsafe { CSMenuManImp::instance_mut() }.ok() else {
            interrupt_gameplay();
            return;
        };

        if param_repo.solo_param_holders[Magic::INDEX as usize].get_res_cap(0).is_none() {
            interrupt_gameplay();
            return;
        }
        run_once!("passed all checks" => {
            tracing::info!("Passed all checks");
        });
        if is_debugging() {
            run_every!("D passed all checks" every Duration::from_secs(1) => {
                tracing::info!("Passed all checks");
            });
        }
        let settings = Settings::read_or_default();
        if settings.await_xinput_hook {
            install_xinput_hook();
        }
        install_keyboard_and_mouse_hooks();
        try_init_rendering();
        tick_listeners();
        install_io_hooks();

        let selected_spell_index = SELECTED_SPELL_INDEX.swap(-1, Ordering::Relaxed);
        let selected_quick_item_index = SELECTED_QUICK_ITEM_INDEX.swap(-1, Ordering::Relaxed);
        let player_data = &mut *game_data_man.main_player_game_data;
        let player_data_ptr = std::ptr::NonNull::from(&*player_data);
        let equipment = &mut player_data.equipment;
        if !display_settings_open() && !in_menus() {
            if selected_spell_index != -1 {
                select_spell(equipment, selected_spell_index);
            }
            if selected_quick_item_index != -1 {
                equipment.equip_item_data.selected_quick_slot = selected_quick_item_index;
            }
        }

        #[cfg(feature = "extra-memory-slots-support")]
        let mut equipped_spells = Vec::with_capacity(expanded_memory_slots::MAX_SPELL_RECORDS);
        #[cfg(not(feature = "extra-memory-slots-support"))]
        let mut equipped_spells = Vec::with_capacity(14);
        let mut equipped_quick_items = Vec::with_capacity(10);

        let equip_magic_data = &equipment.equip_magic_data;
        #[cfg(feature = "extra-memory-slots-support")]
        let spell_data = expanded_memory_slots::spell_ids(equip_magic_data.as_ptr().cast())
            .unwrap_or_else(|| equip_magic_data
                .entries
                .iter()
                .map(|entry| entry.param_id as u32)
                .enumerate()
                .filter(|(_idx, id)| *id != !0)
                .collect()
            );
        #[cfg(not(feature = "extra-memory-slots-support"))]
        let spell_data = equip_magic_data
                .entries
                .iter()
                .map(|entry| entry.param_id as u32)
                .enumerate()
                .filter(|(_idx, id)| *id != !0)
                .collect();
        let quick_item_data: Vec<_> = equipment
            .equipment_entries
            .quick_tems
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| entry.param_id().map(|param_id| (idx, param_id)))
            .filter(|(_idx, id)| *id != !0)
            .collect();

        let (keyboard_wheel, controller_bindings) = io::wheel_inputs(&settings);
        let (controller_wheel, close_sound) = controller::update_wheel(
            &settings, controller_bindings, keyboard_wheel != WheelType::None,
            |wheel, requested| {
                let slots = match wheel {
                    Wheel::Spells => &spell_data,
                    Wheel::QuickItems => &quick_item_data,
                };
                if let Some(slot) = requested {
                    if !slots.contains(&(slot.index as usize, slot.param_id)) {
                        return None;
                    }
                    match wheel {
                        Wheel::Spells if !select_spell(equipment, slot.index) => return None,
                        Wheel::QuickItems => equipment.equip_item_data.selected_quick_slot = slot.index,
                        Wheel::Spells => {}
                    }
                }
                active_slot(equipment, wheel, slots)
            },
        );
        if let Some(wheel) = close_sound
            && !display_settings_open() && io::is_game_focused()
        {
            if settings.debugging {
                tracing::info!(?wheel, "Controller wheel closed with a changed active slot");
            }
            io::selection_sound::play(wheel);
        }
        let selected_wheel_type = controller_wheel.map_or(keyboard_wheel, |visible| visible.wheel.into());

        let preview_open = display_settings_open();
        let mut details = WheelDetailsData::default();
        let read_spells = details_enabled(&settings, WheelType::Spells)
            && (selected_wheel_type == WheelType::Spells || preview_open);
        let item_card_enabled = details_enabled(&settings, WheelType::QuickItems);
        let read_items = (item_card_enabled || settings.wheel_details_item_quantity)
            && (selected_wheel_type == WheelType::QuickItems || preview_open);
        if (read_spells || read_items)
            && !in_menus()
            && let Ok(world) = unsafe { eldenring::cs::WorldChrMan::instance() }
            && let Some(player) = world.main_player.as_ref()
            && player.player_game_data == player_data_ptr
        {
            details.player_context = Some(player_data_ptr.as_ptr() as usize);
            if read_spells {
                details.active_spell = active_slot(equipment, Wheel::Spells, &spell_data);
                let stats = [
                    player_data.effective_intelligence,
                    player_data.effective_faith,
                    player_data.effective_arcane,
                ]
                .map(|value| (value > 0).then_some(value));
                let fp = (settings.wheel_details_fp_cost || settings.wheel_details_availability)
                    .then(|| SpellFp::read(&player.chr_ins)).flatten();
                details.read_spells(param_repo, &spell_data, stats, fp.as_ref(), &settings);
            }
            if read_items {
                details.active_item = active_slot(equipment, Wheel::QuickItems, &quick_item_data);
                let availability = if item_card_enabled && settings.wheel_details_availability {
                    read_item_availability(world, &player.chr_ins, equipment, player_data.furlcalling_finger_remedy_active)
                } else {
                    Default::default()
                };
                details.read_quick_items(param_repo, equipment, &quick_item_data, &settings, availability);
            }
        }

        for (equipped, data) in [
            (&mut equipped_spells, spell_data), (&mut equipped_quick_items, quick_item_data)
        ] {
            for (idx, id) in data {
                match Item::try_new(param_repo, idx as i32, id) {
                    Ok(item) => equipped.push(item),
                    Err(err) => {
                        run_every!(format!("I invalid item {id} at index {idx}"); every Duration::from_secs(1) => {
                            tracing::warn!("Error creating item with ID {id} at index {idx}: {err}");
                        });
                    }
                }
            }
        }

        if is_debugging() {
            add_to_screen_debug(format!("Equipped spells: {equipped_spells:?}"));
            add_to_screen_debug(format!("Equipped quick items: {equipped_quick_items:?}"));

            run_every!("D equipped" every Duration::from_secs(1) => {
                tracing::info!("Equipped spells: {equipped_spells:?}");
                tracing::info!("Equipped quick items: {equipped_quick_items:?}");
            });
        }

        let display_settings_open = display_settings_open();
        let wheel_changed = *PREV_WHEEL_TYPE.lock().expect("Couldn't get PREV_WHEEL_TYPE") != selected_wheel_type;
        let display_settings_changed = PREV_DISPLAY_SETTINGS_OPEN.load(Ordering::Relaxed) != display_settings_open;

        if wheel_changed || display_settings_changed {
            let is_ui_open = (selected_wheel_type != WheelType::None) || display_settings_open;
            menu_man.disable_mouse_cursor = !is_ui_open;
        }
        set_suppress_camera(controller_wheel.is_none() && selected_wheel_type != WheelType::None);
        *PREV_WHEEL_TYPE.lock().expect("Couldn't get PREV_WHEEL_TYPE") = selected_wheel_type;
        PREV_DISPLAY_SETTINGS_OPEN.store(display_settings_open, Ordering::Relaxed);
        ItemWheelData::mutate(|data| {
            data.spells = equipped_spells;
            data.quick_items = equipped_quick_items;
            data.wheel_type = selected_wheel_type;
            data.controller_wheel = controller_wheel;
            data.details = details;
        });

        commit_screen_debug();
        reset_scroll();
        TICK.fetch_add(1, Ordering::Relaxed);
    );
}
