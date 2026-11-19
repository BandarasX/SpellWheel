use super::*;
use imgui::Context;

fn context() -> Context {
    let mut ctx = Context::create();
    ctx.set_ini_filename(None);
    ctx.set_log_filename(None);
    ctx.io_mut().config_windows_move_from_title_bar_only = true;
    ctx.fonts().add_font(&[panel_font_source()]);
    ctx.fonts().build_rgba32_texture();
    ctx.io_mut()
        .config_flags
        .insert(imgui::ConfigFlags::NAV_ENABLE_KEYBOARD);
    ctx
}
fn frame(ctx: &mut Context, panel: &mut DisplaySettings, size: [f32; 2]) {
    ctx.io_mut().display_size = size;
    ctx.io_mut().font_global_scale = size[0].min(size[1]) / 2160.0;
    ctx.io_mut().delta_time = 1.0 / 60.0;
    let ui = ctx.frame();
    panel.draw(ui);
    ctx.render();
}
fn click(ctx: &mut Context, panel: &mut DisplaySettings, label: &str, size: [f32; 2]) {
    let (_, min, max) = panel
        .bounds
        .iter()
        .find(|(name, _, _)| name == label)
        .unwrap_or_else(|| panic!("No control {label}: {:?}", panel.bounds));
    ctx.io_mut()
        .add_mouse_pos_event([(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5]);
    ctx.io_mut()
        .add_mouse_button_event(imgui::MouseButton::Left, true);
    frame(ctx, panel, size);
    ctx.io_mut()
        .add_mouse_button_event(imgui::MouseButton::Left, false);
    frame(ctx, panel, size);
    frame(ctx, panel, size);
}
fn open() -> DisplaySettings {
    std::fs::write(
        crate::paths::settings(),
        "# UI fixture\ncontroller_wheel_open_delay = 0.25\n",
    )
    .unwrap();
    let mut panel = DisplaySettings::new();
    panel.open();
    panel
}
#[test]
fn real_imgui_footer_remains_inside_small_and_ultrawide_viewports_in_both_modes() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    for size in [
        [3440.0, 1440.0],
        [1280.0, 720.0],
        [800.0, 600.0],
        [640.0, 480.0],
    ] {
        for mode in [Mode::Graphical, Mode::Advanced] {
            panel.draft.mode = mode;
            for _ in 0..3 {
                frame(&mut ctx, &mut panel, size);
            }
            for name in [
                "Reset to Defaults...",
                "Save",
                "Discard",
                if mode == Mode::Graphical {
                    "Advanced mode"
                } else {
                    "Graphical mode"
                },
            ] {
                let (_, min, max) = panel
                    .bounds
                    .iter()
                    .find(|(label, _, _)| label == name)
                    .unwrap();
                assert!(
                    min[0] >= 0.0 && min[1] >= 0.0 && max[0] <= size[0] && max[1] <= size[1],
                    "{name} outside {size:?}: {min:?}..{max:?}"
                );
            }
        }
    }
    panel.close();
    let _ = std::fs::remove_file(crate::paths::settings());
}
#[test]
fn real_imgui_reset_cancel_confirm_discard_and_invalid_mode_switch() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let size = [1280.0, 720.0];
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    let original = panel.draft.text.clone();
    click(&mut ctx, &mut panel, "Reset to Defaults...", size);
    assert_eq!(panel.modal, Some(Modal::Reset));
    assert!(panel.reset_current);
    click(&mut ctx, &mut panel, "Cancel", size);
    assert_eq!(panel.draft.text, original);
    click(&mut ctx, &mut panel, "Reset to Defaults...", size);
    click(&mut ctx, &mut panel, "Confirm reset", size);
    assert!(panel.draft.dirty());
    assert_eq!(panel.draft.last_valid.controller_wheel_open_delay, 0.25);
    click(&mut ctx, &mut panel, "Discard", size);
    assert_eq!(panel.draft.text, original);
    click(&mut ctx, &mut panel, "Advanced mode", size);
    assert_eq!(panel.draft.mode, Mode::Advanced);
    panel.draft.text = "hdr_ui_brightness = nan".into();
    panel.preview();
    frame(&mut ctx, &mut panel, size);
    click(&mut ctx, &mut panel, "Graphical mode", size);
    assert_eq!(panel.draft.mode, Mode::Advanced);
    assert_eq!(panel.draft.text, "hdr_ui_brightness = nan");
    click(&mut ctx, &mut panel, "Discard", size);
    click(&mut ctx, &mut panel, "Reset to Defaults...", size);
    assert!(!panel.reset_current);
    click(&mut ctx, &mut panel, "Cancel", size);
    panel.close();
    let _ = std::fs::remove_file(crate::paths::settings());
}
#[test]
fn dirty_close_continue_and_failed_save_remain_reachable() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let size = [1280.0, 720.0];
    panel.set("hdr_ui_brightness", 550.0);
    panel.request_close();
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    assert!(panel.shown);
    assert_eq!(panel.modal, Some(Modal::Close));
    click(&mut ctx, &mut panel, "Continue editing", size);
    assert!(panel.shown);
    assert!(panel.draft.dirty());
    panel.request_close();
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    // External edit forces conflict; dismissing it must preserve a usable panel.
    std::fs::write(crate::paths::settings(), "# external\n").unwrap();
    click(&mut ctx, &mut panel, "Save and close", size);
    assert_eq!(panel.modal, Some(Modal::Conflict));
    click(&mut ctx, &mut panel, "Keep editing", size);
    assert!(panel.shown);
    assert!(panel.draft.dirty());
    panel.request_close();
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    click(&mut ctx, &mut panel, "Discard and close", size);
    assert!(!panel.shown);
    let _ = std::fs::remove_file(crate::paths::settings());
}

