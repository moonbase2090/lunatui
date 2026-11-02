// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_widgets::graphite::{Face, FontRun, HostFontMetrics, HostTextMetrics, TextMetrics};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Font {
    Text,
    Bold,
    Mono,
    Symbols,
    Emoji,
    Backup,
}

struct Fonts {
    symbols: &'static [Font],
}

impl HostFontMetrics for Fonts {
    type Font = Font;

    fn primary_font(&self, face: Face) -> Font {
        match face {
            Face::Regular => Font::Text,
            Face::SemiBold => Font::Bold,
            Face::Mono => Font::Mono,
        }
    }

    fn symbol_fonts(&self) -> &[Font] {
        self.symbols
    }

    fn supports_grapheme(&self, font: Font, grapheme: &str) -> bool {
        match font {
            Font::Text | Font::Bold | Font::Mono => grapheme.is_ascii() || grapheme == "e\u{301}",
            Font::Symbols => matches!(grapheme, "☃" | "♥" | "A"),
            Font::Emoji => matches!(grapheme, "♥\u{fe0f}" | "👩\u{200d}💻"),
            Font::Backup => matches!(grapheme, "☃" | "♥" | "a\u{20dd}"),
        }
    }

    fn width(&self, font: Font, px: f32, text: &str) -> f32 {
        let advance = if font == Font::Text && text == "AV" {
            13.0
        } else {
            text.graphemes(true)
                .map(|grapheme| {
                    if !self.supports_grapheme(font, grapheme) {
                        return 5.0;
                    }
                    match font {
                        Font::Text => 7.0,
                        Font::Bold => 8.0,
                        Font::Mono => 9.0,
                        Font::Symbols => 11.0,
                        Font::Emoji => 20.0,
                        Font::Backup => 17.0,
                    }
                })
                .sum()
        };
        advance * px / 12.0
    }
}

const FONTS: Fonts = Fonts {
    symbols: &[Font::Symbols, Font::Emoji, Font::Backup],
};

#[test]
fn selects_primary_then_first_covering_fallback_and_measures_the_runs() {
    let metrics = HostTextMetrics::new(&FONTS);
    let runs: Vec<_> = metrics.runs(Face::Regular, "AV☃♥Z").collect();
    assert_eq!(
        runs,
        [
            FontRun {
                font: Font::Text,
                text: "AV"
            },
            FontRun {
                font: Font::Symbols,
                text: "☃♥"
            },
            FontRun {
                font: Font::Text,
                text: "Z"
            },
        ]
    );
    assert_eq!(metrics.width(Face::Regular, 12.0, "AV☃♥Z"), 42.0);
}

#[test]
fn uses_host_fallback_order_and_skips_fonts_without_coverage() {
    let fonts = Fonts {
        symbols: &[Font::Emoji, Font::Backup, Font::Symbols],
    };
    let metrics = HostTextMetrics::new(&fonts);
    assert_eq!(
        metrics.runs(Face::Regular, "☃").collect::<Vec<_>>(),
        [FontRun {
            font: Font::Backup,
            text: "☃"
        }]
    );
    assert_eq!(metrics.width(Face::Regular, 12.0, "☃"), 17.0);
}

#[test]
fn preserves_combining_marks_variation_selectors_and_joined_emoji() {
    let metrics = HostTextMetrics::new(&FONTS);
    assert_eq!(
        metrics
            .runs(Face::Regular, "e\u{301}a\u{20dd}♥\u{fe0f}👩\u{200d}💻")
            .collect::<Vec<_>>(),
        [
            FontRun {
                font: Font::Text,
                text: "e\u{301}"
            },
            FontRun {
                font: Font::Backup,
                text: "a\u{20dd}"
            },
            FontRun {
                font: Font::Emoji,
                text: "♥\u{fe0f}👩\u{200d}💻"
            },
        ]
    );
    assert_eq!(
        metrics.width(
            Face::Regular,
            12.0,
            "e\u{301}a\u{20dd}♥\u{fe0f}👩\u{200d}💻"
        ),
        64.0
    );
}

#[test]
fn uncovered_clusters_keep_the_primary_missing_glyph_and_advance() {
    let metrics = HostTextMetrics::new(&FONTS);
    assert_eq!(
        metrics.runs(Face::SemiBold, "A界Z").collect::<Vec<_>>(),
        [FontRun {
            font: Font::Bold,
            text: "A界Z"
        }]
    );
    assert_eq!(metrics.width(Face::SemiBold, 12.0, "A界Z"), 21.0);
}

#[test]
fn an_empty_fallback_list_uses_only_the_primary_font() {
    let fonts = Fonts { symbols: &[] };
    let metrics = HostTextMetrics::new(&fonts);
    assert_eq!(
        metrics.runs(Face::Regular, "A☃Z").collect::<Vec<_>>(),
        [FontRun {
            font: Font::Text,
            text: "A☃Z"
        }]
    );
    assert_eq!(metrics.width(Face::Regular, 12.0, "A☃Z"), 19.0);
}

#[test]
fn passes_face_and_pixel_size_to_the_host() {
    let metrics = HostTextMetrics::new(&FONTS);
    assert_eq!(metrics.width(Face::Regular, 24.0, "A☃"), 36.0);
    assert_eq!(metrics.width(Face::SemiBold, 24.0, "A☃"), 38.0);
    assert_eq!(metrics.width(Face::Mono, 24.0, "A☃"), 40.0);
    assert_eq!(
        metrics.runs(Face::Mono, "A☃").collect::<Vec<_>>(),
        [
            FontRun {
                font: Font::Mono,
                text: "A"
            },
            FontRun {
                font: Font::Symbols,
                text: "☃"
            },
        ]
    );
}

#[test]
fn empty_text_has_no_runs_or_width() {
    let metrics = HostTextMetrics::new(&FONTS);
    assert_eq!(metrics.runs(Face::Regular, "").next(), None);
    assert_eq!(metrics.width(Face::Regular, 12.0, ""), 0.0);
}

#[test]
fn accepts_a_borrowed_host_trait_object() {
    let host: &dyn HostFontMetrics<Font = Font> = &FONTS;
    let metrics = HostTextMetrics::new(host);
    assert_eq!(metrics.width(Face::Regular, 12.0, "AV☃"), 24.0);
}
