// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, Popover, RenderCx, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn placing_and_drawing_an_untitled_popover_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 12);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 40, 12);
    let popover = Popover::new(Block::new(), Rect::new(30, 0, 6, 1)).size(Size::new(12, 2));
    popover.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(1, &clusters).trim_end(), "");
    let (placed, stats) = ALLOC.measure(|| {
        let mut placed = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&popover).render(screen, &mut cx);
            placed += u32::from(black_box(&popover).area(screen).is_some());
        }
        placed
    });
    println!(
        "popover renders={placed}; measured allocations={}",
        stats.allocations
    );
    assert_eq!(placed, 100);
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(1, &clusters),
        format!("{}╭{}╮", " ".repeat(26), "─".repeat(12))
    );
    assert_eq!(
        grid.row_display(4, &clusters),
        format!("{}╰{}╯", " ".repeat(26), "─".repeat(12))
    );
}
