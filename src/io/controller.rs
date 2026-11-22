use std::sync::Mutex;
use std::time::{Duration, Instant};

use eldenring::cs::{CSFeManHudState, CSFeManImp, WorldChrMan};
use fromsoftware_shared::FromStatic;

use crate::gamepad::{GamepadSample, RightStick};
use crate::io::controller_state::{Context, ControllerState, Slot, VisibleWheel, Wheel};
use crate::settings::Settings;
use crate::settings::display_settings::display_settings_open;

struct Controller {
    state: ControllerState,
    sample: Option<(GamepadSample, Instant)>,
    delay: Duration,
    recent_buttons: u32,
    context: Context,
    last_zoom: Option<(bool, bool)>,
}

impl Controller {
    fn observe(&mut self) {
        self.state.observe_buttons(
            self.sample.map(|(sample, at)| (sample.buttons, at)),
            self.delay,
            self.context,
        );
    }

    fn refresh(&mut self, settings: &Settings) {
        let zoom = if display_settings_open() || !super::is_game_focused() {
            None
        } else {
            zoom_state()
        };
        if zoom != self.last_zoom && settings.debugging {
            tracing::info!(?zoom, "Controller camera state (precision aim, telescope)");
        }
        self.last_zoom = zoom;
        self.context = match zoom {
            Some((true, _)) | Some((_, true)) => Context::Zoom,
            Some((false, false))
                if !crate::in_menus()
                    && unsafe { CSFeManImp::instance() }
                        .is_ok_and(|fe| matches!(fe.hud_state, CSFeManHudState::Default)) =>
            {
                Context::Gameplay
            }
            _ => Context::Interrupted,
        };
        self.observe();
    }
}

static CONTROLLER: Mutex<Controller> = Mutex::new(Controller {
    state: ControllerState::new(),
    sample: None,
    delay: Duration::ZERO,
    recent_buttons: 0,
    context: Context::Interrupted,
    last_zoom: None,
});

fn zoom_state() -> Option<(bool, bool)> {
    // See ChrInsFlags1c5 in eldenring-rs and the Telescope stateInfo in the game param definitions.
    const TELESCOPE_STATE_INFO: u16 = 15;
    let world = unsafe { WorldChrMan::instance() }.ok()?;
    let player = world.main_player.as_ref()?;
    let chr = &player.chr_ins;
    let telescope = chr.special_effect.entries().any(|entry| {
        entry
            .param_data
            .is_some_and(|param| unsafe { param.as_ref() }.state_info() == TELESCOPE_STATE_INFO)
    });
    Some((chr.chr_flags1c5.precision_shooting(), telescope))
}

pub fn record_sample(sample: Option<GamepadSample>) -> bool {
    let mut input = CONTROLLER.lock().unwrap();
    if display_settings_open() {
        input.context = Context::Interrupted;
    }
    input.recent_buttons = sample.map_or(0, |sample| {
        sample.buttons | input.sample.map_or(0, |(previous, _)| previous.buttons)
    });
    input.sample = sample.map(|sample| (sample, Instant::now()));
    input.observe();
    input.state.visible().is_some()
}

pub fn interrupt() {
    let mut input = CONTROLLER.lock().unwrap();
    input.context = Context::Interrupted;
    input.observe();
}

pub fn take_taps(settings: &Settings) -> [bool; 2] {
    let mut input = CONTROLLER.lock().unwrap();
    input.refresh(settings);
    let enabled = [settings.spell_wheel_enabled, settings.item_wheel_enabled];
    Wheel::ALL.map(|wheel| input.state.take_tap(wheel) && enabled[wheel.index()])
}

pub fn can_replay_tap() -> bool {
    let input = CONTROLLER.lock().unwrap();
    input.context == Context::Gameplay && input.sample.is_some()
}

pub fn native_passthrough(wheel: Wheel, binding: u32, detours_ready: bool) -> bool {
    let input = CONTROLLER.lock().unwrap();
    input.recent_buttons & binding != 0
        && (input.context == Context::Zoom || input.state.zoom_owns(wheel) || !detours_ready)
}

pub fn right_stick(wheel: VisibleWheel) -> Option<RightStick> {
    let input = CONTROLLER.lock().unwrap();
    if input.state.visible() != Some(wheel) {
        return None;
    }
    input.sample.map(|(sample, _)| sample.right_stick)
}

pub fn highlight(visible: VisibleWheel, slot: Option<Slot>) -> Option<Slot> {
    let mut input = CONTROLLER.lock().unwrap();
    if let Some(slot) = slot {
        input.state.highlight(visible, slot);
    }
    input.state.visible_slot(visible)
}

pub fn update_wheel(
    settings: &Settings,
    resolved_bindings: [u32; 2],
    keyboard_wheel_open: bool,
    mut slot_access: impl FnMut(Wheel, Option<Slot>) -> Option<Slot>,
) -> (Option<VisibleWheel>, Option<Wheel>) {
    let mut input = CONTROLLER.lock().unwrap();
    input.delay =
        Duration::try_from_secs_f32(settings.controller_wheel_open_delay).unwrap_or(Duration::MAX);
    let enabled = [settings.spell_wheel_enabled, settings.item_wheel_enabled];
    input.state.set_bindings(Wheel::ALL.map(|wheel| {
        let index = wheel.index();
        if enabled[index] && super::hook::controller_hooks_ready(wheel) {
            resolved_bindings[index]
        } else {
            0
        }
    }));
    input.refresh(settings);
    let sound = input
        .state
        .update_selection(settings.switch_instantly, &mut slot_access);
    let mut visible = input.state.visible();
    if visible.is_none() && !keyboard_wheel_open {
        visible = Wheel::ALL
            .into_iter()
            .find_map(|wheel| input.state.open(wheel, || slot_access(wheel, None)));
    }
    (visible, sound)
}
