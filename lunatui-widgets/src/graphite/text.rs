// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use core::fmt;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use fontdue::{Font, FontSettings, Metrics};
use lunatui_fonts::{Family, Weight};

use super::{Face, TextMetrics};

const CACHE_BYTES: usize = 4 * 1024 * 1024;
const MAX_SIZE: f32 = 4096.0;
const MAX_ORIGIN: f32 = 1_048_576.0;
const SLOTS: usize = 6;
const UI_REGULAR: usize = 0;
const UI_BOLD: usize = 1;
const MONO: usize = 2;
const NOTO_REGULAR: usize = 3;
const NOTO_BOLD: usize = 4;
const EMOJI: usize = 5;

/// Proportional font that [`BundledText`] uses for [`Face::Regular`] and [`Face::SemiBold`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UiFont {
    /// Inter, the default.
    #[default]
    Inter,
    /// IBM Plex Sans.
    IbmPlexSans,
    /// Noto Sans.
    NotoSans,
    /// Inter with JetBrains Mono for [`Face::Mono`], from their compact files
    /// ([`lunatui_fonts::Family::compact`]), and no fallback fonts.
    ///
    /// The compact files keep Latin, punctuation, arrows, symbols, box drawing and shapes
    /// ([`lunatui_fonts::COMPACT_RANGES`]) and parse several times faster than the full set, so
    /// the first text draws quickly on a tight CPU budget, such as a plug-in in a sandbox. Any
    /// other character is drawn as the missing-glyph box instead of from a fallback font.
    InterCompact,
}

impl UiFont {
    const fn family(self) -> Family {
        match self {
            Self::Inter | Self::InterCompact => Family::Inter,
            Self::IbmPlexSans => Family::IbmPlexSans,
            Self::NotoSans => Family::NotoSans,
        }
    }
}

/// Vertical metrics of a face at one size, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LineMetrics {
    /// Distance from the baseline up to the top of the tallest glyphs.
    pub ascent: f32,
    /// Distance from the baseline down to the bottom of the lowest glyphs, as a positive number.
    pub descent: f32,
    /// Suggested gap between one line's descent and the next line's ascent.
    pub line_gap: f32,
}

/// Measures and draws text with the fonts bundled in `lunatui-fonts`.
///
/// Each character is drawn with the first font that has it: the face's font, then Noto Sans,
/// then JetBrains Mono for arrows, box drawing and block elements, then Noto Emoji. A character
/// none of them has is drawn as the face font's missing-glyph box.
/// [`Face::SemiBold`] uses the bold files and [`Face::Mono`] uses JetBrains Mono. Invisible
/// formatting characters, such as variation selectors, joiners and control characters, take no
/// space. Text is neither kerned nor shaped: each character advances by its own width, and
/// scripts that need shaping, such as Arabic, are drawn letter by letter.
///
/// Sizes are in pixels and must be finite, above 0 and at most 4096.
///
/// With [`UiFont::InterCompact`], text uses the compact Inter and JetBrains Mono files and has no
/// fallback fonts.
///
/// Fonts are parsed the first time a character needs them, or all at once by
/// [`BundledText::load`]. Glyph images are cached, up to 4 MiB in all; a single glyph image larger
/// than that is drawn without being cached.
pub struct BundledText {
    ui: UiFont,
    fonts: [OnceLock<Option<Font>>; SLOTS],
    glyphs: Mutex<GlyphCache>,
}

static INTER: BundledText = BundledText::new(UiFont::Inter);
static IBM_PLEX_SANS: BundledText = BundledText::new(UiFont::IbmPlexSans);
static NOTO_SANS: BundledText = BundledText::new(UiFont::NotoSans);
static INTER_COMPACT: BundledText = BundledText::new(UiFont::InterCompact);

type GlyphKey = (usize, u16, u32);

struct GlyphCache {
    bytes: usize,
    entries: BTreeMap<GlyphKey, (Metrics, Box<[u8]>)>,
}

pub(crate) struct Glyph<'g> {
    pub(crate) left: i64,
    pub(crate) top: i64,
    pub(crate) width: usize,
    pub(crate) coverage: &'g [u8],
}

impl fmt::Debug for BundledText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BundledText")
            .field("ui", &self.ui)
            .finish_non_exhaustive()
    }
}

impl BundledText {
    /// Creates bundled text with `ui` as the proportional font.
    ///
    /// Each value parses its own copy of the fonts; [`BundledText::shared`] avoids that.
    #[must_use]
    pub const fn new(ui: UiFont) -> Self {
        Self {
            ui,
            fonts: [const { OnceLock::new() }; SLOTS],
            glyphs: Mutex::new(GlyphCache {
                bytes: 0,
                entries: BTreeMap::new(),
            }),
        }
    }

