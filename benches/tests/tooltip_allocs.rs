// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, Tooltip, TooltipSide, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn rendering_a_wrapped_tooltip_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 8);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 40, 8);
    let tooltip = Tooltip::new(
        "unused variable `tag`\nPrefix it with an underscore to silence this warning.",
        Rect::new(2, 6, 6, 1),
    )
    .side(TooltipSide::Below)
    .max_width(30);
    tooltip.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    let (drawn, stats) = ALLOC.measure(|| {
        let mut drawn = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&tooltip).render(screen, &mut cx);
            drawn += u32::from(black_box(&tooltip).area(screen).is_some());
        }
        drawn
    });
    println!(
        "tooltip renders={drawn}; measured allocations={}",
        stats.allocations
    );
    assert_eq!(drawn, 100);
    assert_eq!(stats.allocations, 0);
    let rows: Vec<String> = (0..6).map(|y| grid.row_display(y, &clusters)).collect();
    assert_eq!(rows[3].trim_end(), "   unused variable `tag`");
    assert_eq!(rows[4].trim_end(), "   Prefix it with an underscore");
    assert_eq!(rows[5].trim_end(), "   to silence this warning.");
}
