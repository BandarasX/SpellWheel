use crate::settings::{DetailPosition, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FpCost {
    pub cost: u16,
    pub current: Option<u32>,
}

/// Apply the active category's multiplicative FP modifiers to the initial cast.
/// Keep an invalid effect snapshot distinct from a valid zero-cost cast.
pub fn modified_fp_cost(base: i16, rate: f32) -> Option<u16> {
    if base < 0 || !rate.is_finite() || rate < 0.0 {
        return None;
    }
    let cost = (f32::from(base) * rate).trunc();
    (cost <= f32::from(u16::MAX)).then(|| cost as u16)
}

pub fn scaling_grade(value: f32) -> Option<char> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    Some(match value {
        x if x >= 175.0 => 'S',
        x if x >= 140.0 => 'A',
        x if x >= 90.0 => 'B',
        x if x >= 60.0 => 'C',
        x if x >= 25.0 => 'D',
        _ => 'E',
    })
}

pub const MIN_CARD_FONT: f32 = 12.0;
pub const CARD_MARGIN: f32 = 5.0;
pub const MIN_CARD_CLEARANCE: f32 = 6.0;
const MAX_CARD_CLEARANCE: f32 = 24.0;
pub const CARD_PADDING_Y: f32 = 5.0;
pub const CARD_ROW_GAP: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardSpace {
    pub anchor_y: f32,
    pub height: f32,
    pub position: DetailPosition,
}

impl CardSpace {
    pub fn for_wheel(
        viewport_height: f32,
        [top, bottom]: [f32; 2],
        preferred: DetailPosition,
    ) -> Self {
        let above = Self {
            anchor_y: top - MIN_CARD_CLEARANCE,
            height: top - MIN_CARD_CLEARANCE - CARD_MARGIN,
            position: DetailPosition::Above,
        };
        let below = Self {
            anchor_y: bottom + MIN_CARD_CLEARANCE,
            height: viewport_height - bottom - MIN_CARD_CLEARANCE - CARD_MARGIN,
            position: DetailPosition::Below,
        };
        let (preferred, opposite) = match preferred {
            DetailPosition::Above => (above, below),
            DetailPosition::Below => (below, above),
        };
        // Pick a side from wheel geometry alone so highlighting a different slot cannot flip it.
        let reserve = 2.0 * CARD_PADDING_Y + 3.0 * MIN_CARD_FONT + 2.0 * CARD_ROW_GAP;
        if preferred.height < reserve && opposite.height > preferred.height {
            opposite
        } else {
            preferred
        }
    }

    pub fn top(self, height: f32, reserve: f32) -> Option<f32> {
        if height > self.height {
            return None;
        }
        let extra_gap =
            (self.height - height.max(reserve)).clamp(0.0, MAX_CARD_CLEARANCE - MIN_CARD_CLEARANCE);
        Some(match self.position {
            DetailPosition::Above => self.anchor_y - height - extra_gap,
            DetailPosition::Below => self.anchor_y + extra_gap,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotDetails {
    Spell {
        fp: Option<FpCost>,
        requirements: [u8; 3],
        effective_stats: [Option<u32>; 3],
    },
    QuickItem {
        quantity: Option<u32>,
        fp: Option<u16>,
        hp: Option<u16>,
        scaling: [Option<char>; 5],
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementState {
    Unknown,
    Met,
    Unmet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub amount: u8,
    pub state: RequirementState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardFields {
    fp: Option<u16>,
    hp: Option<u16>,
    requirements: [Option<Requirement>; 3],
    quantity: Option<u32>,
    scaling: [Option<char>; 5],
    insufficient_fp: bool,
    none_remaining: bool,
}

impl CardFields {
    pub fn for_slot(details: SlotDetails, settings: &Settings) -> Option<Self> {
        let mut fields = Self::default();
        match details {
            SlotDetails::Spell {
                fp,
                requirements,
                effective_stats,
            } => {
                if settings.wheel_details_fp_cost {
                    fields.fp = fp.map(|fp| fp.cost);
                }
                if settings.wheel_details_stat_requirements {
                    fields.requirements = std::array::from_fn(|index| {
                        let amount = requirements[index];
                        (amount != 0).then(|| Requirement {
                            amount,
                            state: match effective_stats[index] {
                                Some(current) if current >= u32::from(amount) => {
                                    RequirementState::Met
                                }
                                Some(_) => RequirementState::Unmet,
                                None => RequirementState::Unknown,
                            },
                        })
                    });
                }
                fields.insufficient_fp = settings.wheel_details_availability
                    && matches!(fp, Some(FpCost { cost, current: Some(current) }) if current < u32::from(cost));
            }
            SlotDetails::QuickItem {
                quantity,
                fp,
                hp,
                scaling,
            } => {
                if settings.wheel_details_item_quantity {
                    fields.quantity = quantity;
                }
                fields.fp = fp;
                fields.hp = hp;
                fields.scaling = scaling;
                fields.none_remaining = settings.wheel_details_availability && quantity == Some(0);
            }
        }
        (fields != Self::default()).then_some(fields)
    }

    pub fn requirements(self) -> impl Iterator<Item = (&'static str, Requirement)> {
        ["INT", "FAI", "ARC"]
            .into_iter()
            .zip(self.requirements)
            .filter_map(|(stat, requirement)| Some((stat, requirement?)))
    }

    pub fn text(self) -> (String, &'static str) {
        let mut main = Vec::new();
        if let Some(fp) = self.fp {
            main.push(format!("{fp} FP"));
        }
        if let Some(hp) = self.hp {
            main.push(format!("{hp} HP"));
        }
        if let Some(quantity) = self.quantity {
            main.push(format!("{quantity} remaining"));
        }
        let scaling: Vec<_> = ["STR", "DEX", "INT", "FAI", "ARC"]
            .into_iter()
            .zip(self.scaling)
            .filter_map(|(stat, grade)| Some(format!("{} {stat}", grade?)))
            .collect();
        if !scaling.is_empty() {
            main.push(format!("Scaling {}", scaling.join(", ")));
        }

        let status = if self.insufficient_fp {
            "Insufficient FP"
        } else if self.none_remaining {
            "None remaining"
        } else {
            ""
        };
        (main.join(" · "), status)
    }
}
