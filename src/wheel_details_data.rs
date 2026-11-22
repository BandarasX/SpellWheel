use eldenring::cs::{EquipGameData, EquipParamGoods, ItemCategory, Magic, SoloParamRepository};

use crate::io::controller_state::Slot;
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::{DetailWheels, Settings};
use crate::wheel_details::{FpCost, SlotDetails};

#[cfg(feature = "extra-memory-slots-support")]
use crate::expanded_memory_slots::MAX_SPELL_RECORDS;
#[cfg(not(feature = "extra-memory-slots-support"))]
const MAX_SPELL_RECORDS: usize = 14;
const QUICK_SLOTS: usize = 10;

#[derive(Clone, Copy, Default)]
pub struct WheelDetailsData {
    spells: [Option<(u32, SlotDetails)>; MAX_SPELL_RECORDS],
    quick_items: [Option<(u32, SlotDetails)>; QUICK_SLOTS],
    pub player_context: Option<usize>,
    pub active_spell: Option<Slot>,
    pub active_item: Option<Slot>,
}

impl WheelDetailsData {
    pub fn details(&self, wheel: WheelType, slot: Slot) -> Option<SlotDetails> {
        let entries: &[_] = match wheel {
            WheelType::Spells => &self.spells,
            WheelType::QuickItems => &self.quick_items,
            WheelType::None => return None,
        };
        let (id, details) = (*entries.get(usize::try_from(slot.index).ok()?)?)?;
        (id == slot.param_id).then_some(details)
    }

    pub fn active(&self, wheel: WheelType) -> Option<Slot> {
        match wheel {
            WheelType::Spells => self.active_spell,
            WheelType::QuickItems => self.active_item,
            WheelType::None => None,
        }
    }

    pub fn read_spells(
        &mut self,
        param_repo: &SoloParamRepository,
        equipped: &[(usize, u32)],
        effective_stats: [Option<u32>; 3],
        settings: &Settings,
    ) {
        if !settings.wheel_details_fp_cost
            && !settings.wheel_details_stat_requirements
            && !settings.wheel_details_availability
        {
            return;
        }
        for &(index, id) in equipped {
            if let Some(entry) = self.spells.get_mut(index)
                && let Some(row) = param_repo.get::<Magic>(id)
            {
                *entry = Some((
                    id,
                    SlotDetails::Spell {
                        fp: u16::try_from(row.mp()).ok().map(FpCost::Base),
                        requirements: [
                            row.requirement_intellect(),
                            row.requirement_faith(),
                            row.requirement_luck(),
                        ],
                        effective_stats,
                    },
                ));
            }
        }
    }

    pub fn read_quick_items(
        &mut self,
        param_repo: &SoloParamRepository,
        equipment: &EquipGameData,
        equipped: &[(usize, u32)],
        settings: &Settings,
    ) {
        if !settings.wheel_details_item_quantity && !settings.wheel_details_availability {
            return;
        }
        for &(index, id) in equipped {
            if let Some(entry) = self.quick_items.get_mut(index)
                && let Some(quantity) = quick_item_quantity(param_repo, equipment, index, id)
            {
                *entry = Some((
                    id,
                    SlotDetails::QuickItem {
                        quantity: Some(quantity),
                    },
                ));
            }
        }
    }
}

pub fn details_enabled(settings: &Settings, wheel: WheelType) -> bool {
    matches!(
        (settings.wheel_details_wheels, wheel),
        (DetailWheels::Both | DetailWheels::Spell, WheelType::Spells)
            | (
                DetailWheels::Both | DetailWheels::Item,
                WheelType::QuickItems
            )
    )
}

fn quick_item_quantity(
    param_repo: &SoloParamRepository,
    equipment: &EquipGameData,
    index: usize,
    id: u32,
) -> Option<u32> {
    let equipped_id = equipment
        .equipment_entries
        .quick_tems
        .get(index)?
        .as_valid()?;
    if equipped_id.category() != ItemCategory::Goods || equipped_id.param_id() != id {
        return None;
    }
    if !param_repo.get::<EquipParamGoods>(id)?.is_consume() {
        return None;
    }
    let quick_slot = equipment.equip_item_data.quick_slots.get(index)?;
    let inventory = &equipment.equip_item_data.inventory.items_data;
    let entry = inventory
        .entry_at_slot(u32::try_from(quick_slot.index).ok()?)?
        .as_option()?;
    (entry.item_id == equipped_id).then_some(entry.quantity)
}