    /// The process-wide bundled text for `ui`.
    #[must_use]
    pub fn shared(ui: UiFont) -> &'static Self {
        match ui {
            UiFont::Inter => &INTER,
            UiFont::IbmPlexSans => &IBM_PLEX_SANS,
            UiFont::NotoSans => &NOTO_SANS,
            UiFont::InterCompact => &INTER_COMPACT,
        }
    }

    /// Parses every font file this text can draw with that is not parsed yet, so later text
    /// draws without that cost.
    ///
    /// Parsing the full set takes tens of milliseconds natively; [`UiFont::InterCompact`] takes a
    /// few. A program on a per-call CPU budget can draw its first frames without this text and
    /// call `load` in a later call.
    pub fn load(&self) {
        for slot in 0..SLOTS {
            let _ = self.font(slot);
        }
    }

    /// Whether every font file this text can draw with is parsed, by [`BundledText::load`] or by
    /// text that needed it.
    #[must_use]
    pub fn loaded(&self) -> bool {
        self.fonts
            .iter()
            .enumerate()
            .all(|(slot, font)| font.get().is_some() || self.file(slot).is_none())
    }

    /// The proportional font.
    #[must_use]
    pub const fn ui_font(&self) -> UiFont {
        self.ui
    }

    /// Vertical metrics of `face` at `px`, or `None` when `px` is not finite, above 0 and at most
    /// 4096.
    #[must_use]
    pub fn line_metrics(&self, face: Face, px: f32) -> Option<LineMetrics> {
        if !valid_size(px) {
            return None;
        }
        let font = self.font(*chain(face).first()?)?;
        let metrics = font.horizontal_line_metrics(px)?;
        Some(LineMetrics {
            ascent: metrics.ascent,
            descent: -metrics.descent,
            line_gap: metrics.line_gap,
        })
    }

    pub(crate) fn draw(
        &self,
        face: Face,
        px: f32,
        origin: (f32, f32),
        text: &str,
        mut blit: impl FnMut(Glyph<'_>),
    ) -> bool {
        if !(valid_size(px) && within_reach(origin.0) && within_reach(origin.1)) {
            return false;
        }
        let mut cache = self.glyphs.lock().unwrap_or_else(PoisonError::into_inner);
        let baseline = round(origin.1);
        let mut drew = false;
        self.layout(face, px, text, |slot, font, index, pen| {
            let left = origin.0 + pen;
            if !within_reach(left) {
                return;
            }
            let key = (slot, index, px.to_bits());
            cache.with(
                key,
                || font.rasterize_indexed(index, px),
                |metrics, coverage| {
                    if metrics.width == 0 || metrics.height == 0 {
                        return;
                    }
                    let height = i64::try_from(metrics.height).unwrap_or(i64::MAX);
                    blit(Glyph {
                        left: round(left).saturating_add(i64::from(metrics.xmin)),
                        top: baseline
                            .saturating_sub(height)
                            .saturating_sub(i64::from(metrics.ymin)),
                        width: metrics.width,
                        coverage,
                    });
                    drew = true;
                },
            );
        });
        drew
    }

    fn layout<'s>(
        &'s self,
        face: Face,
        px: f32,
        text: &str,
        mut each: impl FnMut(usize, &'s Font, u16, f32),
    ) -> f32 {
        let fonts = chain(face);
        let mut pen = 0.0;
        for character in text.chars() {
            if invisible(character) {
                continue;
            }
            let Some((slot, font, index)) = self.pick(fonts, character) else {
                continue;
            };
            each(slot, font, index, pen);
            pen += font.metrics_indexed(index, px).advance_width;
        }
        pen
    }

    fn pick(&self, fonts: &[usize], character: char) -> Option<(usize, &Font, u16)> {
        let found = fonts.iter().find_map(|&slot| {
            let font = self.font(slot)?;
            let index = font.lookup_glyph_index(character);
            (index != 0).then_some((slot, font, index))
        });
        found.or_else(|| {
            let first = *fonts.first()?;
            Some((first, self.font(first)?, 0))
        })
    }

    fn font(&self, slot: usize) -> Option<&Font> {
        let file = self.file(slot)?;
        self.fonts
            .get(slot)?
            .get_or_init(|| Font::from_bytes(file, FontSettings::default()).ok())
            .as_ref()
    }

    fn file(&self, slot: usize) -> Option<&'static [u8]> {
        let (family, weight) = match slot {
            UI_REGULAR => (self.ui.family(), Weight::Regular),
            UI_BOLD => (self.ui.family(), Weight::Bold),
            MONO => (Family::JetBrainsMono, Weight::Regular),
            NOTO_REGULAR => (Family::NotoSans, Weight::Regular),
            NOTO_BOLD => (Family::NotoSans, Weight::Bold),
            EMOJI => (Family::NotoEmoji, Weight::Regular),
            _ => return None,
        };
        if self.ui == UiFont::InterCompact {
            return match slot {
                UI_REGULAR | UI_BOLD | MONO => family.compact(weight),
                _ => None,
            };
        }
        Some(family.data(weight))
    }
}

