use eldenring::cs::{
    CSSessionManager, ChrIns, EquipGameData, EquipParamGoods, EquipParamWeapon, ItemCategory,
    ItemId, LobbyState, Magic, ProtocolState, SoloParamRepository, WorldChrMan,
};
use fromsoftware_shared::FromStatic;

use crate::io::controller_state::Slot;
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::{DetailWheels, Settings};
use crate::wheel_details::{
    FpCost, ItemAvailability, SlotDetails, modified_fp_cost, scaling_grade,
    summon_monument_available,
};

#[cfg(feature = "extra-memory-slots-support")]
use crate::expanded_memory_slots::MAX_SPELL_RECORDS;
#[cfg(not(feature = "extra-memory-slots-support"))]
const MAX_SPELL_RECORDS: usize = 14;
const QUICK_SLOTS: usize = 10;

pub struct SpellFp {
    rates: [f32; 2],
    current: Option<u32>,
}

impl SpellFp {
    pub fn read(player: &ChrIns) -> Option<Self> {
        let mut rates = [1.0; 2];
        // SpEffectParam separates sorcery and incantation consumption. Read active
        // effects, which also covers temporary modifiers and both catalyst hands.
        for (index, effect) in player.special_effect.entries().take(257).enumerate() {
            if index == 256 {
                return None;
            }
            let row = unsafe { effect.param_data?.as_ref() };
            rates[0] *= row.magic_consumption_rate();
            rates[1] *= row.miracle_consumption_rate();
        }
        Some(Self {
            rates,
            current: u32::try_from(player.modules.data.fp).ok(),
        })
    }

    fn spell(&self, goods_type: u8, base: i16) -> Option<FpCost> {
        let rate = match goods_type {
            5 | 17 => self.rates[0],  // Sorcery
            16 | 18 => self.rates[1], // Incantation
            _ => return None,
        };
        Some(FpCost {
            cost: modified_fp_cost(base, rate)?,
            current: self.current,
        })
    }
}

#[derive(Default)]
pub struct QuickItemState {
    availability: ItemAvailability,
    active_summon: Option<u32>,
}

pub fn read_item_availability(
    world: &WorldChrMan,
    player: &ChrIns,
    equipment: &EquipGameData,
    remedy_active: bool,
) -> QuickItemState {
    const SPIRIT_CALLING_BELL: u32 = 8158;
    let manager = &world.summon_buddy_manager;
    let multiplayer = unsafe { CSSessionManager::instance() }.is_ok_and(|session| {
        matches!(
            session.protocol_state,
            ProtocolState::Ingame | ProtocolState::WaitReentryToMap
        ) && (session.lobby_state == LobbyState::Client
            || session.players.iter().any(|entry| {
                !entry.is_local_player && !entry.join_wait && entry.base.steam_id != 0
            }))
    });
    let active_summon = if manager.player_has_alive_summon {
        u32::try_from(manager.active_summon_goods_id)
            .ok()
            .filter(|&id| id != 0)
    } else {
        None
    };
    // The current stone is set before summoning. The activation-range flag
    // stayed false in a usable area until the stone was assigned to a live spirit.
    let summon_blocked = !summon_monument_available(manager.buddy_stone_entity_id)
        || manager.player_has_alive_summon
        || manager.item_use_cooldown_timer > 0.0
        || remedy_active
        || multiplayer
        || {
            // The bell is a persistent key item. The quick-item index can miss it,
            // and a key item entry does not need to report a consumable quantity.
            let bell = ItemId::new(ItemCategory::Goods, SPIRIT_CALLING_BELL).unwrap();
            let mut keys = equipment
                .equip_inventory_data
                .items_data
                .key_entries()
                .iter()
                .filter_map(|entry| entry.as_option());
            keys.next().is_some_and(|first| {
                first.item_id != bell && !keys.any(|entry| entry.item_id == bell)
            })
        };
    QuickItemState {
        availability: ItemAvailability {
            current_fp: u32::try_from(player.modules.data.fp).ok(),
            current_hp: u32::try_from(player.modules.data.hp).ok(),
            summon_blocked,
        },
        active_summon,
    }
}

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

    pub fn quick_item_quantity(&self, slot: Slot) -> Option<u32> {
        match self.details(WheelType::QuickItems, slot)? {
            SlotDetails::QuickItem { quantity, .. } => quantity,
            _ => None,
        }
    }

    pub fn read_spells(
        &mut self,
        param_repo: &SoloParamRepository,
        equipped: &[(usize, u32)],
        effective_stats: [Option<u32>; 3],
        fp: Option<&SpellFp>,
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
                        fp: fp.and_then(|fp| {
                            fp.spell(
                                param_repo.get::<EquipParamGoods>(id)?.goods_type(),
                                row.mp(),
                            )
                        }),
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
        state: QuickItemState,
    ) {
        for &(index, id) in equipped {
            if let Some(entry) = self.quick_items.get_mut(index)
                && let Some(goods) = param_repo.get::<EquipParamGoods>(id)
            {
                let quantity = if goods.is_consume() && settings.wheel_details_item_quantity {
                    quick_item_quantity(equipment, index, id)
                } else {
                    None
                };
                let scaling = u32::try_from(goods.ref_virtual_wep_id())
                    .ok()
                    .and_then(|id| param_repo.get::<EquipParamWeapon>(id))
                    .map_or([None; 5], |weapon| {
                        [
                            weapon.correct_strength(),
                            weapon.correct_agility(),
                            weapon.correct_magic(),
                            weapon.correct_faith(),
                            weapon.correct_luck(),
                        ]
                        .map(scaling_grade)
                    });
                *entry = Some((
                    id,
                    SlotDetails::QuickItem {
                        quantity,
                        fp: positive_cost(goods.consume_mp()),
                        hp: positive_cost(goods.consume_hp()),
                        scaling,
                        availability: state.availability.for_goods(
                            goods.goods_type(),
                            id,
                            state.active_summon,
                        ),
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

fn positive_cost(value: i16) -> Option<u16> {
    (value > 0).then(|| value as u16)
}

fn quick_item_quantity(equipment: &EquipGameData, index: usize, id: u32) -> Option<u32> {
    let equipped_id = equipment
        .equipment_entries
        .quick_tems
        .get(index)?
        .as_valid()?;
    if equipped_id.category() != ItemCategory::Goods || equipped_id.param_id() != id {
        return None;
    }
    let quick_slot = equipment.equip_item_data.quick_slots.get(index)?;
    let inventory = &equipment.equip_item_data.inventory.items_data;
    if let Some(entry) = u32::try_from(quick_slot.index)
        .ok()
        .and_then(|index| inventory.entry_at_slot(index))
        .and_then(|entry| entry.as_option())
        .filter(|entry| entry.item_id == equipped_id)
    {
        return Some(entry.quantity);
    }
    // A quick slot may still hold its item ID after the final copy is consumed.
    // Check inventory by item ID before displaying zero for a stale index.
    match inventory.find_item_idx(equipped_id) {
        None => Some(0),
        Some(index) => inventory
            .entry_at_slot(index)?
            .as_option()
            .filter(|entry| entry.item_id == equipped_id)
            .map(|entry| entry.quantity),
    }
}
