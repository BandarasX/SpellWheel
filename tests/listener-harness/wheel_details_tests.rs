use crate::Settings;
use crate::panel_model::{Draft, RuntimeSettings, Tab, parse};
use crate::settings::{DetailPosition, DetailWheels};
use crate::wheel_details::{CardFields, CardSpace, FpCost, ItemAvailability, SlotDetails};
use toml_edit::Value;

#[test]
fn legacy_details_defaults_and_interface_reset_preserve_other_values() {
    let legacy = "# My config\ncontroller_wheel_open_delay = 0.25\nname_tint = [0.5, 1.0, 1.0, 1.0]\nitem_names = 'hide'\n";
    let saved = parse(legacy).unwrap();
    assert_eq!(saved.wheel_details_wheels, DetailWheels::Both);
    assert_eq!(saved.wheel_details_position, DetailPosition::Below);
    assert!(saved.wheel_details_fp_cost);
    assert!(saved.wheel_details_stat_requirements);
    assert!(saved.wheel_details_item_quantity);
    assert!(saved.wheel_details_availability);
    assert!(saved.wheel_details_show_name);
    let mut draft = Draft::new(Some(legacy.into()), saved.clone());
    assert_eq!(draft.text, legacy);

    draft
        .set("wheel_details_fp_cost", Value::from(false))
        .unwrap();
    draft
        .set("wheel_details_wheels", Value::from("off"))
        .unwrap();
    draft
        .set("wheel_details_position", Value::from("above"))
        .unwrap();
    draft
        .set("wheel_details_show_name", Value::from(false))
        .unwrap();
    assert!(draft.switch_mode());
    assert!(draft.switch_mode());
    let mut runtime = RuntimeSettings::new(saved.clone());
    runtime.preview(&draft.last_valid);
    assert_eq!(runtime.effective.wheel_details_wheels, DetailWheels::Off);
    assert_eq!(
        runtime.effective.wheel_details_position,
        DetailPosition::Above
    );
    assert!(!runtime.effective.wheel_details_fp_cost);
    assert!(runtime.effective.wheel_details_item_quantity);
    assert!(!runtime.effective.wheel_details_show_name);
    assert_eq!(runtime.effective.item_names, "hide");
    assert_eq!(runtime.saved, saved);
    runtime.commit(&draft.last_valid);
    assert_eq!(runtime.saved, parse(&draft.text).unwrap());
    draft.reset(Some(Tab::Interface)).unwrap();
    assert_eq!(draft.last_valid, saved);
    assert!(draft.text.contains("# My config"));
}

#[test]
fn typed_card_settings_accept_each_choice_and_reject_unknown_values() {
    for (key, allowed) in [
        (
            "wheel_details_wheels",
            &["both", "spell", "item", "off"][..],
        ),
        ("wheel_details_position", &["below", "above"][..]),
    ] {
        for option in allowed {
            assert!(parse(&format!("{key} = '{option}'")).is_ok());
        }
        assert!(parse(&format!("{key} = 'invalid'")).is_err());
    }
    assert!(parse("wheel_details_item_quantity = 'yes'").is_err());
}

#[test]
fn spell_fp_uses_live_cost_and_warns_only_below_it() {
    use crate::wheel_details::modified_fp_cost;
    assert_eq!(modified_fp_cost(13, 1.0), Some(13));
    // Exact costs stay whole. Fractional costs round up even below the half point.
    assert_eq!(modified_fp_cost(32, 0.75), Some(24));
    assert_eq!(modified_fp_cost(7, 0.75), Some(6));
    assert_eq!(modified_fp_cost(13, 0.75), Some(10));
    assert_eq!(modified_fp_cost(13, 0.75 * 0.85), Some(9));
    assert_eq!(modified_fp_cost(13, 0.0), Some(0));
    assert_eq!(modified_fp_cost(-1, 1.0), None);
    assert_eq!(modified_fp_cost(13, f32::NAN), None);

    let settings = Settings::default();
    let text = |fp| {
        CardFields::for_slot(
            SlotDetails::Spell {
                fp,
                requirements: [0; 3],
                effective_stats: [None; 3],
            },
            &settings,
        )
        .map(CardFields::text)
    };
    assert_eq!(text(None), None);
    assert_eq!(
        text(Some(FpCost {
            cost: 0,
            current: Some(0)
        })),
        Some(("0 FP".into(), String::new()))
    );
    let cost = modified_fp_cost(13, 0.75).unwrap();
    for (current, message) in [(None, ""), (Some(10), ""), (Some(9), "Insufficient FP")] {
        assert_eq!(
            text(Some(FpCost { cost, current })),
            Some(("10 FP".into(), message.into()))
        );
    }
}

