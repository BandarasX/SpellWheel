use crate::settings::{DetailPosition, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FpCost {
    Base(u16),
    // The current bindings supply base cost only.
    #[allow(dead_code)]
    Effective {
        cost: u16,
        current: Option<u32>,
    },
}

pub const MIN_CARD_FONT: f32 = 12.0;
pub const CARD_MARGIN: f32 = 5.0;
pub const CARD_CLEARANCE: f32 = 6.0;
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
            anchor_y: top - CARD_CLEARANCE,
            height: top - CARD_CLEARANCE - CARD_MARGIN,
            position: DetailPosition::Above,
        };
        let below = Self {
            anchor_y: bottom + CARD_CLEARANCE,
            height: viewport_height - bottom - CARD_CLEARANCE - CARD_MARGIN,
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

    pub fn top(self, height: f32) -> Option<f32> {
        if height > self.height {
            return None;
        }
        Some(match self.position {
            DetailPosition::Above => self.anchor_y - height,
            DetailPosition::Below => self.anchor_y,
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
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardFields {
    fp: Option<FpCostLabel>,
    requirements: [u8; 3],
    quantity: Option<u32>,
    unmet: [bool; 3],
    insufficient_fp: bool,
    none_remaining: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FpCostLabel {
    Base(u16),
    Effective(u16),
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
                    fields.fp = fp.map(|cost| match cost {
                        FpCost::Base(cost) => FpCostLabel::Base(cost),
                        FpCost::Effective { cost, .. } => FpCostLabel::Effective(cost),
                    });
                }
                if settings.wheel_details_stat_requirements {
                    fields.requirements = requirements;
                }
                if settings.wheel_details_availability {
                    fields.insufficient_fp = matches!(
                        fp,
                        Some(FpCost::Effective { cost, current: Some(current) })
                            if current < u32::from(cost)
                    );
                    fields.unmet = std::array::from_fn(|index| {
                        effective_stats[index]
                            .is_some_and(|value| value < u32::from(requirements[index]))
                    });
                }
            }
            SlotDetails::QuickItem { quantity } => {
                if settings.wheel_details_item_quantity {
                    fields.quantity = quantity;
                }
                fields.none_remaining = settings.wheel_details_availability && quantity == Some(0);
            }
        }
        (fields != Self::default()).then_some(fields)
    }

    pub fn text(self) -> (String, String) {
        use std::fmt::Write;

        const STATS: [&str; 3] = ["INT", "FAI", "ARC"];
        const SEPARATOR: &str = " · ";
        let mut main = match self.fp {
            Some(FpCostLabel::Base(cost)) => format!("{cost} Base FP"),
            Some(FpCostLabel::Effective(cost)) => format!("{cost} FP"),
            None => String::new(),
        };
        if let Some(quantity) = self.quantity {
            write!(main, "{quantity} remaining").unwrap();
        }
        let mut requirements = self
            .requirements
            .into_iter()
            .zip(STATS)
            .filter(|&(amount, _)| amount != 0);
        if let Some((amount, stat)) = requirements.next() {
            if !main.is_empty() {
                main.push_str(SEPARATOR);
            }
            write!(main, "Requires {amount} {stat}").unwrap();
            for (amount, stat) in requirements {
                write!(main, ", {amount} {stat}").unwrap();
            }
        }

        let mut status = if self.insufficient_fp {
            "Insufficient FP".to_owned()
        } else if self.none_remaining {
            "None remaining".to_owned()
        } else {
            String::new()
        };
        let mut unmet = STATS
            .into_iter()
            .zip(self.unmet)
            .filter_map(|(stat, unmet)| unmet.then_some(stat));
        if let Some(stat) = unmet.next() {
            if !status.is_empty() {
                status.push_str(SEPARATOR);
            }
            write!(status, "Unmet stat requirements ({stat}").unwrap();
            for stat in unmet {
                write!(status, ", {stat}").unwrap();
            }
            status.push(')');
        }
        (main, status)
    }
}
