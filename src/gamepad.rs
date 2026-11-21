use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD, XINPUT_GAMEPAD_TRIGGER_THRESHOLD};

pub type GamepadButtons = windows::Win32::UI::Input::XboxController::XINPUT_GAMEPAD_BUTTON_FLAGS;
pub type RightStick = [f32; 2];

pub const LEFT_TRIGGER: u32 = 1 << 16;
pub const RIGHT_TRIGGER: u32 = 1 << 17;

#[derive(Clone, Copy)]
pub struct GamepadSample {
    pub buttons: u32,
    pub right_stick: RightStick,
}

impl From<XINPUT_GAMEPAD> for GamepadSample {
    fn from(pad: XINPUT_GAMEPAD) -> Self {
        let mut buttons = u32::from(pad.wButtons.0);
        let trigger_threshold = XINPUT_GAMEPAD_TRIGGER_THRESHOLD.0 as u8;
        if pad.bLeftTrigger > trigger_threshold {
            buttons |= LEFT_TRIGGER;
        }
        if pad.bRightTrigger > trigger_threshold {
            buttons |= RIGHT_TRIGGER;
        }
        let max = i16::MAX as f32;
        Self {
            buttons,
            right_stick: [f32::from(pad.sThumbRX) / max, f32::from(pad.sThumbRY) / max],
        }
    }
}
