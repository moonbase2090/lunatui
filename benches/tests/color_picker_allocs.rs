// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    ColorPicker, ColorPickerFocus, ColorPickerState, RenderCx, Widget, WidgetState,
};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_measuring_and_hit_testing_a_color_picker_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(44, 12);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(2, 1, 40, 10);
    let mut states = [
        ColorPickerState::from_rgb(58, 124, 214),
        ColorPickerState::from_rgb(240, 140, 0),
        ColorPickerState::from_rgb(12, 12, 12),
    ];
    states[1].set_focus(ColorPickerFocus::Hue);
    states[2].set_focus(ColorPickerFocus::Hex);
    states[2].type_digit('f');
    states[2].type_digit('0');
    for state in &states {
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        ColorPicker::new()
            .view(state)
            .state(WidgetState::Focused)
            .render(area, &mut cx);
    }
    let (total, counts) = ALLOC.measure(|| {
        let mut total = 0_u32;
        for _ in 0..50 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            for state in black_box(&mut states) {
                let picker = ColorPicker::new().view(state).state(WidgetState::Focused);
                picker.render(area, &mut cx);
                total += u32::from(picker.measure(Size::new(40, 10)).width);
                total += u32::from(ColorPicker::square_area(area).width);
                total += u32::from(state.hex()[1]);
                state.pointer(10, 4, area);
                state.nudge(1, -1);
            }
        }
        total
    });
    println!(
        "color picker renders=150; measured allocations={}",
        counts.allocations
    );
    assert!(total > 0);
    assert_eq!(counts.allocations, 0);
}
