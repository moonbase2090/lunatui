// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, Popup, RenderCx, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn rendering_a_titled_popup_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 12);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 40, 12);
    let popup = Popup::new(Block::new())
        .title("Deploy settings")
        .size(Size::new(20, 3))
        .backdrop(0.0);
    popup.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(3, &clusters).trim_end(), "");
    let ((), stats) = ALLOC.measure(|| {
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&popup).render(screen, &mut cx);
        }
    });
    println!(
        "titled popup renders=100; measured allocations={}",
        stats.allocations
    );
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(3, &clusters),
        format!(
            "{}╭Deploy settings{}╮{}",
            " ".repeat(9),
            "─".repeat(5),
            " ".repeat(9)
        )
    );
}
