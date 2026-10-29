// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{ComboBox, RenderCx, SearchBox, Widget, WidgetState};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const FRUIT: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];

#[test]
fn drawing_an_open_focused_combo_box_with_supplied_matches_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 10);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 40, 10);
    let found = SearchBox::filter(&FRUIT, "ap");
    let combo = ComboBox::new("ap", 2, &FRUIT)
        .open(true)
        .active(Some(1))
        .state(WidgetState::Focused)
        .with_matches(&found);
    combo.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(area, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(1, &clusters).trim_end(), "");
    let (drawn, stats) = ALLOC.measure(|| {
        let mut drawn = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&combo).render(area, &mut cx);
            drawn += 1;
        }
        drawn
    });
    println!(
        "combo box renders={drawn}; measured allocations={}",
        stats.allocations
    );
    assert_eq!(drawn, 100);
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(0, &clusters).trim_end(),
        format!(" ap{}▴", " ".repeat(35))
    );
    assert_eq!(grid.row_display(1, &clusters).trim_end(), " apple");
    assert_eq!(grid.row_display(2, &clusters).trim_end(), " apricot");
    assert_eq!(grid.row_display(3, &clusters).trim_end(), "");
}
