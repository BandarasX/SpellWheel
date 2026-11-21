use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::font::FontId;
use crate::glyphs::font_manager::FontManager;
use crate::hwindow::get_window_size;
use crate::icons::icon_manager::IconManager;
use crate::items::Item;
use crate::mouse::reset_cursor_pos;
use crate::rendering::ITEM_WHEEL_DATA;
use crate::rendering::display_item::DisplayItem;
use crate::rendering::wheel_renderer::{WheelType, render_wheel};
use crate::settings::Settings;
use crate::{guard, set_hwnd, set_selected_quick_item_index, set_selected_spell_index};
use hudhook::{ImguiRenderLoop, RenderContext};
use imgui::{Context, Ui, WindowFlags};
use ordered_float::OrderedFloat;
use windows::Win32::UI::WindowsAndMessaging::{WM_ACTIVATEAPP, WM_KILLFOCUS};
use crate::io::controller;
use crate::io::controller_state::{Slot, VisibleWheel};
use crate::icons::texture_manager::TextureManager;
use crate::rendering::popup_renderer::{add_popup, draw_popups};
use crate::settings::display_settings::{DisplaySettings, PANEL_FONT_SIZE, panel_font_source};

pub struct RenderHook {
    font: FontId,
    panel_font: FontId,
    display_spells: Vec<DisplayItem>,
    display_quick_items: Vec<DisplayItem>,
    display_settings: DisplaySettings,
    prev_type: WheelType,
    preview_type: WheelType,
    prev_controller_wheel: Option<VisibleWheel>,
    controller_highlight: Option<Slot>,
    preview_controller_style: bool,
    prev_layout: Option<(f32, f32, [f32; 2])>,
    prev_spells: Vec<Item>,
    prev_quick_items: Vec<Item>,
}

impl RenderHook {
    pub fn new() -> Self {
        Self {
            font: FontId::none(),
            panel_font: FontId::none(),
            display_spells: vec![],
            display_quick_items: vec![],
            display_settings: DisplaySettings::new(),
            prev_type: WheelType::None,
            preview_type: WheelType::None,
            prev_controller_wheel: None,
            controller_highlight: None,
            preview_controller_style: true,
            prev_layout: None,
            prev_spells: vec![],
            prev_quick_items: vec![],
        }
    }

    fn switch_item(&self) {
        match self.prev_type {
            WheelType::Spells => {
                if let Some(item) = self.display_spells.iter()
                    .find(|item| item.is_highlighted) {

                    set_selected_spell_index(item.slot.index);
                }
            }
            WheelType::QuickItems => {
                if let Some(item) = self.display_quick_items.iter()
                    .find(|item| item.is_highlighted) {

                    set_selected_quick_item_index(item.slot.index);
                }
            }
            WheelType::None => {}
        }
    }
}

impl RenderHook {
    fn try_resize_font(&mut self, ctx: &mut Context) {
        let [ww, wh] = get_window_size();
        ctx.io_mut().font_global_scale = ww.min(wh) / DEFAULT_SCREEN_MIN;
    }
}