#[test]
fn each_requirement_has_its_own_state_without_a_stat_message() {
    use crate::wheel_details::{Requirement, RequirementState::*};
    let details = SlotDetails::Spell {
        fp: Some(FpCost {
            cost: 7,
            current: Some(3),
        }),
        requirements: [10, 20, 15],
        effective_stats: [Some(10), Some(19), None],
    };
    let mut settings = Settings::default();
    let result = |settings: &Settings| CardFields::for_slot(details, settings).unwrap();
    assert_eq!(
        result(&settings).text(),
        ("7 FP".into(), "Insufficient FP".into())
    );
    settings.wheel_details_availability = false;
    assert_eq!(result(&settings).text(), ("7 FP".into(), String::new()));
    assert_eq!(
        result(&settings).requirements().collect::<Vec<_>>(),
        [
            (
                "INT",
                Requirement {
                    amount: 10,
                    state: Met
                }
            ),
            (
                "FAI",
                Requirement {
                    amount: 20,
                    state: Unmet
                }
            ),
            (
                "ARC",
                Requirement {
                    amount: 15,
                    state: Unknown
                }
            ),
        ]
    );
    settings.wheel_details_stat_requirements = false;
    assert_eq!(result(&settings).requirements().count(), 0);
}

#[test]
fn item_cost_and_scaling_do_not_depend_on_the_icon_count() {
    use crate::wheel_details::scaling_grade;
    let mut settings = Settings::default();
    let no_scaling = [None; 5];
    let magic_pot = [0.0, 0.0, 95.0, 0.0, 0.0].map(scaling_grade);
    let text = |quantity, fp, hp, scaling, settings: &Settings| {
        CardFields::for_slot(
            SlotDetails::QuickItem {
                quantity,
                fp,
                hp,
                scaling,
                availability: ItemAvailability::default(),
            },
            settings,
        )
        .map(CardFields::text)
    };
    for quantity in [None, Some(0), Some(12)] {
        assert_eq!(text(quantity, None, None, no_scaling, &settings), None);
        assert_eq!(
            text(quantity, Some(3), None, magic_pot, &settings),
            Some(("3 FP · Scaling B INT".into(), String::new()))
        );
    }
    settings.wheel_details_item_quantity = false;
    assert_eq!(
        text(Some(0), Some(3), None, magic_pot, &settings),
        Some(("3 FP · Scaling B INT".into(), String::new()))
    );
    assert_eq!(
        [1.0, 25.0, 60.0, 90.0, 140.0, 175.0].map(scaling_grade),
        [
            Some('E'),
            Some('D'),
            Some('C'),
            Some('B'),
            Some('A'),
            Some('S')
        ]
    );
}

#[test]
fn item_warnings_combine_and_respect_live_thresholds_and_settings() {
    let mut settings = Settings::default();
    let status = |current_fp, current_hp, summon_blocked, settings: &Settings| {
        CardFields::for_slot(
            SlotDetails::QuickItem {
                quantity: None,
                fp: Some(9),
                hp: Some(30),
                scaling: [None; 5],
                availability: ItemAvailability {
                    current_fp,
                    current_hp,
                    summon_blocked,
                },
            },
            settings,
        )
        .unwrap()
        .text()
        .1
    };
    assert_eq!(
        status(Some(8), Some(29), true, &settings),
        "Insufficient FP · Insufficient HP · Cannot summon"
    );
    assert_eq!(
        status(Some(8), Some(30), false, &settings),
        "Insufficient FP"
    );
    assert_eq!(
        status(Some(9), Some(29), false, &settings),
        "Insufficient HP"
    );
    assert_eq!(status(Some(9), Some(30), true, &settings), "Cannot summon");
    assert_eq!(status(Some(9), Some(30), false, &settings), "");
    assert_eq!(status(None, None, false, &settings), "");
    settings.wheel_details_availability = false;
    assert_eq!(status(Some(8), Some(29), true, &settings), "");
}

#[test]
fn current_monument_clears_the_warning_before_the_first_summon() {
    let settings = Settings::default();
    for (monument_id, expected) in [
        (0, "Cannot summon"),
        (10000104, ""),
        (u32::MAX, "Cannot summon"),
    ] {
        let availability = ItemAvailability {
            current_fp: Some(100),
            summon_blocked: !crate::wheel_details::summon_monument_available(monument_id),
            ..Default::default()
        };
        let fields = CardFields::for_slot(
            SlotDetails::QuickItem {
                quantity: None,
                fp: Some(40),
                hp: None,
                scaling: [None; 5],
                availability: availability.for_goods(7, 221002, None),
            },
            &settings,
        )
        .unwrap();
        assert_eq!(fields.text().1, expected);
    }
}

