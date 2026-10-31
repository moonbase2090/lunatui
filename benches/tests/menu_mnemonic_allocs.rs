// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style, UnderlineStyle};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Menu, MenuItem, MenuState, RenderCx, Widget};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_a_menu_with_mnemonics_and_hit_testing_the_bar_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(60, 12);
    let mut clusters = ClusterTable::new();
    let screen = Rect::new(0, 0, 60, 12);
    let mut menus = MenuState::new(vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", 1).mnemonic('n'),
                MenuItem::item("Open", 2).mnemonic('o'),
                MenuItem::item("Save as", 3).mnemonic('a'),
            ],
        )
        .mnemonic('f'),
        MenuItem::group("Edit", vec![MenuItem::item("Undo", 4).mnemonic('u')]).mnemonic('e'),
        MenuItem::group("Selection", vec![MenuItem::item("All", 5).mnemonic('a')]).mnemonic('s'),
    ]);
    menus.press_mnemonic('f');
    let menu = Menu::new(&menus);
    menu.render(screen, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(screen, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let (hits, counted) = ALLOC.measure(|| {
        let mut hits = 0_usize;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&menu).render(screen, &mut cx);
            hits += black_box(&menu).bar_item_at(screen, 14, 0).unwrap_or(0);
        }
        hits
    });
    println!(
        "menu renders=100; measured allocations={}",
        counted.allocations
    );
    assert_eq!(hits, 200);
    assert_eq!(counted.allocations, 0);
    assert_eq!(
        grid.row_display(0, &clusters).trim_end(),
        "  File  Edit  Selection"
    );
    let underlined: Vec<u16> = (0..60)
        .filter(|x| {
            grid.get(*x, 4)
                .is_some_and(|cell| cell.style().attrs.underline() == UnderlineStyle::Single)
        })
        .collect();
    assert_eq!(underlined, [4]);
}