impl ImguiRenderLoop for RenderHook {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        guard!(
            tracing::info!("Initializing item wheel UI");

            tracing::info!("Setting HWND...");
            set_hwnd();
            tracing::info!("Set HWND");

            tracing::info!("Loading font...");

            hudhook::set_input_message_filter(|message, key, lparam| {
                if message == WM_KILLFOCUS || (message == WM_ACTIVATEAPP && key == 0) {
                    controller::interrupt();
                }
                crate::settings::display_settings::input_message_filter(message, key, lparam)
            });
            ctx.io_mut().config_flags.insert(imgui::ConfigFlags::NAV_ENABLE_KEYBOARD);
            ctx.io_mut().config_windows_move_from_title_bar_only = true;
            let mut sources = FontManager::generate_sources();
            self.font = ctx.fonts().add_font(&sources).into();
            // Keep the configured non-Latin glyphs available in Advanced TOML text.
            sources[0] = panel_font_source();
            for source in &mut sources[1..] {
                if let imgui::FontSource::TtfData { size_pixels, .. } = source {
                    *size_pixels = PANEL_FONT_SIZE;
                }
            }
            self.panel_font = ctx.fonts().add_font(&sources).into();
            tracing::info!("Font loaded");
            IconManager::load(render_context);
        );
    }

    fn before_render<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        guard!(
            self.try_resize_font(ctx);
            let was_open = self.display_settings.shown;
            self.display_settings.update();
            if !was_open && self.display_settings.shown {
                controller::interrupt();
                self.preview_controller_style = self.prev_type == WheelType::None || self.prev_controller_wheel.is_some();
                self.preview_type = match self.prev_type {
                    WheelType::None => WheelType::QuickItems,
                    visible_wheel => visible_wheel,
                };
            }
            let hdr = Settings::read_or_default();
            hudhook::set_dx12_hdr_settings(
                hdr.hdr_enabled,
                hdr.hdr_ui_brightness,
                hdr.hdr_ui_saturation,
            );
            if self.display_settings.reload_stage == 1 {
                self.display_settings.reload_stage += 1;
            } else if self.display_settings.reload_stage == 2 {
                self.display_settings.reload_stage += 1;
                IconManager::reinit();
                tracing::info!("IconManager reinitialized");
                add_popup("IconManager reinitialized (1/2)".to_string());
            } else if self.display_settings.reload_stage == 3 {
                self.display_settings.reload_stage = 0;
                self.prev_spells = vec![];
                self.prev_quick_items = vec![];
                TextureManager::unload_all();
                IconManager::load(render_context);
                tracing::info!("IconManager reloaded");
                add_popup("IconManager reloaded (2/2)".to_string());
            }
        );
    }

    fn message_filter(&self, _io: &imgui::Io) -> hudhook::MessageFilter {
        if self.display_settings.shown { hudhook::MessageFilter::InputAll }
        else { hudhook::MessageFilter::empty() }
    }

    fn render(&mut self, ui: &mut Ui) {
        #[cfg(feature = "settings-diagnostics")]
        if hudhook::settings_diagnostics::sampled() {
            hudhook::settings_diagnostics::event(format_args!("render_loop_begin listener={:x} shown={} display={:?}",
                self.display_settings.shortcut.diagnostic_id(), self.display_settings.shown, ui.io().display_size));
        }
        guard!(
            let font = ui.push_font(self.font.into());
            let (mut wheel_type, controller_wheel, quick_items, spells) = ItemWheelData::get(|data|
                (data.wheel_type, data.controller_wheel, data.quick_items.clone(), data.spells.clone())
            );
            let stick = controller_wheel.and_then(controller::right_stick);
            if controller_wheel.is_some() && stick.is_none() {
                wheel_type = WheelType::None;
            }
            if self.prev_controller_wheel != controller_wheel {
                self.controller_highlight = None;
            }

            let settings = Settings::read_or_default();
            let layout = (settings.icon_scale_multiplier, settings.radius_multiplier, ui.io().display_size);
            let layout_changed = self.prev_layout != Some(layout);
            self.prev_layout = Some(layout);

            if !self.display_settings.shown && (self.prev_type != WheelType::None) && (wheel_type == WheelType::None) &&
                self.prev_controller_wheel.is_none() && settings.center_on_close {
                reset_cursor_pos();
            }

            let switch_items = !self.display_settings.shown && self.prev_controller_wheel.is_none()
                && (settings.switch_instantly || self.prev_type != wheel_type);
            if switch_items {
                self.switch_item();
            }

            let [sw, sh] = ui.io().display_size;
            ui.window("Item Wheel")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([sw, sh], imgui::Condition::Always)
                .flags(
                    WindowFlags::NO_TITLE_BAR |
                    WindowFlags::NO_RESIZE |
                    WindowFlags::NO_SCROLLBAR |
                    WindowFlags::NO_SCROLL_WITH_MOUSE |
                    WindowFlags::NO_BACKGROUND
                )
                .bg_alpha(0.0)
                .no_decoration()
                .no_inputs()
                .movable(false)
                .build(|| {
                    if is_debugging() {
                        tracing::info!("ImGui Display size: {:?}", ui.io().display_size);
                        tracing::info!("ImGui window size: {:?}", ui.window_size());
                        tracing::info!("Process window size: {:?}", get_window_size());
                    }

                    if self.prev_spells != spells || layout_changed {
                        if is_debugging() {
                            tracing::info!("Remaking display spells");
                        }
                        self.display_spells = DisplayItem::from_items(ui, &spells);
                    }
                    self.display_spells.sort_by_key(|di| OrderedFloat(di.highlight_time));
                    if self.prev_quick_items != quick_items || layout_changed {
                        if is_debugging() {
                            tracing::info!("Remaking display quick items");
                        }
                        self.display_quick_items = DisplayItem::from_items(ui, &quick_items);
                    }
                    self.display_quick_items.sort_by_key(|di| OrderedFloat(di.highlight_time));

                    if is_debugging() {
                        add_to_screen_debug(format!("Spells: {:?}", spells));
                        add_to_screen_debug(format!("Quick items: {:?}", quick_items));
                        add_to_screen_debug(format!("Display spells: {:?}", self.display_spells));
                        add_to_screen_debug(format!("Display quick items: {:?}", self.display_quick_items));
                    }

                    let visible_type = if self.display_settings.shown {
                        match self.preview_type {
                            WheelType::QuickItems if quick_items.is_empty() => WheelType::Spells,
                            WheelType::Spells if spells.is_empty() => WheelType::QuickItems,
                            preview => preview,
                        }
                    } else {
                        wheel_type
                    };
                    let display_items = match visible_type {
                        WheelType::Spells => &mut self.display_spells,
                        WheelType::QuickItems => &mut self.display_quick_items,
                        WheelType::None => &mut vec![],
                    };

                    if is_debugging() {
                        add_to_screen_debug(format!("Display items: {display_items:?}"));
                    }

                    self.prev_spells = spells;
                    self.prev_quick_items = quick_items;

                    let draw_list = ui.get_window_draw_list();
                    if self.display_settings.shown {
                        crate::rendering::wheel_renderer::render_wheel_preview(display_items, ui, &draw_list, self.preview_controller_style);
                    } else if wheel_type != WheelType::None {
                        let highlight = render_wheel(display_items, ui, &draw_list, stick, self.controller_highlight);
                        if let Some(visible) = controller_wheel {
                            self.controller_highlight = highlight;
                            if let Some(slot) = highlight {
                                controller::highlight(visible, slot);
                            }
                        }
                    } else {
                        crate::rendering::debug_renderer::draw_debug(ui, &draw_list);
                    }
                });

            let draw_list = ui.get_foreground_draw_list();
            if self.display_settings.shown {
                let _panel_font = ui.push_font(self.panel_font.into());
                self.display_settings.draw(ui);
            }
            draw_popups(ui, &draw_list);

            self.prev_type = wheel_type;
            self.prev_controller_wheel = controller_wheel;
            font.pop();
            #[cfg(feature = "settings-diagnostics")]
            if hudhook::settings_diagnostics::sampled() {
                hudhook::settings_diagnostics::event(format_args!("render_loop_end listener={:x} shown={}",
                    self.display_settings.shortcut.diagnostic_id(), self.display_settings.shown));
            }
        );
    }
}

const DEFAULT_SCREEN_MIN: f32 = 2160.0;

pub struct ItemWheelData {
    pub spells: Vec<Item>,
    pub quick_items: Vec<Item>,
    pub wheel_type: WheelType,
    pub controller_wheel: Option<VisibleWheel>,
}

impl ItemWheelData {
    pub fn new() -> Self {
        Self {
            spells: vec![],
            quick_items: vec![],
            wheel_type: WheelType::None,
            controller_wheel: None,
        }
    }

    pub fn mutate<F: FnOnce(&mut Self)>(f: F) {
        f(&mut ITEM_WHEEL_DATA.write().expect("Couldn't write to ITEM_WHEEL_DATA"))
    }

    pub fn get<F: FnOnce(&Self) -> T, T>(f: F) -> T {
        f(&ITEM_WHEEL_DATA.read().expect("Couldn't read from ITEM_WHEEL_DATA"))
    }
}
