use crate::{Settings, panel_model::*};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use toml_edit::Value;

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "spellwheel-panel-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("spellwheel.toml")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const CUSTOM: &str = "# User notes: café 日本語\ncontroller_wheel_open_delay = 0.25 # keep delay\nmods = ['C:\\mods\\QuestPath']\nicon_scale_multiplier = 0.15 # icon comment\nhdr_enabled = false\nhdr_ui_brightness = 200.0\nunknown_key = 'retained'\n\n[future]\nvalue = 42 # keep nested\n";
fn draft() -> Draft {
    Draft::new(Some(CUSTOM.into()), Settings::default())
}

#[test]
fn shared_roundtrip_preserves_comments_unknown_tables_order_and_unmanaged_values() {
    let mut d = draft();
    d.set("icon_scale_multiplier", Value::from(0.2)).unwrap();
    let staged = d.text.clone();
    for tab in Tab::ALL {
        d.tab = tab;
        assert!(d.switch_mode());
        assert!(d.switch_mode());
        assert_eq!(d.text, staged);
    }
    assert!(d.text.contains("0.2 # icon comment"));
    assert!(d.text.contains("# User notes: café 日本語"));
    assert!(d.text.contains("value = 42 # keep nested"));
    let s = parse(&d.text).unwrap();
    assert_eq!(s.controller_wheel_open_delay, 0.25);
    assert_eq!(s.mods, ["C:\\mods\\QuestPath"]);
    assert!(
        !d.text.contains("settings_modifier"),
        "mode switching must not materialize untouched omitted keys"
    );
    assert!(d.text.find("controller_wheel").unwrap() < d.text.find("unknown_key").unwrap());
}

#[test]
fn invalid_advanced_text_never_loses_text_or_last_valid_preview() {
    let mut d = draft();
    d.mode = Mode::Advanced;
    let old = d.last_valid.clone();
    for invalid in [
        "style = [",
        "style = 12",
        "hdr_ui_brightness = nan",
        "settings_function_key = 'F24'",
        "icon_tint = [1, 2, 3]",
    ] {
        d.text = invalid.into();
        assert!(!d.switch_mode());
        assert_eq!(d.mode, Mode::Advanced);
        assert_eq!(d.text, invalid);
        assert_eq!(d.last_valid, old);
        assert!(d.error.is_some());
    }
    d.discard();
    assert_eq!(d.text, CUSTOM);
    assert!(d.error.is_none());
}

#[test]
fn project_defaults_apply_only_to_new_files_or_explicit_scoped_reset() {
    let empty = Draft::new(Some(String::new()), Settings::default());
    assert!(!empty.last_valid.hdr_enabled);
    assert_eq!(empty.last_valid.hdr_ui_brightness, 200.0);
    assert_eq!(empty.text, "");
    assert!(!empty.dirty());
    let new = Draft::new(None, Settings::default());
    assert!(new.last_valid.hdr_enabled);
    assert_eq!(new.last_valid.hdr_ui_brightness, 400.0);
    assert!(
        !new.dirty(),
        "opening doesn't write or mark unchanged new draft dirty"
    );
    let mut d = draft();
    d.set("style", Value::from("simple")).unwrap();
    d.reset(Some(Tab::Hdr)).unwrap();
    assert_eq!(d.last_valid.style, "simple");
    assert!(d.last_valid.hdr_enabled);
    assert_eq!(d.last_valid.hdr_ui_brightness, 400.0);
    assert_eq!(d.last_valid.controller_wheel_open_delay, 0.25);
    d.reset(Some(Tab::Interface)).unwrap();
    assert_eq!(
        binding(&d.last_valid).unwrap(),
        crate::settings_binding::Binding::DEFAULT
    );
    d.reset(None).unwrap();
    let s = &d.last_valid;
    assert_eq!(s.style, "pretty");
    assert_eq!(s.highlight_expansion, 0.15);
    assert_eq!(s.icon_scale_multiplier, 0.15);
    assert_eq!(s.radius_multiplier, 0.3);
    assert_eq!(s.highlight_time, 0.15);
    assert_eq!(s.item_names, "show");
    assert_eq!(s.font_scale_multiplier, 1.0);
    assert!(s.text_shadows);
    assert_eq!(s.icon_tint, [1.0; 4]);
    assert_eq!(s.icon_background_tint, [1.0; 4]);
    assert_eq!(s.name_tint, [1.0; 4]);
    assert_eq!(s.hdr_ui_saturation, 1.0);
    assert!(d.dirty());
    assert!(d.text.contains("# keep delay"));
    assert!(d.text.contains("unknown_key"));
    d.discard();
    assert_eq!(d.text, CUSTOM);
    assert!(!d.last_valid.hdr_enabled);
}

