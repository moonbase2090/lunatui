// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, BorderType, Overlay, RenderCx, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn rendering_an_overlay_with_a_shadow_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 12);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 40, 12);
    let shadow = Color::Rgb(6, 7, 12);
    let overlay = Overlay::new(
        Block::bordered().border_type(BorderType::Rounded),
        Rect::new(10, 3, 20, 5),
    )
    .backdrop(0.5, Color::Rgb(0, 0, 0))
    .shadow(Style::new().bg(shadow));
    overlay.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(area, Style::new(), '.', &mut clusters);
    assert_eq!(grid.row_display(3, &clusters), ".".repeat(40));
    let ((), stats) = ALLOC.measure(|| {
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&overlay).render(area, &mut cx);
        }
    });
    println!(
        "overlay renders=100; measured allocations={}",
        stats.allocations
    );
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(3, &clusters),
        format!("{}╭{}╮{}", ".".repeat(10), "─".repeat(18), ".".repeat(10))
    );
    assert_eq!(
        grid.row_display(4, &clusters),
        format!("{}│{}│ {}", ".".repeat(10), " ".repeat(18), ".".repeat(9)),
        "modal cleared, shadow column blank"
    );
    let bg = |x, y| grid.get(x, y).map(|cell| cell.style().bg);
    assert_eq!(bg(30, 4), Some(shadow), "right edge of the shadow");
    assert_eq!(bg(15, 8), Some(shadow), "bottom edge of the shadow");
    assert_ne!(bg(30, 3), Some(shadow), "no shadow above the offset");
}
