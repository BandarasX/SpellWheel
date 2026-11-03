use std::collections::{HashMap, HashSet};
use std::{fs, mem};
use std::sync::{Arc, OnceLock, RwLock};
use imgui::{FontConfig, FontGlyphRanges, FontSource};
use lazy_static::lazy_static;
use crate::debugging::is_debugging;
use crate::glyphs::{modded_reader, vanilla_reader};
use crate::glyphs::builder::build_glyph_ranges;
use crate::paths;
use crate::settings::Settings;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontDirection {
    Ltr,
    Rtl,
}
pub const FONT_LATIN_BYTES: &[u8] = include_bytes!("../../assets/font_latin.ttf");
pub type FontEntry = (&'static str, &'static str, FontDirection);
pub const OTHER_FONTS: [FontEntry; 7] = [
    ("arabic.ttf", "araae", FontDirection::Rtl),
    ("cyrillic.ttf", "rusru", FontDirection::Ltr),
    ("japanese.ttf", "jpnjp", FontDirection::Ltr),
    ("korean.ttf", "korkr", FontDirection::Ltr),
    ("simplified_chinese.ttf", "zhocn", FontDirection::Ltr),
    ("traditional_chinese.ttf", "zhotw", FontDirection::Ltr),
    ("thai.ttf", "thath", FontDirection::Ltr),
];
pub const DEFAULT_FONT_HEIGHT: f32 = 54.0;

lazy_static!(
    static ref FONT_MANAGER: OnceLock<Arc<RwLock<FontManager>>> = OnceLock::new();
);

pub struct FontData {
    pub font_bytes: Vec<u8>,
    pub msg_char_ranges: Vec<u32>,
    pub direction: FontDirection,
}

impl FontData {
    pub fn new(font_bytes: Vec<u8>, msg_char_ranges: Vec<u32>, direction: FontDirection) -> Self {
        Self {
            font_bytes,
            msg_char_ranges,
            direction,
        }
    }
}

pub struct FontManager {
    font_data: HashMap<&'static str, FontData>,
}

impl FontManager {
    pub fn init() {
        if FONT_MANAGER.set(Arc::new(RwLock::new(Self::init_inner()))).is_ok() {
            tracing::info!("FontManager initialization finished");
        } else {
            tracing::error!("FontManager was already initialized");
        }
    }

    fn init_inner() -> Self {
        let mut chars = HashMap::new();

        for (font_file, msg_folder, direction) in OTHER_FONTS.iter() {
            let Ok(font_bytes) = fs::read(paths::spellwheel().join(font_file)) else {
                tracing::info!("Skipping {font_file}, not in spellwheel folder");
                continue;
            };

            let mut msg_chars = HashSet::new();
            if let Err(err) = vanilla_reader::read(msg_folder, &mut msg_chars) {
                tracing::error!("Error while loading vanilla MSG chars for {msg_folder}: {err}");
                continue;
            }
            if let Some(upstream_mod_folder) = modded_reader::search_for_mod_folder(msg_folder) &&
                let Err(err) = modded_reader::read(upstream_mod_folder, msg_folder, &mut msg_chars) {
                tracing::error!("Error while loading upstream modded MSG chars for {msg_folder}: {err}");
            }
            for modded_msg_path in Settings::read_or_default().mods() {
                if let Err(err) = modded_reader::read(modded_msg_path, msg_folder, &mut msg_chars) {
                    tracing::error!("Error while loading modded ({modded_msg_path}) MSG chars for {msg_folder}: {err}");
                    continue;
                }
            }

            let font_data = FontData::new(font_bytes, build_glyph_ranges(msg_chars.into_iter()), *direction);

            if is_debugging() {
                tracing::info!("Glyph ranges for {font_file}:\n{:?}", font_data.msg_char_ranges);
            }

            chars.insert(
                *font_file,
                font_data
            );
        }

        Self {
            font_data: chars,
        }
    }

    pub fn get_font_data() -> Option<&'static HashMap<&'static str, FontData>> {
        FONT_MANAGER.get().and_then(|fm| fm.read().ok())
            .map(|fd| unsafe { mem::transmute::<&_, &'static _>(&fd.font_data) })
    }

    pub fn generate_sources() -> Vec<FontSource<'static>> {
        match FONT_MANAGER.get() {
            Some(font_manager) => font_manager.read().expect("Couldn't read FONT_MANAGER").generate_sources_inner(),
            None => {
                tracing::warn!("FontManager not initialized");
                vec![
                    FontSource::TtfData {
                        data: FONT_LATIN_BYTES,
                        size_pixels: DEFAULT_FONT_HEIGHT,
                        config: Some(FontConfig {
                            glyph_ranges: FontGlyphRanges::default(),
                            oversample_h: 3,
                            oversample_v: 1,
                            pixel_snap_h: false,
                            ..FontConfig::default()
                        })
                    }
                ]
            }
        }
    }

    fn generate_sources_inner(&self) -> Vec<FontSource<'static>> {
        let mut font_sources: Vec<FontSource> = vec![
            FontSource::TtfData {
                data: FONT_LATIN_BYTES,
                size_pixels: DEFAULT_FONT_HEIGHT,
                config: Some(FontConfig {
                    glyph_ranges: FontGlyphRanges::default(),
                    oversample_h: 3,
                    oversample_v: 1,
                    pixel_snap_h: false,
                    ..FontConfig::default()
                })
            }
        ];

        for (font_name, font_data) in &self.font_data {
            tracing::info!("Adding font {font_name}");
            font_sources.push(
                FontSource::TtfData {
                    data: unsafe { mem::transmute(font_data.font_bytes.as_slice()) },
                    size_pixels: DEFAULT_FONT_HEIGHT,
                    config: Some(FontConfig {
                        glyph_ranges: FontGlyphRanges::from_slice(unsafe { mem::transmute(font_data.msg_char_ranges.as_slice()) }),
                        oversample_h: 3,
                        oversample_v: 1,
                        pixel_snap_h: false,
                        ..FontConfig::default()
                    })
                }
            );
        }

        font_sources
    }
}