#[test]
fn invalid_reset_is_blocked_until_explicit_discard_or_repair() {
    let mut d = draft();
    d.text = "mods = [".into();
    let invalid = d.text.clone();
    assert!(d.reset(None).is_err());
    assert_eq!(d.text, invalid);
    d.discard();
    d.reset(None).unwrap();
    assert_eq!(d.last_valid.controller_wheel_open_delay, 0.25);
}

#[test]
fn save_success_discard_and_clean_dirty_close_transitions() {
    let temp = Temp::new();
    let path = temp.file();
    fs::write(&path, CUSTOM).unwrap();
    let mut d = draft();
    assert!(d.request_close());
    d.set("hdr_ui_brightness", Value::from(500.0)).unwrap();
    assert!(!d.request_close());
    assert!(d.close_pending);
    let saved = d.save(&path, false).unwrap();
    assert_eq!(saved.hdr_ui_brightness, 500.0);
    assert!(!d.dirty());
    assert_eq!(fs::read_to_string(&path).unwrap(), d.text);
    d.reset(None).unwrap();
    d.discard();
    assert_eq!(d.last_valid.hdr_ui_brightness, 500.0);
    assert!(d.request_close());
    assert_eq!(
        fs::read_dir(&temp.0).unwrap().count(),
        1,
        "temporary files cleaned"
    );
}

#[test]
fn failed_write_keeps_original_draft_saved_and_binding() {
    let temp = Temp::new();
    let path = temp.file();
    fs::write(&path, CUSTOM).unwrap();
    let mut d = draft();
    d.set("settings_function_key", Value::from("F4")).unwrap();
    let saved = d.saved.clone();
    let staged = d.text.clone();
    // Missing parent deterministically fails on Windows and as root on Linux.
    assert!(d.save(&temp.0.join("missing/settings.toml"), true).is_err());
    assert_eq!(d.saved, saved);
    assert_eq!(d.text, staged);
    assert!(d.dirty());
    assert_eq!(fs::read_to_string(&path).unwrap(), CUSTOM);
    // Failure after temporary creation: cannot atomically replace a directory with a file.
    let blocked = temp.0.join("directory.toml");
    fs::create_dir(&blocked).unwrap();
    assert!(atomic_save(&blocked, &staged, &None).is_err());
    assert!(blocked.is_dir());
    assert_eq!(
        fs::read_dir(&temp.0).unwrap().count(),
        2,
        "failed write removes temp file"
    );
}

#[test]
fn external_changes_require_explicit_reload_or_overwrite_even_for_clean_draft() {
    let temp = Temp::new();
    let path = temp.file();
    fs::write(&path, CUSTOM).unwrap();
    let mut d = draft();
    let external = "# external\nhdr_ui_brightness = 650.0\n";
    fs::write(&path, external).unwrap();
    d.observe_disk(read_disk(&path).unwrap());
    assert!(d.external_change);
    assert!(d.save(&path, false).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    d.set("hdr_ui_brightness", Value::from(450.0)).unwrap();
    d.save(&path, true).unwrap();
    assert!(!d.external_change);
    assert!(fs::read_to_string(&path).unwrap().contains("450.0"));
    fs::write(&path, external).unwrap();
    d.reload_disk(read_disk(&path).unwrap(), Settings::default());
    assert_eq!(d.text, external);
    assert_eq!(d.last_valid.hdr_ui_brightness, 650.0);
    assert!(!d.dirty());
}

#[test]
fn invalid_save_and_discard_restore_valid_saved_state_without_writes() {
    let temp = Temp::new();
    let path = temp.file();
    fs::write(&path, CUSTOM).unwrap();
    let mut d = draft();
    d.text = "style = ['invalid']".into();
    assert!(d.save(&path, false).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), CUSTOM);
    assert!(!d.request_close());
    d.discard();
    assert!(d.request_close());
    // Invalid startup has a separate runtime fallback, not defaults silently written over text.
    let runtime = Settings {
        hdr_ui_brightness: 777.0,
        ..Settings::default()
    };
    let mut invalid_start = Draft::new(Some("mods = [".into()), runtime.clone());
    invalid_start.text = new_document();
    invalid_start.validate();
    invalid_start.discard();
    assert_eq!(invalid_start.mode, Mode::Advanced);
    assert_eq!(invalid_start.text, "mods = [");
    assert_eq!(invalid_start.last_valid, runtime);
}

