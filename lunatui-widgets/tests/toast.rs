// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{Cell, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    RenderCx, Toast, ToastId, ToastKind, ToastPlacement, ToastScope, ToastStack,
};

const AREA: Rect = Rect::new(0, 0, 30, 10);
const PANE: Rect = Rect::new(0, 0, 30, 8);

fn pane(id: u32) -> Option<Rect> {
    (id == 1).then_some(PANE)
}

fn secs(count: u64) -> Duration {
    Duration::from_secs(count)
}

fn push(stack: &mut ToastStack, now: Duration, ttl: u64, text: &str) -> ToastId {
    stack.push(
        now,
        Some(secs(ttl)),
        ToastScope::Pane(1),
        ToastKind::Info,
        text,
    )
}

fn rects(stack: &ToastStack, now: Duration) -> Vec<(u32, Rect)> {
    stack
        .placements(now, AREA, pane)
        .into_iter()
        .map(|placement| (placement.id.0, placement.rect))
        .collect()
}

fn draw(stack: &ToastStack, now: Duration) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(30, 10);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    stack.render(now, AREA, pane, &mut cx);
    (grid, clusters)
}

fn rows(grid: &Grid, clusters: &ClusterTable) -> Vec<String> {
    (0..10).map(|y| grid.row_display(y, clusters)).collect()
}

#[test]
fn a_pane_toast_is_a_rounded_box_in_the_pane_corner() {
    let mut stack = ToastStack::new();
    push(&mut stack, Duration::ZERO, 5, "Saved");
    let (grid, clusters) = draw(&stack, Duration::ZERO);
    assert_eq!(
        rows(&grid, &clusters),
        [
            "                              ",
            "                              ",
            "                              ",
            "                              ",
            "                              ",
            "                     ╭───────╮",
            "                     │ Saved │",
            "                     ╰───────╯",
            "                              ",
            "                              ",
        ]
    );
}

#[test]
fn the_newest_toast_takes_the_corner_and_older_ones_stack_above() {
    let mut stack = ToastStack::new();
    let first = push(&mut stack, Duration::ZERO, 5, "one");
    let second = push(&mut stack, Duration::ZERO, 5, "two!");
    assert_eq!(
        rects(&stack, Duration::ZERO),
        [
            (first.0, Rect::new(23, 2, 7, 3)),
            (second.0, Rect::new(22, 5, 8, 3)),
        ]
    );
}

#[test]
fn newer_toasts_sit_above_older_ones_and_status_above_panes() {
    let mut stack = ToastStack::new();
    push(&mut stack, Duration::ZERO, 5, "one");
    push(&mut stack, Duration::ZERO, 5, "two");
    stack.push(
        Duration::ZERO,
        Some(secs(5)),
        ToastScope::Status,
        ToastKind::Success,
        "done",
    );
    let z: Vec<u16> = stack
        .placements(Duration::ZERO, AREA, pane)
        .iter()
        .map(|placement| placement.z)
        .collect();
    assert_eq!(z, [100, 101, 1000]);
}

#[test]
fn a_toast_goes_away_exactly_when_its_time_is_up() {
    let mut stack = ToastStack::new();
    let id = push(&mut stack, secs(10), 5, "later");
    assert_eq!(stack.get(id).and_then(Toast::expires), Some(secs(15)));
    assert_eq!(stack.expire(Duration::from_millis(14_999)), []);
    assert_eq!(rects(&stack, Duration::from_millis(14_999)).len(), 1);
    assert_eq!(rects(&stack, secs(15)), []);
    assert_eq!(stack.expire(secs(15)), [id]);
    assert!(stack.is_empty());
    assert_eq!(stack.expire(secs(99)), []);
}

#[test]
fn expiry_is_per_toast_and_sticky_toasts_stay() {
    let mut stack = ToastStack::new();
    let short = push(&mut stack, Duration::ZERO, 2, "short");
    let long = push(&mut stack, Duration::ZERO, 6, "long");
    let sticky = stack.push(
        Duration::ZERO,
        None,
        ToastScope::Pane(1),
        ToastKind::Warning,
        "sticky",
    );
    assert_eq!(stack.get(sticky).and_then(Toast::expires), None);
    assert_eq!(stack.expire(secs(2)), [short]);
    assert_eq!(stack.len(), 2);
    assert_eq!(stack.expire(secs(6)), [long]);
    assert_eq!(stack.expire(secs(3600)), []);
    assert_eq!(stack.len(), 1);
    assert!(stack.get(sticky).is_some());
}

#[test]
fn a_zero_ttl_toast_is_gone_at_once() {
    let mut stack = ToastStack::new();
    push(&mut stack, secs(3), 0, "blink");
    assert_eq!(rects(&stack, secs(3)), []);
    assert_eq!(stack.expire(secs(3)).len(), 1);
}

