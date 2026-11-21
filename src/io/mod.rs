use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::io::bindings::{Bindings, get_item_bindings, get_spell_bindings, set_item_bindings, set_spell_bindings};
use crate::keyboard::KEYS;
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::Settings;
use crate::{in_menus, keyboard};
use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CAPITAL, VK_TAB};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use lazy_static::lazy_static;
use std::sync::{LockResult, Mutex, MutexGuard};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_0;
use crate::keyboard::listener::{static_listener, Listener};

pub mod bindings;
pub(crate) mod controller;
pub(crate) mod controller_state;
pub mod hook;
mod hook_profiles;
pub(crate) mod selection_sound;

lazy_static!(
	static ref PREV_KEY_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
);

pub fn wheel_inputs(settings: &Settings) -> (WheelType, [u32; 2]) {
    if in_menus() || crate::settings::display_settings::display_settings_open() {
        return (WheelType::None, [get_spell_bindings().pad, get_item_bindings().pad]);
    }
	let pad = unsafe { FD4PadManager::instance().ok()
		.and_then(FD4PadManager::get_in_game_pad) };
	wheel_inputs_from_pad(pad, settings)
}

fn wheel_inputs_from_pad(pad: Option<&CSInGamePad>, settings: &Settings) -> (WheelType, [u32; 2]) {
    let mut controller_bindings = [0; 2];

	if let Some(pad) = pad.as_ref() {
		let bindings = Bindings::from_pad(pad);

		let mut switch_spell = bindings.get_input_bindings(pad, &UserInputKey::SwitchSpell);
		let mut switch_item = bindings.get_input_bindings(pad, &UserInputKey::SwitchItem);
		switch_spell.pad |= bindings.get_input_bindings(pad, &UserInputKey::SwitchSpell2).pad;
		switch_item.pad |= bindings.get_input_bindings(pad, &UserInputKey::SwitchItem2).pad;
		if static_listener("entries".to_string(), || Listener::Released { keys: Box::new([VK_0.0 as i32]) }).is_active() {
			let switch_spell = bindings.get_input_bindings_dbg(pad, &UserInputKey::SwitchSpell);
			let switch_item = bindings.get_input_bindings_dbg(pad, &UserInputKey::SwitchItem);
			tracing::info!("Switch spell keys: {switch_spell:?}");
			tracing::info!("Switch item keys: {switch_item:?}");
			let switch_spell2 = bindings.get_input_bindings_dbg(pad, &UserInputKey::SwitchSpell2);
			let switch_item2 = bindings.get_input_bindings_dbg(pad, &UserInputKey::SwitchItem2);
			tracing::info!("Switch spell keys2: {switch_spell2:?}");
			tracing::info!("Switch item keys2: {switch_item2:?}");
		}
		let spell_status = switch_spell.keyboard_mouse_status();
		let item_status = switch_item.keyboard_mouse_status();
		if is_debugging() {
			add_to_screen_debug(format!("Switch spell inputs: {switch_spell:?}"));
			add_to_screen_debug(format!("  Pressed status: {spell_status:?}"));
			add_to_screen_debug(format!("Switch item inputs: {switch_item:?}"));
			add_to_screen_debug(format!("  Pressed status: {item_status:?}"));
		}
		set_spell_bindings(switch_spell);
		set_item_bindings(switch_item);

        controller_bindings = [switch_spell.pad, switch_item.pad];
        if settings.spell_wheel_enabled && (spell_status.key() || spell_status.mouse())
            && settings.spells_key.eq_ignore_ascii_case("ingame")
        {
            return (WheelType::Spells, controller_bindings);
        }
        if settings.item_wheel_enabled && (item_status.key() || item_status.mouse())
            && settings.quick_items_key.eq_ignore_ascii_case("ingame")
        {
            return (WheelType::QuickItems, controller_bindings);
        }
    }

	let spells = (
		WheelType::Spells, settings.spell_wheel_enabled,
		PREV_KEY_SPELLS.lock(), settings.spells_key.clone(), VK_TAB,
	);
	let quick_items = (
		WheelType::QuickItems, settings.item_wheel_enabled,
		PREV_KEY_QUICK_ITEMS.lock(), settings.quick_items_key.clone(), VK_CAPITAL,
	);
	for (
		wheel_type, enabled,
		prev_key, key, raw_key_default,
	) in [spells, quick_items] {
		if !enabled {
			continue;
		}
		if let Some(value) = is_wheel_open_key_pressed(wheel_type, prev_key, key, &raw_key_default) {
			return (value, controller_bindings);
		}
	}

	(WheelType::None, controller_bindings)
}

fn is_wheel_open_key_pressed(wheel_type: WheelType, prev_key: LockResult<MutexGuard<Option<String>>>, key: String, raw_key_default: &VIRTUAL_KEY) -> Option<WheelType> {
	let mut prev_key = prev_key.expect("Couldn't acquire prev_key");
	let key_changed = matches!(&*prev_key, Some(prev_key) if prev_key != &key);
	let key_code = *KEYS.get(key.to_uppercase().as_str())
		.unwrap_or_else(|| {
			if key_changed {
				tracing::error!("Invalid key: {key}");
			}
			&raw_key_default.0
		}) as i32;
	*prev_key = Some(key);
	let out = keyboard::is_pressed(key_code);
	if out {
		return Some(wheel_type);
	}
	None
}

pub fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