#[test]
fn headers_collapse_with_keyboard_focus_and_independent_section_state() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let size = [3440.0, 1440.0];
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    click(&mut ctx, &mut panel, "Selection effect", size);
    assert!(
        panel
            .sections
            .iter()
            .any(|(n, open, _)| n == "Wheel layout" && *open)
    );
    assert!(
        panel
            .sections
            .iter()
            .any(|(n, open, _)| n == "Selection effect" && *open)
    );
    ctx.io_mut().add_key_event(imgui::Key::LeftArrow, true);
    frame(&mut ctx, &mut panel, size);
    ctx.io_mut().add_key_event(imgui::Key::LeftArrow, false);
    frame(&mut ctx, &mut panel, size);
    assert!(
        panel
            .sections
            .iter()
            .any(|(n, open, focused)| n == "Selection effect" && !*open && *focused),
        "collapsed header retains keyboard focus: {:?}",
        panel.sections
    );
    assert!(
        panel
            .sections
            .iter()
            .any(|(n, open, _)| n == "Wheel layout" && *open)
    );
    click(&mut ctx, &mut panel, "Advanced mode", size);
    click(&mut ctx, &mut panel, "Graphical mode", size);
    assert!(
        panel
            .sections
            .iter()
            .any(|(n, open, _)| n == "Wheel layout" && *open)
    );
    assert!(
        panel
            .sections
            .iter()
            .any(|(n, open, _)| n == "Selection effect" && !*open)
    );
    panel.close();
    let _ = std::fs::remove_file(crate::paths::settings());
}

#[test]
fn io_failure_in_save_and_close_keeps_modal_and_draft_reachable() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let size = [1280.0, 720.0];
    panel.set("hdr_ui_brightness", 570.0);
    let text = panel.draft.text.clone();
    panel.request_close();
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    std::fs::remove_file(crate::paths::settings()).unwrap();
    std::fs::create_dir(crate::paths::settings()).unwrap();
    click(&mut ctx, &mut panel, "Save and close", size);
    assert_eq!(panel.modal, Some(Modal::Close));
    assert!(panel.shown);
    assert_eq!(panel.draft.text, text);
    click(&mut ctx, &mut panel, "Continue editing", size);
    assert_eq!(panel.modal, None);
    assert!(panel.draft.dirty());
    panel.close();
    std::fs::remove_dir(crate::paths::settings()).unwrap();
}

#[test]
fn unreadable_open_retains_runtime_and_requires_explicit_successful_reload() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let saved = Settings {
        hdr_ui_brightness: 777.0,
        ..Settings::default()
    };
    SettingsContext::committed(&saved);
    std::fs::create_dir(crate::paths::settings()).unwrap();
    let mut panel = DisplaySettings::new();
    panel.open();
    assert!(panel.read_error);
    assert!(!panel.save(false));
    panel.discard();
    assert_eq!(Settings::read_or_default().hdr_ui_brightness, 777.0);
    std::fs::remove_dir(crate::paths::settings()).unwrap();
    std::fs::write(crate::paths::settings(), "hdr_ui_brightness = 650.0\n").unwrap();
    panel.reload_disk();
    assert!(!panel.read_error);
    assert_eq!(Settings::read_or_default().hdr_ui_brightness, 650.0);
    panel.close();
    std::fs::remove_file(crate::paths::settings()).unwrap();
}