#[test]
fn expired_toasts_leave_a_gap_that_older_ones_fall_into() {
    let mut stack = ToastStack::new();
    let old = push(&mut stack, Duration::ZERO, 10, "old");
    push(&mut stack, Duration::ZERO, 2, "new");
    assert_eq!(
        rects(&stack, Duration::ZERO)[0],
        (old.0, Rect::new(23, 2, 7, 3))
    );
    assert_eq!(rects(&stack, secs(2)), [(old.0, Rect::new(23, 5, 7, 3))]);
}

#[test]
fn only_the_newest_toasts_up_to_the_limit_are_shown() {
    let mut stack = ToastStack::new().max_visible(2);
    let tall = |_| Some(Rect::new(0, 0, 30, 20));
    let shown = |stack: &ToastStack| -> Vec<u32> {
        stack
            .placements(Duration::ZERO, AREA, tall)
            .iter()
            .map(|placement| placement.id.0)
            .collect()
    };
    let a = push(&mut stack, Duration::ZERO, 50, "a");
    let b = push(&mut stack, Duration::ZERO, 50, "b");
    let c = push(&mut stack, Duration::ZERO, 50, "c");
    assert!(!stack.is_empty());
    assert_eq!(shown(&stack), [b.0, c.0]);
    assert!(stack.dismiss(c));
    assert_eq!(shown(&stack), [a.0, b.0]);
    assert_eq!(stack.len(), 2);
}

#[test]
fn toasts_that_do_not_fit_the_pane_are_left_out() {
    let mut stack = ToastStack::new();
    push(&mut stack, Duration::ZERO, 5, "one");
    push(&mut stack, Duration::ZERO, 5, "two");
    push(&mut stack, Duration::ZERO, 5, "three");
    assert_eq!(rects(&stack, Duration::ZERO).len(), 2);
    let short = |id: u32| (id == 1).then_some(Rect::new(0, 0, 30, 2));
    assert_eq!(stack.placements(Duration::ZERO, AREA, short), []);
    let exact = |id: u32| (id == 1).then_some(Rect::new(0, 0, 30, 3));
    assert_eq!(stack.placements(Duration::ZERO, AREA, exact).len(), 1);
}

#[test]
fn a_long_message_is_clipped_to_the_pane_width() {
    let mut stack = ToastStack::new();
    push(&mut stack, Duration::ZERO, 5, "abcdefghijkl");
    let narrow = |_| Some(Rect::new(0, 0, 10, 8));
    let placed = stack.placements(Duration::ZERO, AREA, narrow);
    assert_eq!(placed[0].rect, Rect::new(0, 5, 10, 3));
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(30, 10);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    stack.render(Duration::ZERO, AREA, narrow, &mut cx);
    assert_eq!(grid.row_display(6, &clusters).trim_end(), "│ abcdef │");
    assert_eq!(grid.row_display(5, &clusters).trim_end(), "╭────────╮");
}

#[test]
fn a_pane_too_narrow_for_a_box_shows_nothing() {
    let mut stack = ToastStack::new();
    push(&mut stack, Duration::ZERO, 5, "x");
    let thin = |_| Some(Rect::new(0, 0, 3, 8));
    assert_eq!(stack.placements(Duration::ZERO, AREA, thin), []);
    let just = |_| Some(Rect::new(0, 0, 4, 8));
    assert_eq!(stack.placements(Duration::ZERO, AREA, just).len(), 1);
}

#[test]
fn a_toast_for_a_pane_that_is_not_on_screen_waits() {
    let mut stack = ToastStack::new();
    stack.push(
        Duration::ZERO,
        Some(secs(5)),
        ToastScope::Pane(2),
        ToastKind::Info,
        "later",
    );
    assert_eq!(rects(&stack, Duration::ZERO), []);
    let both = |_| Some(PANE);
    assert_eq!(stack.placements(Duration::ZERO, AREA, both).len(), 1);
}

#[test]
fn panes_keep_their_own_stacks() {
    let mut stack = ToastStack::new();
    let left = Rect::new(0, 0, 14, 10);
    let right = Rect::new(15, 0, 15, 10);
    let panes = |id: u32| match id {
        1 => Some(left),
        2 => Some(right),
        _ => None,
    };
    let a = stack.push(
        Duration::ZERO,
        None,
        ToastScope::Pane(1),
        ToastKind::Info,
        "aa",
    );
    let b = stack.push(
        Duration::ZERO,
        None,
        ToastScope::Pane(2),
        ToastKind::Info,
        "bb",
    );
    let c = stack.push(
        Duration::ZERO,
        None,
        ToastScope::Pane(1),
        ToastKind::Info,
        "cc",
    );
    let placed: Vec<(u32, Rect)> = stack
        .placements(Duration::ZERO, AREA, panes)
        .iter()
        .map(|p: &ToastPlacement| (p.id.0, p.rect))
        .collect();
    assert_eq!(
        placed,
        [
            (a.0, Rect::new(8, 4, 6, 3)),
            (b.0, Rect::new(24, 7, 6, 3)),
            (c.0, Rect::new(8, 7, 6, 3)),
        ]
    );
}

