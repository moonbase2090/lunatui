// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_badge::{Badge, Kbd, Look};
use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::{Role, Tone};
use lunatui_widgets::{RenderCx, Widget};

fn draw(w: u16, h: u16, widget: &dyn Widget) -> (Grid, ClusterTable, Theme) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(w, h);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, w, h), &mut cx);
    (grid, clusters, theme)
}

#[test]
fn badge_pads_and_measures_its_label() {
    let badge = Badge::new("live");
    assert_eq!(badge.measure(Size::new(40, 5)), Size::new(6, 1));
    let (grid, clusters, _) = draw(8, 1, &badge);
    assert_eq!(grid.row_display(0, &clusters), " live   ");
}

#[test]
fn filled_badge_uses_the_tone_fill() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(8, 1, &Badge::new("failed").tone(Tone::Danger));
    let cell = grid.get(1, 0).ok_or("missing cell")?;
    assert_eq!(cell.style().bg, theme.resolve(Tone::Danger.fill()));
    let (grid, _, theme) = draw(
        8,
        1,
        &Badge::new("ok").look(Look::Subtle).tone(Tone::Success),
    );
    let cell = grid.get(1, 0).ok_or("missing cell")?;
    assert_eq!(cell.style().fg, theme.resolve(Tone::Success.fill()));
    assert_eq!(cell.style().bg, theme.resolve(Role::Card.token()));
    let (grid, _, theme) = draw(8, 1, &Badge::new("idle"));
    let cell = grid.get(1, 0).ok_or("missing cell")?;
    assert_eq!(cell.style().bg, theme.resolve(Role::Background.token()));
    assert_ne!(cell.style().bg, theme.resolve(Role::Card.token()));
    let (grid, _, theme) = draw(8, 1, &Badge::new("idle").look(Look::Subtle));
    let cell = grid.get(1, 0).ok_or("missing cell")?;
    assert_ne!(cell.style().fg, theme.resolve(Role::Card.token()));
    Ok(())
}

#[test]
fn badge_clips_to_a_narrow_area() {
    let (grid, clusters, _) = draw(4, 1, &Badge::new("deploying"));
    assert_eq!(grid.row_display(0, &clusters), " dep");
    let (grid, clusters, _) = draw(0, 1, &Badge::new("x"));
    assert_eq!(grid.display_text(&clusters).trim(), "");
}

#[test]
fn kbd_draws_one_chip_per_key() {
    let kbd = Kbd::new(&["Ctrl", "K"]);
    assert_eq!(kbd.measure(Size::new(40, 1)), Size::new(10, 1));
    let (grid, clusters, _) = draw(12, 1, &kbd);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), " Ctrl   K");
}

#[test]
fn kbd_stops_at_the_area_edge() {
    let long = "k".repeat(70_000);
    let keys = [long.as_str(), "x"];
    let kbd = Kbd::new(&keys);
    let avail = Size::new(10, 1);
    assert_eq!(kbd.measure(avail), avail);
    let (grid, clusters, _) = draw(6, 1, &Kbd::new(&["Esc", "Tab", "q"]));
    assert_eq!(grid.row_display(0, &clusters), " Esc  ");
}
