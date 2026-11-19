//! Strict, platform-independent settings binding schema. Invalid startup values use DEFAULT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    None,
    LeftShift,
    LeftCtrl,
    LeftAlt,
}

impl Modifier {
    pub const ALL: [Self; 4] = [Self::None, Self::LeftShift, Self::LeftCtrl, Self::LeftAlt];
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LeftShift => "left_shift",
            Self::LeftCtrl => "left_ctrl",
            Self::LeftAlt => "left_alt",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::LeftShift => "Left Shift",
            Self::LeftCtrl => "Left Ctrl",
            Self::LeftAlt => "Left Alt",
        }
    }
    pub fn bit(self) -> u8 {
        match self {
            Self::None => 0,
            Self::LeftShift => 1,
            Self::LeftCtrl => 4,
            Self::LeftAlt => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub function: u8,
    pub modifier: Modifier,
}

impl Binding {
    pub const DEFAULT: Self = Self {
        function: 10,
        modifier: Modifier::LeftShift,
    };
    pub fn parse(function: &str, modifier: &str) -> Result<Self, String> {
        let function = (1..=12)
            .find(|i| function == format!("F{i}"))
            .ok_or("settings_function_key must be F1 through F12")?;
        let modifier = Modifier::ALL
            .into_iter()
            .find(|m| m.name() == modifier)
            .ok_or("settings_modifier must be none, left_shift, left_ctrl, or left_alt")?;
        Ok(Self { function, modifier })
    }
    pub fn label(self) -> String {
        if self.modifier == Modifier::None {
            format!("F{}", self.function)
        } else {
            format!("{} + F{}", self.modifier.label(), self.function)
        }
    }
}

/// One input-task snapshot: F1..F12 bits, then left/right Shift, Ctrl, Alt bits.
#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub functions: u16,
    pub modifiers: u8,
}
impl Keys {
    pub fn function(self, b: Binding) -> bool {
        self.functions & (1 << (b.function - 1)) != 0
    }
    pub fn down(self, b: Binding) -> bool {
        self.function(b) && self.modifiers == b.modifier.bit()
    }
    pub fn released(self, b: Binding) -> bool {
        !self.function(b) && self.modifiers == 0
    }
}