#[test]
fn a_status_toast_is_one_row_on_the_bottom_right() {
    let mut stack = ToastStack::new();
    stack.push(
        Duration::ZERO,
        Some(secs(5)),
        ToastScope::Status,
        ToastKind::Success,
        "Ready",
    );
    let (grid, clusters) = draw(&stack, Duration::ZERO);
    assert_eq!(
        grid.row_display(9, &clusters),
        "                        Ready "
    );
    assert_eq!(grid.row_display(8, &clusters).trim(), "");
}

#[test]
fn status_toasts_stack_upward_one_row_each() {
    let mut stack = ToastStack::new();
    let a = stack.push(
        Duration::ZERO,
        None,
        ToastScope::Status,
        ToastKind::Info,
        "a",
    );
    let b = stack.push(
        Duration::ZERO,
        None,
        ToastScope::Status,
        ToastKind::Info,
        "bb",
    );
    assert_eq!(
        stack
            .placements(Duration::ZERO, AREA, pane)
            .iter()
            .map(|p| (p.id, p.rect))
            .collect::<Vec<_>>(),
        [(a, Rect::new(27, 8, 3, 1)), (b, Rect::new(26, 9, 4, 1)),]
    );
}

#[test]
fn a_status_toast_draws_over_a_pane_toast() {
    let mut stack = ToastStack::new();
    stack.push(
        Duration::ZERO,
        None,
        ToastScope::Pane(1),
        ToastKind::Info,
        "Saved",
    );
    stack.push(
        Duration::ZERO,
        None,
        ToastScope::Status,
        ToastKind::Warning,
        "Low disk",
    );
    let tall = |_| Some(Rect::new(0, 0, 30, 10));
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(30, 10);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    stack.render(Duration::ZERO, AREA, tall, &mut cx);
    assert_eq!(
        grid.row_display(9, &clusters),
        "                     Low disk "
    );
    assert_eq!(
        grid.row_display(8, &clusters).trim_end(),
        "                     │ Saved │"
    );
}

#[test]
fn kinds_pick_their_theme_colors_and_the_fill_is_the_surface() {
    let theme = Theme::builtin(Variant::Dark);
    for (kind, token) in [
        (ToastKind::Info, Token::Accent),
        (ToastKind::Success, Token::Ok),
        (ToastKind::Warning, Token::Warn),
        (ToastKind::Error, Token::Fail),
    ] {
        let mut stack = ToastStack::new();
        stack.push(Duration::ZERO, None, ToastScope::Pane(1), kind, "Hi");
        let (grid, _) = draw(&stack, Duration::ZERO);
        let style = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default();
        let border: Style = style(24, 5);
        assert_eq!(border.fg, theme.resolve(token), "{kind:?} border");
        assert_eq!(
            border.bg,
            theme.resolve(Token::Surface1),
            "{kind:?} border fill"
        );
        let text = style(26, 6);
        assert_eq!(text.fg, theme.resolve(Token::TextPrimary), "{kind:?} text");
        assert_eq!(
            text.bg,
            theme.resolve(Token::Surface1),
            "{kind:?} text fill"
        );
        let padding = style(25, 6);
        assert_eq!(
            padding.bg,
            theme.resolve(Token::Surface1),
            "{kind:?} padding"
        );
    }
    let mut stack = ToastStack::new();
    stack.push(
        Duration::ZERO,
        None,
        ToastScope::Status,
        ToastKind::Error,
        "Hi",
    );
    let (grid, _) = draw(&stack, Duration::ZERO);
    let style = |x, y| grid.get(x, y).map(Cell::style).unwrap_or_default();
    assert_eq!(style(28, 9).fg, theme.resolve(Token::Fail));
    assert_eq!(style(27, 9).bg, theme.resolve(Token::Surface1));
}

#[test]
fn dismiss_and_clear_remove_toasts() {
    let mut stack = ToastStack::new();
    let a = push(&mut stack, Duration::ZERO, 5, "a");
    let b = push(&mut stack, Duration::ZERO, 5, "b");
    assert_ne!(a, b);
    assert!(stack.dismiss(a));
    assert!(!stack.dismiss(a));
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.get(b).map(Toast::text), Some("b"));
    assert_eq!(stack.get(b).map(Toast::kind), Some(ToastKind::Info));
    assert_eq!(stack.get(b).map(Toast::scope), Some(ToastScope::Pane(1)));
    assert_eq!(stack.get(b).map(Toast::id), Some(b));
    stack.clear();
    assert!(stack.is_empty());
    assert_eq!(ToastStack::default(), ToastStack::new());
}

#[test]
fn ids_keep_counting_after_toasts_go_away() {
    let mut stack = ToastStack::new();
    let a = push(&mut stack, Duration::ZERO, 1, "a");
    stack.expire(secs(1));
    let b = push(&mut stack, secs(1), 1, "b");
    assert_eq!((a.0, b.0), (0, 1));
}

#[test]
fn drawing_with_nothing_to_show_leaves_the_grid_alone() {
    let stack = ToastStack::new();
    let (grid, clusters) = draw(&stack, Duration::ZERO);
    assert!(
        rows(&grid, &clusters)
            .iter()
            .all(|row| row.trim().is_empty())
    );
}
