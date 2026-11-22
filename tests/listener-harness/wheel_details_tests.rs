use crate::Settings;
use crate::panel_model::{Draft, RuntimeSettings, Tab, parse};
use crate::settings::{DetailPosition, DetailWheels};
use crate::wheel_details::{CardFields, CardSpace, FpCost, SlotDetails};
use toml_edit::Value;

#[test]
fn legacy_details_defaults_and_interface_reset_preserve_other_values() {
    let legacy =
        "# My config\ncontroller_wheel_open_delay = 0.25\nname_tint = [0.5, 1.0, 1.0, 1.0]\n";
    let saved = parse(legacy).unwrap();
    assert_eq!(saved.wheel_details_wheels, DetailWheels::Both);
    assert_eq!(saved.wheel_details_position, DetailPosition::Below);
    assert!(saved.wheel_details_fp_cost);
    assert!(saved.wheel_details_stat_requirements);
    assert!(saved.wheel_details_item_quantity);
    assert!(saved.wheel_details_availability);
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
fn spells_distinguish_base_effective_zero_and_missing_costs() {
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
        text(Some(FpCost::Base(0))).unwrap(),
        ("0 Base FP".into(), String::new())
    );
    assert_eq!(
        text(Some(FpCost::Base(7))).unwrap(),
        ("7 Base FP".into(), String::new())
    );
    for (current, reason) in [(None, ""), (Some(7), ""), (Some(0), "Insufficient FP")] {
        assert_eq!(
            text(Some(FpCost::Effective { cost: 7, current })).unwrap(),
            ("7 FP".into(), reason.into())
        );
    }
}

#[test]
fn grouped_requirements_and_status_visibility_are_independent() {
    let details = SlotDetails::Spell {
        fp: Some(FpCost::Effective {
            cost: 7,
            current: Some(3),
        }),
        requirements: [10, 20, 15],
        effective_stats: [Some(10), Some(19), None],
    };
    let mut settings = Settings::default();
    let result =
        |settings: &Settings| CardFields::for_slot(details, settings).map(CardFields::text);
    assert_eq!(
        result(&settings).unwrap(),
        (
            "7 FP · Requires 10 INT, 20 FAI, 15 ARC".into(),
            "Insufficient FP · Unmet stat requirements (FAI)".into()
        )
    );
    settings.wheel_details_availability = false;
    assert_eq!(
        result(&settings).unwrap(),
        (
            "7 FP · Requires 10 INT, 20 FAI, 15 ARC".into(),
            String::new()
        )
    );
    settings.wheel_details_stat_requirements = false;
    settings.wheel_details_fp_cost = false;
    assert_eq!(result(&settings), None);
    settings.wheel_details_availability = true;
    assert_eq!(
        result(&settings).unwrap(),
        (
            String::new(),
            "Insufficient FP · Unmet stat requirements (FAI)".into()
        )
    );
}

#[test]
fn item_counts_refresh_and_missing_is_distinct_from_zero() {
    let mut settings = Settings::default();
    let text = |quantity, settings: &Settings| {
        CardFields::for_slot(SlotDetails::QuickItem { quantity }, settings).map(CardFields::text)
    };
    assert_eq!(
        text(Some(12), &settings).unwrap(),
        ("12 remaining".into(), String::new())
    );
    assert_eq!(
        text(Some(0), &settings).unwrap(),
        ("0 remaining".into(), "None remaining".into())
    );
    assert_eq!(text(None, &settings), None);
    settings.wheel_details_item_quantity = false;
    assert_eq!(text(Some(12), &settings), None);
    assert_eq!(
        text(Some(0), &settings).unwrap(),
        (String::new(), "None remaining".into())
    );
    settings.wheel_details_availability = false;
    assert_eq!(text(Some(0), &settings), None);
}

#[test]
fn position_uses_the_same_wheel_edge_for_different_content_heights() {
    let below = CardSpace::for_wheel(720.0, [120.0, 590.0], DetailPosition::Below);
    assert_eq!(below.top(36.0), below.top(64.0));
    let above = CardSpace::for_wheel(720.0, [120.0, 590.0], DetailPosition::Above);
    assert_eq!(
        above.top(36.0).unwrap() + 36.0,
        above.top(64.0).unwrap() + 64.0
    );
    let fallback = CardSpace::for_wheel(720.0, [120.0, 690.0], DetailPosition::Below);
    assert_eq!(fallback, above);
    assert_eq!(fallback.top(110.0), None);
}
