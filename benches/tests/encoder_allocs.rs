// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn paint(grid: &mut Grid, clusters: &mut ClusterTable, phase: u16) {
    let base = Style::new().bg(Color::Rgb(26, 27, 38));
    grid.fill(Rect::new(0, 0, 60, 12), base, ' ', clusters);
    for row in 0..12_u16 {
        let shade = u8::try_from((row * 20 + phase * 7) % 256).unwrap_or(0);
        let style = base.fg(Color::Rgb(shade, 200, 255 - shade));
        grid.put_str(2 + phase % 5, row, style, "diff and encode", clusters);
        let indexed = base.fg(Color::Indexed(u8::try_from(row + phase).unwrap_or(0)));
        grid.put_str(30, row, indexed, "indexed", clusters);
    }
    let bold = base.fg(Color::Indexed(1));
    grid.put_str(40, 3, bold, "bold", clusters);
    grid.set_style(
        Rect::new(40, 3, 4, 1),
        bold.with_attrs(Attrs::default().set_bold(true)),
    );
    grid.put_str(1, 11, base.fg(Color::Default), "│", clusters);
}

#[test]
fn steady_state_encoding_allocates_nothing() {
    let mut clusters = ClusterTable::new();
    let mut previous = Grid::new(60, 12);
    let mut next = Grid::new(60, 12);
    let mut encoder = Encoder::new();
    for phase in 0..40 {
        paint(&mut next, &mut clusters, phase);
        let _ = encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor);
        std::mem::swap(&mut previous, &mut next);
    }
    let mut allocations = 0;
    let mut bytes = 0;
    for phase in 40..80 {
        paint(&mut next, &mut clusters, phase);
        let (len, stats) = ALLOC.measure(|| {
            encoder
                .encode_diff(&previous, &next, &clusters, ColorMode::TrueColor)
                .len()
        });
        allocations += stats.allocations;
        bytes += len;
        std::mem::swap(&mut previous, &mut next);
    }
    assert!(bytes > 0, "the frames produced no output");
    assert_eq!(allocations, 0, "encode_diff allocated in steady state");
}
