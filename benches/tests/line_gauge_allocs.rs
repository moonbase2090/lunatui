// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{LineGauge, RenderCx, Widget};
use std::hint::black_box;

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn actual_default_value_and_subcell_render_allocate_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(20, 1);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 20, 1);
    LineGauge::new()
        .label("CPU")
        .ratio(0.25)
        .render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    let ((), stats) = ALLOC.measure(|| {
        LineGauge::new()
            .label("CPU")
            .ratio(black_box(0.53125))
            .render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    });
    assert_eq!(grid.row_display(0, &clusters), "CPU ━━━━━━▍───── 53%");
    assert_eq!(stats.allocations, 0);
}
