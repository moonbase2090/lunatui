// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Button, ButtonVariant, RenderCx, Widget, WidgetState};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_measuring_and_hit_testing_buttons_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 3);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 40, 3);
    let buttons = [
        Button::new("Run checks").state(WidgetState::Focused),
        Button::new("Deploy")
            .variant(ButtonVariant::Primary)
            .pressed(true),
        Button::new("Roll back").variant(ButtonVariant::Destructive),
    ];
    let slots = [
        Rect::new(0, 1, 14, 1),
        Rect::new(15, 0, 10, 3),
        Rect::new(26, 1, 13, 1),
    ];
    for (button, slot) in buttons.iter().zip(slots) {
        button.render(slot, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    }
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(1, &clusters).trim_end(), "");
    let (width, stats) = ALLOC.measure(|| {
        let mut width = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            for (button, slot) in black_box(&buttons).iter().zip(slots) {
                button.render(slot, &mut cx);
                width += u32::from(button.measure(Size::new(40, 1)).width);
                width += u32::from(button.hit_area(slot).width);
            }
        }
        width
    });
    println!(
        "button renders=300; measured allocations={}",
        stats.allocations
    );
    assert_eq!(width, 100 * (14 + 10 + 13 + 14 + 10 + 13));
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(1, &clusters),
        "  Run checks     Deploy     Roll back   "
    );
}
