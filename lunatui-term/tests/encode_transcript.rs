// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Style};
use lunatui_term::{ColorMode, Encoder};

#[test]
fn empty_to_two_ascii_cells_matches_transcript() {
    let mut clusters = ClusterTable::new();
    let prev = Grid::new(2, 1);
    let mut next = Grid::new(2, 1);
    next.put_str(0, 0, Style::new(), "AB", &mut clusters);
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::NoColor);
    assert_eq!(bytes, b"\x1b[1;1HAB\x1b[m\x1b[m\x1b[m\x1b[0m");
}

#[test]
fn style_change_emits_sgr_before_glyph() {
    let mut clusters = ClusterTable::new();
    let prev = Grid::new(1, 1);
    let mut next = Grid::new(1, 1);
    next.put_str(
        0,
        0,
        Style::new().fg(lunatui_core::Color::Indexed(1)),
        "X",
        &mut clusters,
    );
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
    assert!(bytes.starts_with(b"\x1b["));
    assert!(bytes.ends_with(b"X\x1b[m\x1b[m\x1b[m\x1b[0m"));
}

#[test]
fn ascii_run_with_style_changes_and_wide_cell_matches_transcript() {
    let mut clusters = ClusterTable::new();
    let prev = Grid::new(8, 1);
    let mut next = Grid::new(8, 1);
    let red = Style::new().fg(lunatui_core::Color::Indexed(1));
    next.put_str(0, 0, Style::new(), "ab", &mut clusters);
    next.put_str(2, 0, red, "cd", &mut clusters);
    next.put_str(4, 0, red, "\u{65e5}", &mut clusters);
    next.put_str(6, 0, red, "ef", &mut clusters);
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
    assert_eq!(
        bytes,
        "\x1b[1;1Hab\x1b[0;38;5;1;49mcd\u{65e5}ef\x1b[m\x1b[m\x1b[m\x1b[0m".as_bytes()
    );
}
