// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![cfg(feature = "app")]
#![doc = include_str!("../README.md")]

use lunatui_core::Rect;
use lunatui_runtime::{AccessibilityRole, ControlMetadata, HitId, HitMap};
use lunatui_term::{CellPixels, MouseEvent, MouseKind, MouseUnit};

fn pixel_event(x: u16, y: u16) -> Result<MouseEvent, String> {
    let cell = CellPixels::new(8, 16).ok_or("no cell size")?;
    Ok(MouseEvent::new(MouseKind::Move, x, y)
        .unit(MouseUnit::Pixels)
        .in_cells(cell))
}

#[test]
fn the_pixel_refines_where_the_pointer_is_inside_the_region() -> Result<(), String> {
    let mut map = HitMap::new();
    map.push(HitId(7), Rect::new(2, 1, 10, 1), 0);
    let point = map
        .pick_point(&pixel_event(5 * 8 + 6, 16 + 3)?)
        .ok_or("no hit")?;
    assert_eq!(point.id, HitId(7));
    assert_eq!(point.area, Rect::new(2, 1, 10, 1));
    assert!((point.across - (3.0 + 6.5 / 8.0) / 10.0).abs() < 1e-12);
    assert!((point.down - 3.5 / 16.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn a_cell_event_puts_the_pointer_at_the_centre_of_its_cell() -> Result<(), String> {
    let mut map = HitMap::new();
    map.push(HitId(7), Rect::new(2, 1, 10, 2), 0);
    let point = map
        .pick_point(&MouseEvent::new(MouseKind::Move, 5, 2))
        .ok_or("no hit")?;
    assert!((point.across - 3.5 / 10.0).abs() < 1e-12);
    assert!((point.down - 1.5 / 2.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn a_refined_pick_follows_the_cell_pick_for_misses_and_disabled_controls() -> Result<(), String> {
    let mut map = HitMap::new();
    map.push_control(
        HitId(3),
        Rect::new(0, 0, 4, 1),
        0,
        ControlMetadata::new("off", AccessibilityRole::Button).disabled(true),
    );
    assert_eq!(map.pick(1, 0), None);
    assert_eq!(map.pick_point(&pixel_event(12, 4)?), None);
    assert_eq!(map.pick_point(&pixel_event(80, 4)?), None);
    Ok(())
}
