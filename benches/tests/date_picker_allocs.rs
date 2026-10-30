// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Date, DatePicker, RenderCx, Widget, WidgetState};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_a_focused_date_picker_with_a_range_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(30, 10);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 30, 10);
    let day = |day| Date::from_ymd(2026, 10, day).unwrap_or(Date::UNIX_EPOCH);
    let picker = DatePicker::new(day(14))
        .range(day(8), day(14))
        .today(day(12))
        .min(day(3))
        .max(day(28))
        .state(WidgetState::Focused);
    picker.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(area, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let (drawn, stats) = ALLOC.measure(|| {
        let mut drawn = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&picker).render(area, &mut cx);
            drawn += 1;
        }
        drawn
    });
    println!(
        "date picker renders={drawn}; measured allocations={}",
        stats.allocations
    );
    assert_eq!(drawn, 100);
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(0, &clusters).trim_end(),
        " ‹  October 2026   ›"
    );
    assert_eq!(
        grid.row_display(4, &clusters).trim_end(),
        " 11 12 13 14 15 16 17"
    );
}
