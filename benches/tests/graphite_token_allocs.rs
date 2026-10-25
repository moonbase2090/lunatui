// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_core::Color;
use lunatui_theme::{Theme, Token, Variant};
use std::hint::black_box;

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn measured_token_resolution_returns_graphite_colors_without_allocations() {
    let theme = Theme::builtin(Variant::Dark);
    let (colors, stats) = ALLOC.measure(|| {
        [
            black_box(&theme).resolve(black_box(Token::Border)),
            black_box(&theme).resolve(black_box(Token::TextMuted)),
            black_box(&theme).resolve(black_box(Token::Warn)),
            black_box(&theme).resolve(black_box(Token::Fail)),
        ]
    });
    assert_eq!(
        colors,
        [
            Color::Rgb(38, 42, 50),
            Color::Rgb(163, 171, 184),
            Color::Rgb(242, 184, 75),
            Color::Rgb(255, 122, 107)
        ]
    );
    assert_eq!(stats.allocations, 0);
}