impl TextMetrics for BundledText {
    fn width(&self, face: Face, px: f32, text: &str) -> f32 {
        if !valid_size(px) {
            return 0.0;
        }
        self.layout(face, px, text, |_, _, _, _| {})
    }
}

impl GlyphCache {
    fn with<R>(
        &mut self,
        key: GlyphKey,
        rasterize: impl FnOnce() -> (Metrics, Vec<u8>),
        use_glyph: impl FnOnce(Metrics, &[u8]) -> R,
    ) -> R {
        if let Some((metrics, coverage)) = self.entries.get(&key) {
            return use_glyph(*metrics, coverage);
        }
        let (metrics, coverage) = rasterize();
        if coverage.len() > CACHE_BYTES {
            return use_glyph(metrics, &coverage);
        }
        if self.bytes + coverage.len() > CACHE_BYTES {
            self.entries.clear();
            self.bytes = 0;
        }
        self.bytes += coverage.len();
        let result = use_glyph(metrics, &coverage);
        self.entries
            .insert(key, (metrics, coverage.into_boxed_slice()));
        result
    }
}

const fn chain(face: Face) -> &'static [usize] {
    match face {
        Face::SemiBold => &[UI_BOLD, NOTO_BOLD, MONO, EMOJI],
        Face::Mono => &[MONO, NOTO_REGULAR, EMOJI],
        Face::Regular => &[UI_REGULAR, NOTO_REGULAR, MONO, EMOJI],
    }
}

fn valid_size(px: f32) -> bool {
    px.is_finite() && px > 0.0 && px <= MAX_SIZE
}

fn within_reach(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_ORIGIN
}

fn invisible(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{00AD}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{FE00}'..='\u{FE0F}'
                | '\u{FEFF}'
                | '\u{E0020}'..='\u{E007F}'
                | '\u{E0100}'..='\u{E01EF}'
        )
}

fn round(value: f32) -> i64 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "inputs are finite and the cast saturates"
    )]
    let rounded = value.round() as i64;
    rounded
}

#[cfg(test)]
mod tests {
    use fontdue::Metrics;

    use super::{CACHE_BYTES, GlyphCache};

    fn cache() -> GlyphCache {
        GlyphCache {
            bytes: 0,
            entries: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn a_glyph_larger_than_the_cap_is_used_but_not_kept() {
        let mut glyphs = cache();
        let seen = glyphs.with(
            (0, 1, 0),
            || (Metrics::default(), vec![7; CACHE_BYTES + 1]),
            |_, coverage| coverage.len(),
        );
        assert_eq!(seen, CACHE_BYTES + 1);
        assert_eq!(glyphs.bytes, 0);
        assert!(glyphs.entries.is_empty());
    }

    #[test]
    fn the_cache_never_holds_more_than_the_cap() {
        let mut glyphs = cache();
        let size = CACHE_BYTES / 3 + 1;
        for index in 0..7 {
            glyphs.with(
                (0, index, 0),
                || (Metrics::default(), vec![1; size]),
                |_, _| (),
            );
            let held: usize = glyphs.entries.values().map(|(_, bytes)| bytes.len()).sum();
            assert_eq!(held, glyphs.bytes);
            assert!(glyphs.bytes <= CACHE_BYTES, "{}", glyphs.bytes);
        }
        assert_eq!(glyphs.entries.len(), 1);
    }

    #[test]
    fn a_cached_glyph_is_not_rasterized_again() {
        let mut glyphs = cache();
        glyphs.with((0, 1, 0), || (Metrics::default(), vec![9; 4]), |_, _| ());
        let again = glyphs.with(
            (0, 1, 0),
            || (Metrics::default(), vec![0; 2]),
            |_, coverage| coverage.to_vec(),
        );
        assert_eq!(again, vec![9; 4]);
    }
}
