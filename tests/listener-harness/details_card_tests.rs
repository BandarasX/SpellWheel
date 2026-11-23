use crate::Settings;
use crate::controller_state::Slot;
use crate::rendering::details_card::DetailsCard;
use crate::rendering::display_item::DisplayItem;
use crate::rendering::wrapped_text::WrappedText;
use crate::settings::DetailPosition;
use crate::wheel_details::{CardFields, FpCost, SlotDetails};
use imgui::{Condition, Context, DrawVert, FontSource, WindowFlags};

fn frame(
    ctx: &mut Context,
    card: &mut DetailsCard,
    settings: &Settings,
    [width, height]: [f32; 2],
) -> Vec<DrawVert> {
    ctx.io_mut().display_size = [width, height];
    ctx.io_mut().font_global_scale = height / 2160.0;
    let ui = ctx.frame();
    ui.window("Item Wheel")
        .position([0.0, 0.0], Condition::Always)
        .size([width, height], Condition::Always)
        .flags(WindowFlags::NO_BACKGROUND)
        .bg_alpha(0.0)
        .no_decoration()
        .no_inputs()
        .build(|| {
            let items = [0.20, 0.80].map(|y| DisplayItem {
                pos: [width / 2.0, height * y],
                name: WrappedText::new(ui, height * 0.15, "Spell"),
            });
            ui.set_window_font_scale(1.15);
            let original_font_size = ui.current_font_size();
            card.draw(
                ui,
                &ui.get_window_draw_list(),
                settings,
                [width, height],
                &items,
            );
            assert!((ui.current_font_size() - original_font_size).abs() < 0.001);
        });
    let draw_data = ctx.render();
    if draw_data.draw_lists_count() == 0 {
        return Vec::new();
    }
    draw_data
        .draw_lists()
        .flat_map(|list| list.vtx_buffer().iter().copied())
        .collect()
}

#[test]
fn production_card_wraps_within_viewport_and_keeps_its_wheel_facing_edge() {
    let _serial = crate::tests::SERIAL.lock().unwrap();
    let mut context = Context::create();
    context.set_ini_filename(None);
    context.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../../assets/font_latin.ttf"),
        size_pixels: 54.0,
        config: None,
    }]);
    context.fonts().build_rgba32_texture();
    let mut card = DetailsCard::default();
    let mut settings = Settings::default();
    let detail = SlotDetails::Spell {
        fp: Some(FpCost {
            cost: 11,
            current: Some(11),
        }),
        requirements: [20, 16, 17],
        effective_stats: [Some(19), Some(25), None],
    };
    let fields = CardFields::for_slot(detail, &settings).unwrap();
    for style in ["Pretty", "Simple"] {
        settings.style = style.into();
        for [width, height] in [[3440.0, 1440.0], [1280.0, 720.0]] {
            for position in [DetailPosition::Above, DetailPosition::Below] {
                settings.wheel_details_position = position;
                let slot = Slot {
                    index: 0,
                    param_id: 100,
                };
                card.update(slot, fields, "Spell");
                frame(&mut context, &mut card, &settings, [width, height]);
                let short = frame(&mut context, &mut card, &settings, [width, height]);
                card.update(Slot { index: 1, ..slot }, fields, "A very long spell name, Destruction of the Ancient Celestial Tower and Its Guardians");
                let long = frame(&mut context, &mut card, &settings, [width, height]);
                assert!(
                    !short.is_empty() && !long.is_empty(),
                    "{style} {width}x{height} {position:?}"
                );
                assert!(long.len() > short.len());
                let near_edge = |points: &[DrawVert]| {
                    if position == DetailPosition::Above {
                        points.iter().map(|p| p.pos[1]).reduce(f32::max).unwrap()
                    } else {
                        points.iter().map(|p| p.pos[1]).reduce(f32::min).unwrap()
                    }
                };
                // Longer wrapped content can use the remaining travel toward the wheel.
                assert!((near_edge(&short) - near_edge(&long)).abs() <= 20.01);
                assert!(long.iter().all(|vertex| {
                    let [x, y] = vertex.pos;
                    x > 0.0 && x < width && y > 0.0 && y < height
                }));
                match position {
                    DetailPosition::Above => assert!(near_edge(&long) < height * 0.125),
                    DetailPosition::Below => assert!(near_edge(&long) > height * 0.875),
                }
            }
        }
    }
    settings.wheel_details_background_transparency = 0.5;
    settings.wheel_details_gold_border = true;
    let with_border = frame(&mut context, &mut card, &settings, [3440.0, 1440.0]);
    settings.wheel_details_gold_border = false;
    let without_border = frame(&mut context, &mut card, &settings, [3440.0, 1440.0]);
    let color = |vertices: &[DrawVert], rgba| vertices.iter().any(|v| v.col == rgba);
    assert!(color(&with_border, [148, 117, 66, 204]));
    assert!(!color(&without_border, [148, 117, 66, 204]));
    for vertices in [&with_border, &without_border] {
        assert!(color(vertices, [15, 13, 10, 128]));
        assert!(color(vertices, [168, 201, 161, 255]));
        assert!(color(vertices, [219, 153, 145, 255]));
    }
    settings.wheel_details_background_transparency = 1.0;
    let no_fill = frame(&mut context, &mut card, &settings, [3440.0, 1440.0]);
    assert!(!color(&no_fill, [15, 13, 10, 128]));
    assert!(color(&no_fill, [168, 201, 161, 255]));
    card.clear();
    assert!(frame(&mut context, &mut card, &settings, [1280.0, 720.0]).is_empty());
}
