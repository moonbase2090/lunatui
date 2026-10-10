// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../../README.md")]

use std::io;

use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
use lunatui_bench::scenes::ratatui::log::{self, Layout};
use lunatui_bench::scenes::ratatui::{ColorOutput, encode, gradient, set_color_output, table};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Style, UnderlineStyle, width};
use lunatui_term::{ColorMode, Encoder};
use lunatui_test::{Oracle, diff_matches};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color as RColor;
use ratatui::style::Modifier;
use ratatui::style::Style as RStyle;

const SCENE: Rect = Rect::new(0, 0, 40, 12);

#[test]
fn scripted_grids_match_ratatui_and_the_oracle() {
    set_color_output(ColorOutput::Enabled);
    ascii_update();
    wide_update();
    styled_update();
}

#[test]
fn scene_frames_match_the_oracle() {
    set_color_output(ColorOutput::Enabled);
    let mut dashboard = Buffer::empty(SCENE);
    dashboard::render(&Model::new(), SCENE, &mut dashboard);
    assert_scene("dashboard", &dashboard);

    let mut log_buf = Buffer::empty(SCENE);
    log::render(&log::text(30), 0, Layout::Wrapped, SCENE, &mut log_buf);
    assert_scene("log", &log_buf);

    let rows = table::rows(40);
    let mut table_buf = Buffer::empty(SCENE);
    table::render(&rows, rows.len(), SCENE, &mut table_buf);
    assert_scene("table", &table_buf);

    assert_scene("gradient", &gradient::frame(SCENE));
}

fn ascii_update() {
    let area = Rect::new(0, 0, 4, 1);
    let mut previous = Buffer::empty(area);
    previous[(0, 0)].set_symbol("A");
    let mut next = previous.clone();
    next[(1, 0)].set_symbol("B");
    assert_pair("ascii", &previous, &next);
}

fn wide_update() {
    let area = Rect::new(0, 0, 4, 1);
    let mut previous = Buffer::empty(area);
    previous[(0, 0)].set_symbol("a");
    let mut next = previous.clone();
    next[(1, 0)].set_symbol("日");
    assert_pair("wide", &previous, &next);
}

fn styled_update() {
    let area = Rect::new(0, 0, 3, 1);
    let mut previous = Buffer::empty(area);
    previous[(0, 0)].set_symbol("x").set_style(
        RStyle::default()
            .fg(RColor::Red)
            .add_modifier(Modifier::BOLD),
    );
    let mut next = Buffer::empty(area);
    next[(0, 0)].set_symbol("y").set_style(
        RStyle::default()
            .bg(RColor::Rgb(1, 2, 3))
            .add_modifier(Modifier::ITALIC | Modifier::UNDERLINED),
    );
    assert_pair("styled", &previous, &next);
}

fn assert_scene(name: &str, frame: &Buffer) {
    let blank = Buffer::empty(frame.area);
    assert_pair(name, &blank, frame);
}

fn assert_pair(name: &str, previous: &Buffer, next: &Buffer) {
    let (grid_prev, _) = grid_from_buffer(previous);
    let (grid_next, clusters) = grid_from_buffer(next);
    let ours = diff_matches(&grid_prev, &grid_next, &clusters, ColorMode::TrueColor);
    assert!(
        ours.is_ok(),
        "{name} lunatui oracle: {}",
        ours.map_or_else(|err| err.to_string(), |()| String::new())
    );

    let blank_buffer = Buffer::empty(next.area);
    let mut rat_prev = Vec::new();
    let mut rat_next_bytes = Vec::new();
    encode(&blank_buffer, previous, &mut rat_prev).expect_encode(name);
    encode(previous, next, &mut rat_next_bytes).expect_encode(name);
    let mut rat_oracle = Oracle::new(next.area.height, next.area.width);
    rat_oracle.process(&rat_prev);
    rat_oracle.process(&rat_next_bytes);

    let mut lun_oracle = Oracle::new(next.area.height, next.area.width);
    let blank = Grid::new(grid_next.width(), grid_next.height());
    let mut encoder = Encoder::new();
    let paint = encoder
        .encode_diff(&blank, &grid_prev, &clusters, ColorMode::TrueColor)
        .to_vec();
    lun_oracle.process(&paint);
    let diff = encoder
        .encode_diff(&grid_prev, &grid_next, &clusters, ColorMode::TrueColor)
        .to_vec();
    lun_oracle.process(&diff);
    assert_eq!(
        rat_oracle.contents(),
        lun_oracle.contents(),
        "{name} ratatui and lunatui oracle text"
    );
}

trait EncodeResult {
    fn expect_encode(self, name: &str);
}

impl EncodeResult for io::Result<lunatui_bench::scenes::ratatui::Encoded> {
    fn expect_encode(self, name: &str) {
        assert!(self.is_ok(), "{name} ratatui encode");
    }
}

fn grid_from_buffer(buffer: &Buffer) -> (Grid, ClusterTable) {
    let area = buffer.area;
    let mut grid = Grid::new(area.width, area.height);
    let mut clusters = ClusterTable::new();
    for y in 0..area.height {
        let mut x = 0;
        while x < area.width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol.is_empty() {
                x += 1;
                continue;
            }
            grid.put_str(x, y, map_style(cell.style()), symbol, &mut clusters);
            let span = u16::try_from(width(symbol).max(1)).unwrap_or(1);
            x += span;
        }
    }
    (grid, clusters)
}

fn map_style(style: RStyle) -> Style {
    let mut attrs = Attrs::new();
    let modifier = style.add_modifier;
    if modifier.contains(Modifier::BOLD) {
        attrs = attrs.set_bold(true);
    }
    if modifier.contains(Modifier::DIM) {
        attrs = attrs.set_dim(true);
    }
    if modifier.contains(Modifier::ITALIC) {
        attrs = attrs.set_italic(true);
    }
    if modifier.contains(Modifier::UNDERLINED) {
        attrs = attrs.set_underline(UnderlineStyle::Single);
    }
    if modifier.contains(Modifier::REVERSED) {
        attrs = attrs.set_reverse(true);
    }
    Style::new()
        .fg(map_color(style.fg.unwrap_or(RColor::Reset)))
        .bg(map_color(style.bg.unwrap_or(RColor::Reset)))
        .with_attrs(attrs)
}

fn map_color(color: RColor) -> Color {
    match color {
        RColor::Reset => Color::Default,
        RColor::Black => Color::Indexed(0),
        RColor::Red => Color::Indexed(1),
        RColor::Green => Color::Indexed(2),
        RColor::Yellow => Color::Indexed(3),
        RColor::Blue => Color::Indexed(4),
        RColor::Magenta => Color::Indexed(5),
        RColor::Cyan => Color::Indexed(6),
        RColor::Gray => Color::Indexed(7),
        RColor::DarkGray => Color::Indexed(8),
        RColor::LightRed => Color::Indexed(9),
        RColor::LightGreen => Color::Indexed(10),
        RColor::LightYellow => Color::Indexed(11),
        RColor::LightBlue => Color::Indexed(12),
        RColor::LightMagenta => Color::Indexed(13),
        RColor::LightCyan => Color::Indexed(14),
        RColor::White => Color::Indexed(15),
        RColor::Indexed(index) => Color::Indexed(index),
        RColor::Rgb(red, green, blue) => Color::Rgb(red, green, blue),
    }
}
