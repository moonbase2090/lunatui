// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_prismattyc::{FrameDiff, HostRenderer, PresentStats, PrismattycBackend};

struct RecordingHost {
    frames: u32,
    last_runs: usize,
    display: Grid,
}

impl RecordingHost {
    fn new(width: u16, height: u16) -> Self {
        Self {
            frames: 0,
            last_runs: 0,
            display: Grid::new(width, height),
        }
    }
}

impl HostRenderer for RecordingHost {
    fn present_frame(&mut self, frame: &FrameDiff<'_>) -> io::Result<()> {
        self.frames += 1;
        self.last_runs = frame.runs.len();
        self.display = frame.next.clone();
        Ok(())
    }
}

#[test]
fn present_delivers_damage_without_ansi() -> io::Result<()> {
    let mut clusters = ClusterTable::new();
    let style = Style::new().fg(Color::Rgb(0x7a, 0xa2, 0xf7));
    let mut backend = PrismattycBackend::new(8, 2, RecordingHost::new(8, 2));

    let mut first = Grid::new(8, 2);
    first.put_str(0, 0, style, "hi", &mut clusters);
    let stats = backend.present(&first, &clusters)?;
    assert_eq!(stats.ansi_bytes, 0);
    assert!(stats.changed_runs > 0);
    assert_eq!(backend.renderer().frames, 1);
    assert_eq!(
        backend.renderer().display.row_display(0, &clusters),
        "hi      "
    );

    let stats = backend.present(&first, &clusters)?;
    assert_eq!(stats, PresentStats::skipped());
    assert_eq!(backend.renderer().frames, 1);

    let mut second = first.clone();
    second.put_str(3, 0, style, "!", &mut clusters);
    let runs = diff_runs(&first, &second);
    let stats = backend.present(&second, &clusters)?;
    assert_eq!(stats.changed_runs, runs.len());
    assert_eq!(
        stats.changed_cells,
        runs.iter().map(|r| usize::from(r.width)).sum()
    );
    assert_eq!(backend.renderer().frames, 2);
    Ok(())
}

#[test]
fn blank_resize_still_notifies_host() -> io::Result<()> {
    let mut clusters = ClusterTable::new();
    let mut backend = PrismattycBackend::new(4, 2, RecordingHost::new(6, 2));
    let mut grid = Grid::new(4, 2);
    grid.put_str(0, 0, Style::new(), "ab", &mut clusters);
    backend.present(&grid, &clusters)?;

    let mut larger = Grid::new(6, 2);
    larger.put_str(0, 0, Style::new(), "ab", &mut clusters);
    let stats = backend.present(&larger, &clusters)?;
    assert_eq!(stats.changed_runs, 0);
    assert_eq!(stats.ansi_bytes, 0);
    assert_eq!(backend.renderer().frames, 2);
    assert_eq!(backend.previous_grid().width(), 6);
    assert_eq!(backend.renderer().display.width(), 6);

    let mut smaller = Grid::new(3, 2);
    smaller.put_str(0, 0, Style::new(), "ab", &mut clusters);
    let stats = backend.present(&smaller, &clusters)?;
    assert_eq!(stats.changed_runs, 0);
    assert_eq!(backend.renderer().frames, 3);
    assert_eq!(backend.previous_grid().width(), 3);
    Ok(())
}

#[test]
fn resize_invalidates_previous_grid() -> io::Result<()> {
    let mut clusters = ClusterTable::new();
    let mut backend = PrismattycBackend::new(4, 2, RecordingHost::new(4, 2));
    let mut grid = Grid::new(4, 2);
    grid.put_str(0, 0, Style::new(), "ab", &mut clusters);
    backend.present(&grid, &clusters)?;

    let mut larger = Grid::new(6, 2);
    larger.fill(
        Rect {
            x: 0,
            y: 0,
            width: 6,
            height: 2,
        },
        Style::new(),
        ' ',
        &mut clusters,
    );
    larger.put_str(0, 0, Style::new(), "ab", &mut clusters);
    larger.put_str(4, 0, Style::new(), "xy", &mut clusters);
    let stats = backend.present(&larger, &clusters)?;
    assert!(stats.changed_runs > 0);
    assert_eq!(backend.renderer().display.width(), 6);
    assert!(
        backend
            .renderer()
            .display
            .row_display(0, &clusters)
            .contains("xy")
    );
    Ok(())
}
