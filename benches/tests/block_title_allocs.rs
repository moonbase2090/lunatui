// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Alignment, Block, BorderType, RenderCx, TitlePosition, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn rendering_a_titled_block_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 4);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 40, 4);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title("Deploys")
        .title(" prod ")
        .title_top_aligned("12:04", Alignment::Center)
        .title_top_aligned("3 failed", Alignment::Right)
        .title_at(
            "Esc closes",
            TitlePosition::Bottom,
            Alignment::Right,
            Style::new(),
        );
    block.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(area, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let ((), stats) = ALLOC.measure(|| {
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&block).render(area, &mut cx);
        }
    });
    println!(
        "titled block renders=100; measured allocations={}",
        stats.allocations
    );
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(0, &clusters),
        "╭Deploys─ prod ──12:04─────────3 failed╮"
    );
    assert_eq!(
        grid.row_display(3, &clusters),
        "╰────────────────────────────Esc closes╯"
    );
}
