// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, TagInput, Widget, WidgetState};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_a_wrapped_tag_input_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(24, 4);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 24, 4);
    let tags = ["rust", "tui", "async", "widgets"];
    let input = TagInput::new(&tags, "gri", 3)
        .selected(None)
        .state(WidgetState::Focused);
    input.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(area, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let (drawn, stats) = ALLOC.measure(|| {
        let mut drawn = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&input).render(area, &mut cx);
            drawn += 1;
        }
        drawn
    });
    println!(
        "tag input renders={drawn}; measured allocations={}",
        stats.allocations
    );
    assert_eq!(drawn, 100);
    assert_eq!(stats.allocations, 0);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), " rust ×   tui ×");
    assert_eq!(
        grid.row_display(1, &clusters).trim_end(),
        " async ×   widgets ×"
    );
    assert_eq!(grid.row_display(2, &clusters).trim_end(), "gri");
}
