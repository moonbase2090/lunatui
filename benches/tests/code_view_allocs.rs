// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::ops::Range;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{CodeMark, CodeView, CodeViewState, RenderCx, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn keywords(_: usize, text: &str, paint: &mut dyn FnMut(Range<usize>, Style)) {
    for word in ["fn", "let", "match"] {
        if let Some(at) = text.find(word) {
            paint(
                at..at + word.len(),
                Style::new().fg(Color::Rgb(203, 166, 247)),
            );
        }
    }
}

#[test]
fn drawing_a_scrolled_highlighted_view_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(60, 12);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 60, 12);
    let lines: Vec<String> = (0..200)
        .map(|n| format!("\tlet value_{n} = match input {{ Some(x) => x * {n}, None => 0 }};"))
        .collect();
    let marks = [
        (40, CodeMark::Error),
        (42, CodeMark::Added),
        (45, CodeMark::Breakpoint),
    ];
    let mut state = CodeViewState::new();
    CodeView::new(&lines).sync(screen, &mut state);
    state.set_cursor(Some(44));
    state.scroll_right(2);
    let highlight = keywords;
    let view = CodeView::new(&lines)
        .view(&state)
        .marks(&marks)
        .highlight(&highlight);
    view.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let (hits, counted) = ALLOC.measure(|| {
        let mut hits = 0_usize;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&view).render(screen, &mut cx);
            hits += black_box(&view).line_at(screen, 11).unwrap_or(0);
        }
        hits
    });
    println!(
        "code view renders=100; measured allocations={}",
        counted.allocations
    );
    assert_eq!(hits, 100 * 44);
    assert_eq!(counted.allocations, 0);
    let top = grid.row_display(0, &clusters);
    assert!(
        top.starts_with("  34   let value_33 = match input { Some(x) => x * 33,"),
        "{top}"
    );
    assert_eq!(top.chars().count(), 60);
    let cursor = grid.row_display(11, &clusters);
    assert!(
        cursor.starts_with("  45   let value_44 = match"),
        "{cursor}"
    );
}
