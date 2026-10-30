// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::hint::black_box;

use lunatui_bench::CountingAllocator;
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    NotificationCenter, NotificationLog, RenderCx, ToastKind, Widget, WidgetState,
};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

#[test]
fn drawing_a_notification_center_allocates_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(40, 6);
    let mut clusters = ClusterTable::new();
    let area = Rect::new(0, 0, 40, 6);
    let mut log = NotificationLog::new(8);
    log.push(ToastKind::Success, "Build 4821 finished", "12:04");
    log.push(ToastKind::Warning, "Disk almost full", "12:09");
    let newest = log.push(ToastKind::Error, "Deploy broken", "12:15");
    log.mark_read(newest);
    let center = NotificationCenter::new(&log)
        .selected(Some(1))
        .state(WidgetState::Focused);
    center.render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    grid.fill(area, Style::new(), ' ', &mut clusters);
    assert_eq!(grid.row_display(0, &clusters).trim_end(), "");
    let (drawn, stats) = ALLOC.measure(|| {
        let mut drawn = 0_u32;
        for _ in 0..100 {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            black_box(&center).render(area, &mut cx);
            drawn += 1;
        }
        drawn
    });
    println!(
        "notification center renders={drawn}; measured allocations={}",
        stats.allocations
    );
    assert_eq!(drawn, 100);
    assert_eq!(stats.allocations, 0);
    assert_eq!(
        grid.row_display(0, &clusters).trim_end(),
        " Notifications                 2 unread"
    );
    assert_eq!(
        grid.row_display(1, &clusters).trim_end(),
        " ○ 12:15 Deploy broken                ×"
    );
}
