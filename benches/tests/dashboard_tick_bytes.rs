// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::scenes::lunatui::dashboard as luna;
use lunatui_bench::scenes::ratatui::dashboard::Model;
use lunatui_term::{ColorMode, Encoder};

const WIDTH: u16 = 200;
const HEIGHT: u16 = 60;
const TARGET_BYTES: usize = 160;
const NO_COLOR_TICK_BYTES: usize = 157;
const TRUECOLOR_TICK_BYTES: usize = 382;

fn tick_bytes(colors: ColorMode) -> usize {
    let mut model = Model::new();
    let (first, _) = luna::render_new(&model, WIDTH, HEIGHT);
    model.tick += 1;
    let (tick, clusters) = luna::render_new(&model, WIDTH, HEIGHT);
    Encoder::new()
        .encode_diff(&first, &tick, &clusters, colors)
        .len()
}

#[test]
fn dashboard_tick_at_200x60_meets_the_byte_target_without_colour() {
    let bytes = tick_bytes(ColorMode::NoColor);
    assert_eq!(bytes, NO_COLOR_TICK_BYTES);
    assert!(
        bytes <= TARGET_BYTES,
        "{bytes} B is over the {TARGET_BYTES} B target"
    );
}

#[test]
fn dashboard_tick_at_200x60_has_a_known_truecolor_size() {
    assert_eq!(tick_bytes(ColorMode::TrueColor), TRUECOLOR_TICK_BYTES);
}
