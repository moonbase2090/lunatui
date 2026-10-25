// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use crate::Variant;
use lunatui_core::Color;

pub(crate) fn colors(variant: Variant) -> [Color; 20] {
    match variant {
        Variant::Dark => [
            Color::Rgb(16, 18, 22),
            Color::Rgb(24, 27, 33),
            Color::Rgb(230, 233, 238),
            Color::Rgb(98, 168, 255),
            Color::Rgb(76, 201, 138),
            Color::Rgb(242, 184, 75),
            Color::Rgb(255, 122, 107),
            Color::Rgb(98, 168, 255),
            Color::Rgb(38, 42, 50),
            Color::Rgb(163, 171, 184),
            Color::Rgb(242, 244, 247),
            Color::Rgb(37, 42, 51),
            Color::Rgb(34, 38, 45),
            Color::Rgb(61, 95, 143),
            Color::Rgb(107, 115, 128),
            Color::Rgb(28, 35, 48),
            Color::Rgb(42, 49, 64),
            Color::Rgb(180, 188, 201),
            Color::Rgb(26, 13, 11),
            Color::Rgb(214, 232, 255),
        ],
        Variant::Light => [
            Color::Rgb(233, 236, 240),
            Color::Rgb(255, 255, 255),
            Color::Rgb(31, 35, 41),
            Color::Rgb(79, 137, 209),
            Color::Rgb(31, 138, 85),
            Color::Rgb(184, 122, 10),
            Color::Rgb(194, 57, 43),
            Color::Rgb(79, 137, 209),
            Color::Rgb(213, 217, 223),
            Color::Rgb(91, 100, 114),
            Color::Rgb(31, 35, 41),
            Color::Rgb(226, 230, 235),
            Color::Rgb(227, 230, 234),
            Color::Rgb(157, 187, 232),
            Color::Rgb(138, 146, 158),
            Color::Rgb(234, 241, 252),
            Color::Rgb(205, 220, 243),
            Color::Rgb(74, 82, 94),
            Color::Rgb(255, 255, 255),
            Color::Rgb(22, 63, 128),
        ],
    }
}

#[cfg(feature = "color-profiles")]
pub(crate) fn ansi16(variant: Variant) -> [Color; 20] {
    let (ground, panel, text, strong, hover, danger_text, head) = match variant {
        Variant::Dark => (0, 0, 7, 15, 8, 0, 15),
        Variant::Light => (7, 15, 0, 0, 7, 7, 4),
    };
    [
        ground,
        panel,
        text,
        4,
        2,
        3,
        1,
        4,
        8,
        8,
        strong,
        hover,
        8,
        4,
        8,
        4,
        4,
        8,
        danger_text,
        head,
    ]
    .map(Color::Indexed)
}