fn drag(
    ctx: &mut Context,
    panel: &mut DisplaySettings,
    size: [f32; 2],
    from: [f32; 2],
    to: [f32; 2],
) {
    ctx.io_mut().add_mouse_pos_event(from);
    frame(ctx, panel, size);
    ctx.io_mut()
        .add_mouse_button_event(imgui::MouseButton::Left, true);
    frame(ctx, panel, size);
    ctx.io_mut().add_mouse_pos_event(to);
    frame(ctx, panel, size);
    ctx.io_mut()
        .add_mouse_button_event(imgui::MouseButton::Left, false);
    frame(ctx, panel, size);
}

#[test]
fn panel_opens_left_and_can_be_dragged_and_resized() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let viewport = [3440.0, 1440.0];
    frame(&mut ctx, &mut panel, viewport);
    let (_, [x, y], [width, height]) = panel.window.unwrap();
    assert_eq!([width, height], [620.0, 560.0]);
    assert!(x < 50.0);
    drag(
        &mut ctx,
        &mut panel,
        viewport,
        [x + 100.0, y + 10.0],
        [x + 250.0, y + 60.0],
    );
    let (_, [x, y], size) = panel.window.unwrap();
    assert!(x >= 150.0 && y >= 480.0);
    assert_eq!(size, [width, height]);
    let corner = [x + width - 5.0, y + height - 5.0];
    drag(
        &mut ctx,
        &mut panel,
        viewport,
        corner,
        [corner[0] - 140.0, corner[1] - 130.0],
    );
    let (_, _, [resized_width, resized_height]) = panel.window.unwrap();
    assert!(resized_width < width && resized_height < height);
    frame(&mut ctx, &mut panel, [640.0, 480.0]);
    let (_, [x, y], [width, height]) = panel.window.unwrap();
    assert!(x >= 0.0 && y >= 0.0 && x + width <= 640.0 && y + height <= 480.0);
    panel.close();
    std::fs::remove_file(crate::paths::settings()).unwrap();
}

#[test]
fn percent_slider_preserves_a_value_outside_its_range_until_edited() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let viewport = [3440.0, 1440.0];
    panel.set("radius_multiplier", 0.9);
    let draft = panel.draft.text.clone();
    frame(&mut ctx, &mut panel, viewport);
    frame(&mut ctx, &mut panel, viewport);
    assert_eq!(panel.draft.text, draft);
    assert_eq!(panel.draft.last_valid.radius_multiplier, 0.9);
    click(&mut ctx, &mut panel, "radius_multiplier", viewport);
    let percent = panel_model::to_percent(panel.draft.last_valid.radius_multiplier, 0.3);
    assert!((50.0..=200.0).contains(&percent));
    assert!((percent * 2.0 - (percent * 2.0).round()).abs() < 0.0001);
    assert_ne!(panel.draft.text, draft);
    panel.close();
    std::fs::remove_file(crate::paths::settings()).unwrap();
}

#[test]
fn advanced_live_settings_apply_only_after_successful_save() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut panel = open();
    assert!(panel.draft.switch_mode());
    let active = Settings::read_or_default();
    let original = panel.draft.saved.clone();
    panel.draft.text = r#"
