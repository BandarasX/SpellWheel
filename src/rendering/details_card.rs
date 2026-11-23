use imgui::{DrawListMut, Ui};

use crate::io::controller_state::Slot;
use crate::rendering::display_item::DisplayItem;
use crate::rendering::wrapped_text::WrappedText;
use crate::settings::{ItemNames, Settings, Style};
use crate::wheel_details::{
    CARD_MARGIN, CARD_PADDING_Y, CARD_ROW_GAP, CardFields, CardSpace, MIN_CARD_FONT,
    RequirementState,
};

const CARD_PADDING_X: f32 = 10.0;
const TEXT: [f32; 4] = [0.94, 0.91, 0.84, 1.0];
const GOLD: [f32; 4] = [1.0, 0.89, 0.67, 1.0];
const MUTED_GREEN: [f32; 4] = [0.66, 0.79, 0.63, 1.0];
const MUTED_RED: [f32; 4] = [0.86, 0.60, 0.57, 1.0];

#[derive(Default)]
pub struct DetailsCard {
    content: Option<CardContent>,
}

struct CardContent {
    slot: Slot,
    fields: CardFields,
    heading: String,
    rows: Vec<Vec<TextRun>>,
    status: TextRun,
    layout: Option<(Layout, Option<MeasuredText>)>,
}

#[derive(Clone, Copy, PartialEq)]
struct Layout {
    width: f32,
    height: f32,
    base_font_size: f32,
    font_multiplier: f32,
}

struct TextRun {
    text: String,
    color: [f32; 4],
}

impl TextRun {
    fn new(text: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            text: text.into(),
            color,
        }
    }
}

struct MeasuredText {
    font_scale: f32,
    font_size: f32,
    height: f32,
    lines: Vec<MeasuredLine>,
}

