// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::scenes::lunatui::dashboard as luna;
use lunatui_bench::scenes::ratatui::dashboard::{self as rat, Model, advance_scripted};
use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
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

fn differences(width: u16, height: u16, tick: u32) -> Vec<String> {
    let mut model = Model::new();
    advance_scripted(&mut model, tick);
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    rat::render(&model, area, &mut buffer);
    let (grid, clusters): (Grid, ClusterTable) = luna::render_new(&model, width, height);
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

fn assert_identical(width: u16, height: u16, tick: u32) {
    let diffs = differences(width, height, tick);
    let shown: Vec<_> = diffs.iter().take(25).collect();
    assert!(
        diffs.is_empty(),
        "{width}x{height} tick {tick}: {} differing cells, first: {shown:#?}",
        diffs.len()
    );
}

#[test]
fn dashboard_scenes_match_at_80x24() {
    for tick in (0..240).step_by(3) {
        assert_identical(80, 24, tick);
    }
}

#[test]
fn dashboard_scenes_match_at_200x60() {
    for tick in (0..240).step_by(17) {
        assert_identical(200, 60, tick);
    }
}

#[test]
fn dashboard_scenes_differ_only_inside_the_table_at_40x12() {
    for tick in [0, 12] {
        let diffs = differences(40, 12, tick);
        let outside: Vec<_> = diffs
            .iter()
            .filter(|line| {
                let coords = line.trim_start_matches('(').split(')').next().unwrap_or("");
                let mut parts = coords.split(',');
                let x: u16 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                let y: u16 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                !((4..24).contains(&x) && (2..10).contains(&y))
            })
            .collect();
        assert!(outside.is_empty(), "tick {tick}: {outside:#?}");
    }
}
