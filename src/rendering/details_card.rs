use imgui::{DrawListMut, Ui};

use crate::io::controller_state::Slot;
use crate::rendering::display_item::DisplayItem;
use crate::rendering::wrapped_text::WrappedText;
use crate::settings::{ItemNames, Settings, Style};
use crate::wheel_details::{
    CARD_MARGIN, CARD_PADDING_Y, CARD_ROW_GAP, CardFields, CardSpace, MIN_CARD_FONT,
};

const CARD_PADDING_X: f32 = 10.0;

#[derive(Default)]
pub struct DetailsCard {
    content: Option<CardContent>,
}

struct CardContent {
    slot: Slot,
    fields: CardFields,
    heading: String,
    rows: [String; 3],
    layout: Option<(Layout, Option<MeasuredText>)>,
}

#[derive(Clone, Copy, PartialEq)]
struct Layout {
    width: f32,
    height: f32,
    base_font_size: f32,
    font_multiplier: f32,
}

#[derive(Clone, Copy)]
struct MeasuredText {
    font_scale: f32,
    height: f32,
    row_heights: [f32; 3],
}

impl DetailsCard {
    pub fn clear(&mut self) {
        self.content = None;
    }

    pub fn update(&mut self, slot: Slot, fields: CardFields, heading: &str) {
        if !self.content.as_ref().is_some_and(|content| {
            content.slot == slot && content.fields == fields && content.heading == heading
        }) {
            let (main, status) = fields.text();
            self.content = Some(CardContent {
                slot,
                fields,
                heading: heading.to_owned(),
                rows: [WrappedText::directional_line(heading), main, status],
                layout: None,
            });
        }
    }

    pub fn draw(
        &mut self,
        ui: &Ui,
        draw_list: &DrawListMut<'_>,
        settings: &Settings,
        wheel_size: [f32; 2],
        items: &[DisplayItem],
    ) {
        let Some(content) = self.content.as_mut() else {
            return;
        };
        if items.is_empty() {
            return;
        }

        let [ww, wh] = wheel_size;
        let [vw, vh] = ui.io().display_size;
        let cx = ww / 2.0;
        let space = CardSpace::for_wheel(
            vh,
            wheel_vertical_bounds(wheel_size, items, settings),
            settings.wheel_details_position,
        );
        // Trade horizontal space for fewer wrapped lines when the wheel leaves a short gap.
        let width_ratio = if space.height < 5.0 * MIN_CARD_FONT + 2.0 * CARD_PADDING_Y {
            0.78
        } else {
            0.42
        };
        let width = (ww.min(wh) * width_ratio)
            .clamp(256.0, 640.0)
            .min(2.0 * (cx.min(vw - cx) - CARD_MARGIN));
        if width < 2.0 * CARD_PADDING_X + MIN_CARD_FONT
            || space.height < 2.0 * CARD_PADDING_Y + MIN_CARD_FONT
        {
            return;
        }

        let previous_font_size = ui.current_font_size();
        if previous_font_size <= 0.0 {
            return;
        }
        ui.set_window_font_scale(1.0);
        let base_font_size = ui.current_font_size();
        let layout = Layout {
            width: width - 2.0 * CARD_PADDING_X,
            height: space.height - 2.0 * CARD_PADDING_Y,
            base_font_size,
            font_multiplier: settings.font_scale_multiplier,
        };
        if !content.layout.is_some_and(|(cached, _)| cached == layout) {
            content.layout = Some((layout, measure_text(ui, &content.rows, layout)));
        }

        if let Some((_, Some(text))) = content.layout
            && let Some(top) = space.top(text.height + 2.0 * CARD_PADDING_Y)
        {
            let left = cx - width / 2.0;
            let from = [left, top];
            let to = [left + width, top + text.height + 2.0 * CARD_PADDING_Y];
            draw_list
                .add_rect(from, to, [0.058, 0.051, 0.041, 0.94])
                .rounding(7.0)
                .filled(true)
                .build();
            draw_list
                .add_rect(from, to, [0.58, 0.46, 0.26, 0.80])
                .rounding(7.0)
                .build();

            ui.set_window_font_scale(text.font_scale);
            let text_x = left + CARD_PADDING_X;
            let wrap = ui.push_text_wrap_pos_with_pos(text_x + layout.width);
            let mut y = top + CARD_PADDING_Y;
            let colors = [
                [1.0, 0.89, 0.67, 1.0],
                [0.94, 0.91, 0.84, 1.0],
                [1.0, 0.72, 0.56, 1.0],
            ];
            for ((row, height), color) in content.rows.iter().zip(text.row_heights).zip(colors) {
                if height > 0.0 {
                    ui.set_cursor_screen_pos([text_x, y]);
                    ui.text_colored(color, row);
                    y += height + CARD_ROW_GAP;
                }
            }
            wrap.end();
        }
        ui.set_window_font_scale(previous_font_size / base_font_size);
    }
}

fn measure_text(ui: &Ui, rows: &[String; 3], layout: Layout) -> Option<MeasuredText> {
    let preferred_size =
        (layout.base_font_size * layout.font_multiplier * 0.75).clamp(MIN_CARD_FONT, 36.0);
    // Only reduce text size if the chosen side cannot hold the full wrapped card.
    for size in [preferred_size, MIN_CARD_FONT] {
        let font_scale = size / layout.base_font_size;
        ui.set_window_font_scale(font_scale);
        let row_heights = rows.each_ref().map(|row| {
            if row.is_empty() {
                0.0
            } else {
                ui.calc_text_size_with_opts(row, false, layout.width)[1]
            }
        });
        let row_count = row_heights.iter().filter(|&&height| height > 0.0).count();
        let height =
            row_heights.iter().sum::<f32>() + row_count.saturating_sub(1) as f32 * CARD_ROW_GAP;
        if height <= layout.height {
            return Some(MeasuredText {
                font_scale,
                height,
                row_heights,
            });
        }
    }
    None
}

fn wheel_vertical_bounds(
    [width, height]: [f32; 2],
    items: &[DisplayItem],
    settings: &Settings,
) -> [f32; 2] {
    let short_side = width.min(height);
    let pretty = matches!(settings.style(), Style::Pretty);
    let scale = if pretty {
        1.0 + settings.highlight_expansion
    } else {
        1.0
    };
    let img_dim = settings.icon_scale_multiplier * short_side * scale;
    let center_y = height / 2.0;
    let radius = settings.radius_multiplier * short_side;
    let names = settings.item_names();
    // Reserve the largest footprint for each slot, independent of animation and selection.
    items.iter().fold(
        [center_y - radius, center_y + radius],
        |[top, bottom], item| {
            let name_height = item.name.line_height * item.name.lines.len() as f32;
            let label_height = if matches!(names, ItemNames::Show) {
                name_height * scale
            } else {
                0.0
            };
            let icon_top = item.pos[1] - (img_dim + label_height) / 2.0;
            let label_bottom =
                icon_top + img_dim + label_height * settings.font_scale_multiplier.max(1.0);
            let (above, below) = if pretty {
                (2.0, 2.0 + item.name.line_height * scale * 0.25)
            } else {
                (10.0, 10.0)
            };
            let top = top.min(icon_top - above);
            let bottom = bottom.max(label_bottom + below);
            if matches!(names, ItemNames::Center) {
                let half_name = name_height * settings.font_scale_multiplier / 2.0 + 2.0;
                [
                    top.min(center_y - half_name),
                    bottom.max(center_y + half_name),
                ]
            } else {
                [top, bottom]
            }
        },
    )
}