#[derive(Default)]
struct MeasuredLine {
    width: f32,
    y: f32,
    font_scale: Option<f32>,
    runs: Vec<(f32, TextRun)>,
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
            let main = if fields == CardFields::default() {
                "No item details to display".to_owned()
            } else {
                main
            };
            let mut rows = vec![
                vec![TextRun::new(WrappedText::directional_line(heading), GOLD)],
                vec![TextRun::new(main, TEXT)],
            ];
            let mut requirements: Vec<_> = fields
                .requirements()
                .map(|(stat, requirement)| {
                    let color = match requirement.state {
                        RequirementState::Met => MUTED_GREEN,
                        RequirementState::Unmet => MUTED_RED,
                        RequirementState::Unknown => TEXT,
                    };
                    TextRun::new(format!("{} {stat}", requirement.amount), color)
                })
                .collect();
            if !requirements.is_empty() {
                requirements.insert(0, TextRun::new("Requires", TEXT));
                rows.push(requirements);
            }
            self.content = Some(CardContent {
                slot,
                fields,
                heading: heading.to_owned(),
                rows,
                status: TextRun::new(status, [1.0, 0.72, 0.56, 1.0]),
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
        if !content
            .layout
            .as_ref()
            .is_some_and(|(cached, _)| *cached == layout)
        {
            content.layout = Some((
                layout,
                measure_text(ui, &content.rows, &content.status, layout),
            ));
        }

        if let Some((_, Some(text))) = &content.layout {
            let card_height = text.height + 2.0 * CARD_PADDING_Y;
            let reserve = 3.0 * text.font_size + 2.0 * (CARD_PADDING_Y + CARD_ROW_GAP);
            if let Some(top) = space.top(card_height, reserve) {
                let left = cx - width / 2.0;
                let from = [left, top];
                let to = [left + width, top + card_height];
                draw_list
                    .add_rect(
                        from,
                        to,
                        [
                            0.058,
                            0.051,
                            0.041,
                            1.0 - settings.wheel_details_background_transparency,
                        ],
                    )
                    .rounding(7.0)
                    .filled(true)
                    .build();
                if settings.wheel_details_gold_border {
                    draw_list
                        .add_rect(from, to, [0.58, 0.46, 0.26, 0.80])
                        .rounding(7.0)
                        .build();
                }

                ui.set_window_font_scale(text.font_scale);
                for line in &text.lines {
                    if let Some(scale) = line.font_scale {
                        ui.set_window_font_scale(scale);
                    }
                    let left = cx - line.width / 2.0;
                    for (offset, run) in &line.runs {
                        draw_list.add_text(
                            [left + offset, top + CARD_PADDING_Y + line.y],
                            run.color,
                            &run.text,
                        );
                    }
                }
            }
        }
        ui.set_window_font_scale(previous_font_size / base_font_size);
    }
}

impl MeasuredLine {
    fn push(&mut self, text: &str, color: [f32; 4], width: f32, space: f32) {
        let x = self.width + if self.runs.is_empty() { 0.0 } else { space };
        self.runs.push((x, TextRun::new(text, color)));
        self.width = x + width;
    }
}

fn wrap_row(ui: &Ui, row: &[TextRun], width: f32) -> Vec<MeasuredLine> {
    let space = ui.calc_text_size(" ")[0];
    let mut lines = Vec::new();
    let mut line = MeasuredLine::default();
    for run in row {
        for word in run.text.split_whitespace() {
            let word_width = ui.calc_text_size(word)[0];
            if !line.runs.is_empty() && line.width + space + word_width > width {
                lines.push(std::mem::take(&mut line));
            }
            if word_width <= width {
                line.push(word, run.color, word_width, space);
                continue;
            }

            // Some languages and modded names contain no whitespace.
            let mut start = 0;
            for (index, character) in word.char_indices() {
                let next = index + character.len_utf8();
                if index > start && ui.calc_text_size(&word[start..next])[0] > width {
                    let part = &word[start..index];
                    line.push(part, run.color, ui.calc_text_size(part)[0], space);
                    lines.push(std::mem::take(&mut line));
                    start = index;
                }
            }
            let last = &word[start..];
            line.push(last, run.color, ui.calc_text_size(last)[0], space);
        }
    }
    if !line.runs.is_empty() {
        lines.push(line);
    }
    lines
}

fn measure_text(
    ui: &Ui,
    rows: &[Vec<TextRun>],
    status: &TextRun,
    layout: Layout,
) -> Option<MeasuredText> {
    let preferred_size =
        (layout.base_font_size * layout.font_multiplier * 0.75).clamp(MIN_CARD_FONT, 36.0);
    // The layout budget already allows the minimum wheel clearance. Shrink only if needed.
    for size in [preferred_size, MIN_CARD_FONT] {
        let font_scale = size / layout.base_font_size;
        ui.set_window_font_scale(font_scale);
        let mut height = 0.0;
        let mut lines = Vec::new();
        for row in rows {
            let mut row = wrap_row(ui, row, layout.width);
            if row.is_empty() {
                continue;
            }
            if !lines.is_empty() {
                height += CARD_ROW_GAP;
            }
            for line in &mut row {
                line.y = height;
                height += size;
            }
            lines.extend(row);
        }
        if !status.text.is_empty() {
            if !lines.is_empty() {
                height += CARD_ROW_GAP;
            }
            let width = ui.calc_text_size(&status.text)[0];
            // Fit all warnings on one line, independently of the other rows.
            let scale = ((layout.width - 1.0) / width).min(1.0);
            ui.set_window_font_scale(font_scale * scale);
            let mut line = MeasuredLine {
                y: height,
                font_scale: Some(font_scale * scale),
                ..Default::default()
            };
            line.push(
                &status.text,
                status.color,
                ui.calc_text_size(&status.text)[0],
                0.0,
            );
            lines.push(line);
            height += size * scale;
        }
        if height <= layout.height {
            return Some(MeasuredText {
                font_scale,
                font_size: size,
                height,
                lines,
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