#[test]
fn preview_isolation_and_periodic_reload_preserve_gameplay_hook_and_path_state() {
    let base = parse(CUSTOM).unwrap();
    let mut rt = RuntimeSettings::new(base.clone());
    let mut edited = base.clone();
    edited.hdr_ui_brightness = 800.0;
    edited.controller_wheel_open_delay = 0.99;
    edited.mods = vec!["different".into()];
    edited.force_rgba = true;
    edited.settings_function_key = "F1".into();
    rt.editing = true;
    rt.preview(&edited);
    assert_eq!(rt.effective.hdr_ui_brightness, 800.0);
    assert_eq!(rt.effective.mods, base.mods);
    assert_eq!(rt.effective.controller_wheel_open_delay, 0.25);
    assert!(!rt.effective.force_rgba);
    assert_eq!(rt.effective.settings_function_key, "F10");
    rt.refresh(&project_defaults());
    assert_eq!(rt.effective.hdr_ui_brightness, 800.0);
    rt.discard();
    assert_eq!(rt.effective, base);
    rt.commit(&edited);
    assert_eq!(rt.saved.settings_function_key, "F1");
    assert_eq!(rt.saved.mods, base.mods);
    assert_eq!(rt.saved.controller_wheel_open_delay, 0.99);
    assert_eq!(rt.effective.controller_wheel_open_delay, 0.99);
    rt.editing = false;
    edited.hdr_ui_brightness = 400.0;
    edited.spells_key = "F8".into();
    edited.timing_offset = 2.0;
    edited.await_xinput_hook = true;
    edited.modded_spells = vec!["different-legacy".into()];
    rt.refresh(&edited);
    assert_eq!(rt.effective.hdr_ui_brightness, 400.0);
    assert_eq!(rt.effective.spells_key, "F8");
    assert_eq!(rt.effective.mods, base.mods);
    assert_eq!(rt.effective.modded_spells, base.modded_spells);
    assert_eq!(rt.effective.force_rgba, base.force_rgba);
    assert_eq!(rt.effective.timing_offset, base.timing_offset);
    assert_eq!(rt.effective.await_xinput_hook, base.await_xinput_hook);
}

#[test]
fn numeric_domains_boundaries_percentages_and_simple_style_are_non_destructive() {
    assert!((to_percent(0.15, 0.15) - 100.0).abs() < 0.0001);
    assert!((from_percent(100.0, 0.15) - 0.15).abs() < 0.0001);
    assert!((to_percent(0.15, 1.0) - 15.0).abs() < 0.0001); // f32 representation; no 0.15 -> 1.0 mutation
    let mut s = project_defaults();
    s.highlight_time = 0.0;
    s.hdr_ui_brightness = 50.0;
    s.hdr_ui_saturation = 0.0;
    validate(&s).unwrap();
    s.hdr_ui_brightness = 2000.0;
    s.hdr_ui_saturation = 2.0;
    validate(&s).unwrap();
    s.icon_scale_multiplier = 2.0;
    s.radius_multiplier = 3.0;
    s.font_scale_multiplier = 8.0;
    validate(&s).unwrap(); // Values well beyond ordinary slider suggestions are legal exact entries.
    for bad in [f32::NAN, f32::INFINITY, -1.0, 2000.1] {
        s.hdr_ui_brightness = bad;
        assert!(validate(&s).is_err());
    }
    s = project_defaults();
    s.radius_multiplier = s.icon_scale_multiplier + 0.01;
    assert!(validate(&s).is_err());
    s.radius_multiplier += 0.001;
    validate(&s).unwrap();
    s.highlight_time = -0.01;
    assert!(validate(&s).is_err());
    let mut d = draft();
    d.set("highlight_expansion", Value::from(0.75)).unwrap();
    d.set("style", Value::from("simple")).unwrap();
    assert_eq!(d.last_valid.highlight_expansion, 0.75);
    d.set("style", Value::from("pretty")).unwrap();
    assert_eq!(d.last_valid.highlight_expansion, 0.75);
}

#[test]
fn invalid_startup_binding_recovers_without_resetting_unrelated_settings_or_text() {
    for field in [
        "settings_function_key = 'F20'",
        "settings_modifier = 'right_shift'",
        "settings_function_key = 123",
        "settings_modifier = ['left_shift', 'left_ctrl']",
    ] {
        let original = format!("{CUSTOM}\n");
        // Put new root keys before [future], not inside that unknown table.
        let text = format!("{field}\n{original}");
        let (settings, warning) = parse_startup(&text).unwrap();
        assert_eq!(
            binding(&settings).unwrap(),
            crate::settings_binding::Binding::DEFAULT
        );
        assert_eq!(settings.controller_wheel_open_delay, 0.25);
        assert_eq!(settings.mods, ["C:\\mods\\QuestPath"]);
        assert_eq!(settings.hdr_ui_brightness, 200.0);
        assert!(warning.unwrap().contains("Left Shift + F10"));
        let d = Draft::new(Some(text.clone()), settings);
        assert_eq!(d.mode, Mode::Advanced);
        assert_eq!(d.text, text);
    }
}

#[test]
fn legacy_case_insensitive_style_and_large_supported_unmanaged_thresholds_are_retained() {
    let text = "style = 'SIMPLE'\nitem_names = 'CENTER'\nmouse_min_radius = 1000.0\ncontroller_min_radius = 1000.0\n";
    let mut d = Draft::new(Some(text.into()), Settings::default());
    assert_eq!(d.mode, Mode::Graphical);
    assert!(d.switch_mode());
    assert!(d.switch_mode());
    assert_eq!(d.text, text);
    assert!(!d.dirty());
}
