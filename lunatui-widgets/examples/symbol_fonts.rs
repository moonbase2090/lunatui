// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_widgets::graphite::{Face, HostFontMetrics, HostTextMetrics, TextMetrics};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Font {
    Text,
    Symbols,
}

struct DemoHost;

impl HostFontMetrics for DemoHost {
    type Font = Font;

    fn primary_font(&self, _face: Face) -> Font {
        Font::Text
    }

    fn symbol_fonts(&self) -> &[Font] {
        &[Font::Symbols]
    }

    fn supports_grapheme(&self, font: Font, grapheme: &str) -> bool {
        match font {
            Font::Text => grapheme.is_ascii(),
            Font::Symbols => grapheme == "☃",
        }
    }

    fn width(&self, font: Font, px: f32, text: &str) -> f32 {
        text.graphemes(true)
            .map(|grapheme| {
                let advance = match (font, grapheme) {
                    (Font::Symbols, "☃") => 12.0,
                    _ => 8.0,
                };
                advance * px / 12.0
            })
            .sum()
    }
}

fn main() {
    let metrics = HostTextMetrics::new(&DemoHost);
    let text = "Build ☃";
    for run in metrics.runs(Face::Regular, text) {
        println!("{:?}: {:?}", run.font, run.text);
    }
    println!(
        "Width at 12 px: {}",
        metrics.width(Face::Regular, 12.0, text)
    );
}
