// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
    assert_eq!(bytes, b"AB\x1b[m\x1b[m\x1b[m\x1b[0m");
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
