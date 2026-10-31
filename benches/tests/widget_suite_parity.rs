// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::widget_suite::{CASES, Case, Parity, SIZES, render_both};
use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid};
use ratatui::buffer::Buffer;
use ratatui::style::{Color as RatColor, Modifier};

fn lunatui_color(color: RatColor) -> Color {
    match color {
        RatColor::Reset => Color::Default,
        RatColor::Black => Color::Indexed(0),
        RatColor::Red => Color::Indexed(1),
        RatColor::Green => Color::Indexed(2),
        RatColor::Yellow => Color::Indexed(3),
        RatColor::Blue => Color::Indexed(4),
        RatColor::Magenta => Color::Indexed(5),
        RatColor::Cyan => Color::Indexed(6),
        RatColor::Gray => Color::Indexed(7),
        RatColor::DarkGray => Color::Indexed(8),
        RatColor::LightRed => Color::Indexed(9),
        RatColor::LightGreen => Color::Indexed(10),
        RatColor::LightYellow => Color::Indexed(11),
        RatColor::LightBlue => Color::Indexed(12),
        RatColor::LightMagenta => Color::Indexed(13),
        RatColor::LightCyan => Color::Indexed(14),
        RatColor::White => Color::Indexed(15),
        RatColor::Indexed(index) => Color::Indexed(index),
        RatColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
    }
}

fn differences(
    grid: &Grid,
    clusters: &ClusterTable,
    buffer: &Buffer,
    width: u16,
    height: u16,
) -> Vec<String> {
    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let want = &buffer[(x, y)];
            let Some(cell) = grid.get(x, y) else {
                out.push(format!("({x},{y}) missing cell"));
                continue;
            };
            let symbol = match cell.glyph().decode() {
                DecodedGlyph::Inline(scalar) => scalar.to_string(),
                DecodedGlyph::Cluster(id) => clusters
                    .cluster_text(id)
                    .map(str::to_owned)
                    .unwrap_or_default(),
                DecodedGlyph::Continuation => String::new(),
            };
            let style = cell.style();
            let got = (
                symbol,
                style.fg,
                style.bg,
                style.attrs.bold(),
                style.attrs.dim(),
            );
            let expected = (
                want.symbol().to_owned(),
                lunatui_color(want.fg),
                lunatui_color(want.bg),
                want.modifier.contains(Modifier::BOLD),
                want.modifier.contains(Modifier::DIM),
            );
            if got != expected {
                out.push(format!("({x},{y}) lunatui={got:?} ratatui={expected:?}"));
            }
        }
    }
    out
}

fn check(case: &'static Case, width: u16, height: u16, tick: u32) -> Result<(), String> {
    let (grid, clusters, buffer) = render_both(case, width, height, tick);
    let diffs = differences(&grid, &clusters, &buffer, width, height);
    if diffs.is_empty() {
        return Ok(());
    }
    let shown: Vec<_> = diffs.iter().take(3).collect();
    Err(format!(
        "{} {width}x{height} tick {tick}: {} differing cells, first: {shown:#?}",
        case.name,
        diffs.len()
    ))
}

#[test]
fn exact_cases_match_ratatui_cell_for_cell() {
    let mut failures = Vec::new();
    for case in CASES {
        if case.parity != Parity::Exact {
            continue;
        }
        for (width, height) in SIZES {
            for tick in [0, 1, 2, 3, 7, 13, 49, 99] {
                if let Err(message) = check(case, width, height, tick) {
                    failures.push(message);
                    break;
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn documented_cases_state_a_reason() {
    for case in CASES {
        if let Parity::Documented(reason) = case.parity {
            assert!(!reason.is_empty(), "{} has an empty reason", case.name);
        }
    }
}

#[test]
fn heatmap_matches_its_independent_buffer_reference() -> Result<(), String> {
    let case = CASES
        .iter()
        .find(|case| case.name == "heatmap")
        .ok_or("missing heatmap case")?;
    for (width, height) in SIZES {
        for tick in [0, 1, 30, 75, 150, 299] {
            check(case, width, height, tick)?;
        }
    }
    Ok(())
}

#[test]
fn every_case_has_a_unique_name() {
    let mut names: Vec<_> = CASES.iter().map(|case| case.name).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len());
}
