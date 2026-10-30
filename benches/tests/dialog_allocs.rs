// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Dialog, DialogKind, DialogState, RenderCx, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_and_hit_testing_a_prompt_dialog_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(60, 16);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 60, 16);
    let mut dialog_state = DialogState::new();
    dialog_state.open(DialogKind::Prompt, Some(3));
    let dialog = Dialog::prompt(
        "Rename branch",
        "Pick a new name for feature/login. Open pull requests keep their history.",
        "feature/login-form",
        7,
    )
    .placeholder("branch name")
    .width(44)
    .state(&dialog_state);
    dialog.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(4, &clusters).trim_end(), "");
    let (hits, stats) = ALLOC.measure(|| {
        let mut hits = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&dialog).render(screen, &mut cx);
            hits += u32::from(black_box(&dialog).control_at(screen, 45, 10).is_some());
        }
        hits
    });
    println!(
        "dialog renders=100; measured allocations={}",
        stats.allocations
    );
    assert_eq!(hits, 100);
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(3, &clusters),
        format!(
            "{}╭Rename branch{}╮{}",
            " ".repeat(8),
            "─".repeat(29),
            " ".repeat(8)
        )
    );
}
