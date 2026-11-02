// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Host-owned fonts for the Graphite text metrics bridge.

use unicode_segmentation::UnicodeSegmentation;

use super::{Face, TextMetrics};

/// Font selection and pixel metrics supplied by the drawing host.
///
/// The host owns font discovery, loading, shaping, and handle lifetimes. Coverage
/// and metrics must describe the same fonts used to draw [`FontRun`] values.
/// Keep the font set stable between layout and drawing.
pub trait HostFontMetrics {
    /// A host-owned font handle. Equal handles identify the same font.
    type Font: Copy + Eq;

    /// Returns the primary font for the requested text face.
    fn primary_font(&self, face: Face) -> Self::Font;

    /// Returns symbol fallback fonts in preference order.
    fn symbol_fonts(&self) -> &[Self::Font];

    /// Whether this font can draw the entire extended grapheme cluster.
    ///
    /// Check shaped cluster coverage, including combining marks, variation
    /// selectors, and joined emoji. A base character alone is insufficient.
    fn supports_grapheme(&self, font: Self::Font, grapheme: &str) -> bool;

    /// Measures a contiguous run in this font at the requested pixel size.
    ///
    /// Return a finite, nonnegative horizontal advance using the same shaping
    /// options as drawing. Unsupported clusters use this font's missing glyph.
    fn width(&self, font: Self::Font, px: f32, text: &str) -> f32;
}

/// A contiguous borrowed text run drawn with one host-owned font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontRun<'text, Font> {
    /// The selected primary or symbol font.
    pub font: Font,
    /// Text containing only complete extended grapheme clusters.
    pub text: &'text str,
}

/// Graphite-style pixel metrics with host-provided symbol font fallback.
///
/// Each grapheme uses the primary font when covered, otherwise the first
/// covering symbol font. If none covers it, the primary font draws its missing
/// glyph. Adjacent clusters with the same font form one run, preserving shaping
/// within that run. [`Self::runs`] exposes the same selection to the renderer.
/// No fonts are loaded and no text is allocated by this adapter.
pub struct HostTextMetrics<'host, H: HostFontMetrics + ?Sized> {
    host: &'host H,
}

impl<'host, H: HostFontMetrics + ?Sized> HostTextMetrics<'host, H> {
    /// Borrows the host's font set and metrics.
    #[must_use]
    pub const fn new(host: &'host H) -> Self {
        Self { host }
    }

    /// Selects fonts and groups adjacent clusters for measurement or drawing.
    ///
    /// Empty text produces no runs. Primary-font coverage always takes priority
    /// over the ordered symbol fonts. Run text borrows the input unchanged.
    pub fn runs<'text>(
        &'text self,
        face: Face,
        mut text: &'text str,
    ) -> impl Iterator<Item = FontRun<'text, H::Font>> + 'text {
        let primary = self.host.primary_font(face);
        std::iter::from_fn(move || {
            let mut graphemes = text.grapheme_indices(true);
            let (_, first) = graphemes.next()?;
            let font = self.select_font(primary, first);
            let end = graphemes
                .find_map(|(index, grapheme)| {
                    (self.select_font(primary, grapheme) != font).then_some(index)
                })
                .unwrap_or(text.len());
            let (run, rest) = text.split_at(end);
            text = rest;
            Some(FontRun { font, text: run })
        })
    }

    fn select_font(&self, primary: H::Font, grapheme: &str) -> H::Font {
        if self.host.supports_grapheme(primary, grapheme) {
            return primary;
        }
        self.host
            .symbol_fonts()
            .iter()
            .copied()
            .find(|&font| self.host.supports_grapheme(font, grapheme))
            .unwrap_or(primary)
    }
}

impl<H: HostFontMetrics + ?Sized> TextMetrics for HostTextMetrics<'_, H> {
    fn width(&self, face: Face, px: f32, text: &str) -> f32 {
        self.runs(face, text)
            .map(|run| self.host.width(run.font, px, run.text))
            .sum()
    }
}