#[test]
fn active_ash_or_puppet_suppresses_only_its_own_summon_warning() {
    let settings = Settings::default();
    let ash = 203010;
    let puppet = 206007;
    let state = ItemAvailability {
        current_fp: Some(5),
        current_hp: Some(12),
        summon_blocked: false,
    };
    let status = |availability| {
        CardFields::for_slot(
            SlotDetails::QuickItem {
                quantity: None,
                fp: Some(9),
                hp: Some(30),
                scaling: [None; 5],
                availability,
            },
            &settings,
        )
        .unwrap()
        .text()
        .1
    };
    let resources = "Insufficient FP · Insufficient HP";
    assert_eq!(status(state.for_goods(7, ash, None)), resources);
    let blocked = ItemAvailability {
        summon_blocked: true,
        ..state
    };
    for (kind, id, other) in [(7, ash, puppet), (8, puppet, ash)] {
        assert_eq!(status(blocked.for_goods(kind, id, Some(id))), resources);
        assert_eq!(
            status(blocked.for_goods(kind, id, Some(other))),
            "Insufficient FP · Insufficient HP · Cannot summon"
        );
    }
    assert_eq!(status(blocked.for_goods(0, ash, None)), resources);
}

#[test]
fn quantity_font_scale_validates_and_follows_preview_save_and_reset() {
    let settings = Settings::default();
    assert_eq!(settings.wheel_details_quantity_font_scale, 1.0);
    for valid in [0.25, 1.275, 3.0] {
        assert_eq!(
            parse(&format!("wheel_details_quantity_font_scale = {valid:?}"))
                .unwrap()
                .wheel_details_quantity_font_scale,
            valid
        );
    }
    for invalid in ["0.0", "3.01", "nan", "'large'"] {
        assert!(parse(&format!("wheel_details_quantity_font_scale = {invalid}")).is_err());
    }
    let mut draft = Draft::new(Some(String::new()), settings.clone());
    draft
        .set("wheel_details_quantity_font_scale", Value::from(1.275))
        .unwrap();
    let mut runtime = RuntimeSettings::new(settings);
    runtime.preview(&draft.last_valid);
    assert_eq!(runtime.effective.wheel_details_quantity_font_scale, 1.275);
    assert_eq!(runtime.saved.wheel_details_quantity_font_scale, 1.0);
    runtime.commit(&draft.last_valid);
    assert_eq!(runtime.saved.wheel_details_quantity_font_scale, 1.275);
    draft.reset(Some(Tab::Interface)).unwrap();
    assert_eq!(draft.last_valid.wheel_details_quantity_font_scale, 1.0);
}

#[test]
fn gap_compacts_before_card_height_on_both_sides() {
    for position in [DetailPosition::Above, DetailPosition::Below] {
        let card = 60.0;
        for (gap, expected) in [(110.0, 24.0), (82.0, 17.0), (69.0, 4.0)] {
            let bounds = match position {
                DetailPosition::Above => [gap, 690.0],
                DetailPosition::Below => [29.0, 720.0 - gap],
            };
            let space = CardSpace::for_wheel(720.0, bounds, position);
            let top = space.top(card, card).unwrap();
            let distance = match position {
                DetailPosition::Above => bounds[0] - top - card,
                DetailPosition::Below => top - bounds[1],
            };
            assert_eq!(distance, expected);
        }
    }
}

#[test]
fn card_transparency_validates_and_previews_independently_of_border() {
    let defaults = Settings::default();
    assert_eq!(defaults.wheel_details_background_transparency, 0.06);
    assert!(defaults.wheel_details_gold_border);
    assert!(parse("wheel_details_background_transparency = 1.01").is_err());
    let mut draft = Draft::new(Some(String::new()), defaults.clone());
    draft
        .set("wheel_details_background_transparency", Value::from(0.3175))
        .unwrap();
    draft
        .set("wheel_details_gold_border", Value::from(false))
        .unwrap();
    let mut runtime = RuntimeSettings::new(defaults.clone());
    runtime.preview(&draft.last_valid);
    assert_eq!(
        runtime.effective.wheel_details_background_transparency,
        0.3175
    );
    assert!(!runtime.effective.wheel_details_gold_border);
    draft.reset(Some(Tab::Interface)).unwrap();
    assert_eq!(draft.last_valid, defaults);
}
