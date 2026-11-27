// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::graphite_cases::Repaint;
use lunatui_bench::graphite_monitor::{Density, IMAGE_LIMIT, MonitorRun, changed_cells};
use lunatui_core::{DecodedGlyph, Rect};

fn frame(run: &mut MonitorRun) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    run.frame_into(&mut out)
        .map_err(|error| error.to_string())?;
    Ok(out)
}

fn text_cells(run: &MonitorRun, area: Rect) -> Vec<(u16, u16)> {
    let (grid, _) = run.grid();
    (area.y..area.y + area.height)
        .flat_map(|y| (area.x..area.x + area.width).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            grid.get(x, y)
                .is_some_and(|cell| cell.glyph().decode() != DecodedGlyph::Inline(' '))
        })
        .collect()
}

fn cell_bits(run: &MonitorRun, area: Rect) -> Vec<u128> {
    let (grid, _) = run.grid();
    (area.y..area.y + area.height)
        .flat_map(|y| (area.x..area.x + area.width).map(move |x| (x, y)))
        .map(|(x, y)| grid.cell_bits(x, y))
        .collect()
}

fn transmissions(stream: &[u8]) -> usize {
    String::from_utf8_lossy(stream)
        .split("\x1b_G")
        .skip(1)
        .filter(|command| {
            let keys = command.split([';', '\x1b']).next().unwrap_or("");
            keys.split(',').any(|pair| pair == "a=t")
        })
        .count()
}

#[test]
fn the_widgets_paint_every_cell_of_a_full_frame() -> Result<(), String> {
    for density in [Density::Monitor, Density::Dense] {
        for (cols, rows) in [(80, 24), (185, 51), (200, 60)] {
            for scale in [1, 2] {
                let mut run = MonitorRun::new(density, Repaint::Full, scale, cols, rows)?;
                let stream = frame(&mut run)?;
                let left = text_cells(&run, Rect::new(0, 0, cols, rows));
                assert!(
                    left.is_empty(),
                    "{density:?} {cols}x{rows}@{scale}x: text left in cells at {left:?}"
                );
                assert!(
                    run.images() > 10 && run.images() <= IMAGE_LIMIT,
                    "{density:?} {cols}x{rows}: {} images",
                    run.images()
                );
                assert!(
                    transmissions(&stream) >= run.images(),
                    "{density:?} {cols}x{rows}@{scale}x: every image is sent in a full frame"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn the_dense_scene_draws_more_than_the_monitor() -> Result<(), String> {
    for (cols, rows) in [(80, 24), (200, 60)] {
        let mut monitor = MonitorRun::new(Density::Monitor, Repaint::Full, 1, cols, rows)?;
        let mut dense = MonitorRun::new(Density::Dense, Repaint::Full, 1, cols, rows)?;
        let monitor_bytes = frame(&mut monitor)?.len();
        let dense_bytes = frame(&mut dense)?.len();
        assert!(
            dense.images() > monitor.images(),
            "{cols}x{rows}: {} dense images, {} monitor images",
            dense.images(),
            monitor.images()
        );
        assert!(dense_bytes > monitor_bytes, "{cols}x{rows}");
        let seats = changed_cells(cols, rows, Density::Dense)?;
        assert_eq!(seats, Rect::new(0, 2, cols, rows - 3));
    }
    Ok(())
}

#[test]
fn a_changed_frame_redraws_only_the_seat_rows() -> Result<(), String> {
    for density in [Density::Monitor, Density::Dense] {
        let (cols, rows) = (120, 30);
        let seats = changed_cells(cols, rows, density)?;
        let mut changed = MonitorRun::new(density, Repaint::Changed, 1, cols, rows)?;
        let mut full = MonitorRun::new(density, Repaint::Full, 1, cols, rows)?;
        let first = frame(&mut full)?;
        let outside: Vec<Rect> = [
            Rect::new(0, 0, cols, seats.y),
            Rect::new(
                0,
                seats.y + seats.height,
                cols,
                rows - seats.y - seats.height,
            ),
            Rect::new(seats.width, seats.y, cols - seats.width, seats.height),
        ]
        .into_iter()
        .filter(|area| area.width > 0 && area.height > 0)
        .collect();
        let kept: Vec<Vec<u128>> = outside.iter().map(|area| cell_bits(&full, *area)).collect();
        for tick in 0..4 {
            let sent = frame(&mut changed)?;
            if tick > 0 {
                frame(&mut full)?;
            }
            assert_eq!(
                cell_bits(&changed, seats),
                cell_bits(&full, seats),
                "{density:?} tick {tick}: the seat rows match a full frame"
            );
            for (area, cells) in outside.iter().zip(&kept) {
                assert_eq!(
                    &cell_bits(&changed, *area),
                    cells,
                    "{density:?} tick {tick}: cells outside the seat rows stay as first drawn"
                );
            }
            let bound = match density {
                Density::Monitor => first.len() / 2,
                Density::Dense => first.len(),
            };
            assert!(
                sent.len() < bound,
                "{density:?} tick {tick}: a changed frame sent {} bytes, a full frame {}",
                sent.len(),
                first.len()
            );
            assert!(
                transmissions(&sent) < transmissions(&first),
                "{density:?} tick {tick}: the kept images are not sent again"
            );
            assert!(transmissions(&sent) > 0, "{density:?} tick {tick}");
            assert!(text_cells(&changed, seats).is_empty(), "{density:?}");
        }
    }
    Ok(())
}

#[test]
fn frames_are_deterministic_and_change_each_tick() -> Result<(), String> {
    let mut a = MonitorRun::new(Density::Monitor, Repaint::Changed, 2, 80, 24)?;
    let mut b = MonitorRun::new(Density::Monitor, Repaint::Changed, 2, 80, 24)?;
    let mut previous = Vec::new();
    for tick in 0..6 {
        let left = frame(&mut a)?;
        assert_eq!(left, frame(&mut b)?, "tick {tick}");
        assert_ne!(left, previous, "tick {tick} sends something new");
        previous = left;
    }
    assert_eq!(a.tick(), 6);
    Ok(())
}

#[test]
fn a_window_too_small_for_the_monitor_is_refused() {
    for (cols, rows) in [(39, 24), (80, 7)] {
        assert!(MonitorRun::new(Density::Monitor, Repaint::Full, 1, cols, rows).is_err());
        assert!(changed_cells(cols, rows, Density::Dense).is_err());
    }
}