spell_wheel_enabled = false
item_wheel_enabled = false
spells_key = 'F8'
quick_items_key = 'F9'
center_on_close = false
switch_instantly = false
mouse_min_radius = 0.65
controller_min_radius = 0.75
controller_wheel_open_delay = 0.75
debugging = true
mods = ['different-mod']
modded_spells = ['legacy-mod']
force_rgba = true
timing_offset = 0.5
await_xinput_hook = true
"#
    .into();
    let submitted = panel_model::parse(&panel.draft.text).unwrap();
    panel.preview();
    assert_eq!(Settings::read_or_default(), active);

    // A directory in place of the file forces Save to fail before runtime commit.
    let path = crate::paths::settings();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(!panel.save(false));
    assert_eq!(Settings::read_or_default(), active);
    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, &original).unwrap();

    assert!(panel.save(false));
    assert_eq!(
        panel_model::parse(&std::fs::read_to_string(&path).unwrap()).unwrap(),
        submitted
    );
    let expected = Settings {
        mods: active.mods.clone(),
        modded_spells: active.modded_spells.clone(),
        force_rgba: active.force_rgba,
        timing_offset: active.timing_offset,
        await_xinput_hook: active.await_xinput_hook,
        ..submitted
    };
    assert_eq!(Settings::read_or_default(), expected);

    panel.draft.text = original;
    panel.preview();
    panel.discard();
    assert_eq!(Settings::read_or_default(), expected);
    panel.close();
    assert_eq!(Settings::read_or_default(), expected);
    panel.open();
    assert_eq!(Settings::read_or_default(), expected);
    panel.close();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn percentage_entry_retains_precision_and_keyboard_slider_advances_by_half_points() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let viewport = [3440.0, 1440.0];
    // A TOML value can carry more precision than the graphical field displays.
    let original = panel_model::from_percent(123.274, 0.3);
    panel.set("radius_multiplier", panel_model::float_value(original));
    frame(&mut ctx, &mut panel, viewport);
    frame(&mut ctx, &mut panel, viewport);
    click(&mut ctx, &mut panel, "radius_multiplier exact", viewport);
    assert_eq!(panel.draft.last_valid.radius_multiplier, original);
    for ch in "123.37".chars() {
        ctx.io_mut().add_input_character(ch);
    }
    frame(&mut ctx, &mut panel, viewport);
    let percent = |panel: &DisplaySettings| {
        panel_model::to_percent(panel.draft.last_valid.radius_multiplier, 0.3)
    };
    assert!((percent(&panel) - 123.37).abs() < 0.0001);

    // Navigate left from the exact entry to its slider, then activate it.
    fn key(ctx: &mut Context, panel: &mut DisplaySettings, viewport: [f32; 2], key: Key) {
        ctx.io_mut().add_key_event(key, true);
        frame(ctx, panel, viewport);
        ctx.io_mut().add_key_event(key, false);
        frame(ctx, panel, viewport);
    }
    key(&mut ctx, &mut panel, viewport, Key::Enter);
    key(&mut ctx, &mut panel, viewport, Key::LeftArrow);
    key(&mut ctx, &mut panel, viewport, Key::Space);
    key(&mut ctx, &mut panel, viewport, Key::RightArrow);
    assert!(
        (percent(&panel) - 123.5).abs() < 0.0001,
        "{}",
        percent(&panel)
    );
    key(&mut ctx, &mut panel, viewport, Key::RightArrow);
    assert!((percent(&panel) - 124.0).abs() < 0.0001);
    key(&mut ctx, &mut panel, viewport, Key::LeftArrow);
    assert!((percent(&panel) - 123.5).abs() < 0.0001);
    panel.close();
    std::fs::remove_file(crate::paths::settings()).unwrap();
}

#[test]
fn keyboard_mouse_ring_can_be_saved_discarded_and_reset_in_appearance() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut ctx = context();
    let mut panel = open();
    let size = [1280.0, 720.0];
    const TOGGLE: &str = "Show inner ring with keyboard and mouse";
    frame(&mut ctx, &mut panel, size);
    frame(&mut ctx, &mut panel, size);
    let effective = || Settings::read_or_default().keyboard_mouse_inner_ring;
    let saved = || {
        panel_model::parse(&std::fs::read_to_string(crate::paths::settings()).unwrap())
            .unwrap()
            .keyboard_mouse_inner_ring
    };
    assert!(!effective() && !saved());
    click(&mut ctx, &mut panel, TOGGLE, size);
    assert!(effective() && !saved());
    click(&mut ctx, &mut panel, "Discard", size);
    assert!(!effective());
    click(&mut ctx, &mut panel, TOGGLE, size);
    click(&mut ctx, &mut panel, "Save", size);
    assert!(effective() && saved());
    click(&mut ctx, &mut panel, "Reset to Defaults...", size);
    click(&mut ctx, &mut panel, "Confirm reset", size);
    assert!(!effective() && saved());
    click(&mut ctx, &mut panel, "Save", size);
    assert!(!effective() && !saved());
    panel.close();
    std::fs::remove_file(crate::paths::settings()).unwrap();
}
