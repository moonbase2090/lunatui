// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_core::{CellWidth, ClusterTable, Color, Grid, Rect, Style};
use lunatui_runtime::effect::{Direction, Effect, Palette};
use lunatui_runtime::{Ease, MotionPolicy};

const WHITE: Color = Color::Rgb(255, 255, 255);
const BLACK: Color = Color::Rgb(0, 0, 0);

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn palette() -> Palette<'static> {
    Palette::new([255, 255, 255], [0, 0, 0])
}

fn text_grid(width: u16, height: u16, text: &str) -> Grid {
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let style = Style::new().fg(WHITE).bg(BLACK);
    for row in 0..height {
        grid.put_str(0, row, style, text, &mut clusters);
    }
    grid
}

fn cells(grid: &Grid) -> Vec<u128> {
    (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .map(|(x, y)| grid.cell_bits(x, y))
        .collect()
}

fn area(grid: &Grid) -> Rect {
    Rect::new(0, 0, grid.width(), grid.height())
}

fn render(effect: &Effect, elapsed: Duration, grid: &mut Grid) {
    effect.render(elapsed, grid, area(grid), &palette());
}

fn hidden(grid: &Grid, x: u16, y: u16) -> bool {
    grid.get(x, y).is_some_and(|cell| {
        let style = cell.style();
        style.fg == style.bg
    })
}

#[test]
fn strength_follows_the_timing_and_reversal() {
    let fade = Effect::fade(BLACK, ms(100), Ease::Linear);
    assert!((fade.strength(ms(50)) - 0.5).abs() < 1e-6);
    assert!((fade.strength(ms(500)) - 1.0).abs() < 1e-6);
    let reversed = fade.clone().reversed();
    assert!((reversed.strength(ms(25)) - 0.75).abs() < 1e-6);
    let delayed = fade.delayed(ms(40));
    assert!(delayed.strength(ms(40)).abs() < 1e-6);
    assert!((delayed.strength(ms(90)) - 0.5).abs() < 1e-6);
    assert_eq!(delayed.total(), ms(140));
    assert!(!delayed.is_done(ms(139)));
    assert!(delayed.is_done(ms(140)));
}

#[test]
fn fade_blends_in_oklab_and_ends_on_the_target() {
    let fade = Effect::fade(BLACK, ms(100), Ease::Linear);
    let mut grid = text_grid(4, 1, "abcd");
    render(&fade, ms(50), &mut grid);
    let mid = Color::Rgb(99, 99, 99);
    let style = grid.get(1, 0).map(lunatui_core::Cell::style);
    assert_eq!(
        style.map(|s| (s.fg, s.bg)),
        Some((mid, Color::Rgb(0, 0, 0)))
    );
    render(&fade, ms(100), &mut grid);
    assert!(hidden(&grid, 0, 0) && hidden(&grid, 3, 0));
    assert_eq!(grid.get(2, 0).map(|cell| cell.style().fg), Some(BLACK));
}

#[test]
fn zero_strength_leaves_theme_tokens_untouched() {
    let mut grid = Grid::new(3, 1);
    let mut clusters = ClusterTable::new();
    let style = Style::new().fg(Color::Token(5)).bg(Color::Indexed(4));
    grid.put_str(0, 0, style, "abc", &mut clusters);
    let before = cells(&grid);
    let fade_in = Effect::fade(BLACK, ms(100), Ease::Linear).reversed();
    render(&fade_in, ms(100), &mut grid);
    assert_eq!(cells(&grid), before);
    let fade_out = Effect::fade(BLACK, ms(100), Ease::Linear);
    render(&fade_out, Duration::ZERO, &mut grid);
    assert_eq!(cells(&grid), before);
}

#[test]
fn tokens_resolve_through_the_palette_for_cells_and_targets() {
    let resolve = |color: Color| match color {
        Color::Token(5) => Some([10, 20, 30]),
        _ => None,
    };
    let palette = palette().with_resolver(&resolve);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(0, 0, Style::new().fg(WHITE), "ab", &mut clusters);
    let fade = Effect::fade(Color::Token(5), ms(10), Ease::Linear);
    let whole = area(&grid);
    fade.render(ms(10), &mut grid, whole, &palette);
    let style = grid.get(0, 0).map(lunatui_core::Cell::style);
    assert_eq!(style.map(|s| s.fg), Some(Color::Rgb(10, 20, 30)));
    assert_eq!(style.map(|s| s.bg), Some(Color::Rgb(10, 20, 30)));
}

#[test]
fn dissolve_is_deterministic_monotonic_and_scatters() {
    let dissolve = Effect::dissolve(7, ms(100), Ease::Linear);
    let count = |elapsed: u64| {
        let mut grid = text_grid(40, 10, &"x".repeat(40));
        render(&dissolve, ms(elapsed), &mut grid);
        let mut again = text_grid(40, 10, &"x".repeat(40));
        render(&dissolve, ms(elapsed), &mut again);
        assert_eq!(cells(&grid), cells(&again));
        (0..10)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|&(x, y)| hidden(&grid, x, y))
            .count()
    };
    assert_eq!(count(0), 0);
    assert_eq!(count(100), 400);
    let (quarter, half, three) = (count(25), count(50), count(75));
    assert!(quarter < half && half < three);
    assert!((140..260).contains(&half), "half dissolved hid {half}");
    let other = Effect::dissolve(8, ms(100), Ease::Linear);
    let mut a = text_grid(40, 10, &"x".repeat(40));
    let mut b = text_grid(40, 10, &"x".repeat(40));
    render(&dissolve, ms(50), &mut a);
    render(&other, ms(50), &mut b);
    assert_ne!(cells(&a), cells(&b));
}

#[test]
fn hard_sweep_hides_a_growing_prefix_in_its_direction() {
    let sweep = Effect::sweep(Direction::Right, 0.0, ms(100), Ease::Linear);
    let mut grid = text_grid(10, 2, "0123456789");
    render(&sweep, ms(50), &mut grid);
    for y in 0..2 {
        let shown: Vec<bool> = (0..10).map(|x| !hidden(&grid, x, y)).collect();
        assert_eq!(
            shown,
            [
                false, false, false, false, false, true, true, true, true, true
            ]
        );
    }
    let left = Effect::sweep(Direction::Left, 0.0, ms(100), Ease::Linear);
    let mut grid = text_grid(10, 1, "0123456789");
    render(&left, ms(50), &mut grid);
    let shown: Vec<bool> = (0..10).map(|x| !hidden(&grid, x, 0)).collect();
    assert_eq!(
        shown,
        [
            true, true, true, true, true, false, false, false, false, false
        ]
    );
}

#[test]
fn soft_sweep_has_a_partial_edge_and_finishes_hidden() {
    let sweep = Effect::sweep(Direction::Right, 0.5, ms(100), Ease::Linear);
    let mut grid = text_grid(10, 1, "0123456789");
    render(&sweep, ms(40), &mut grid);
    let fg = |x: u16| grid.get(x, 0).map(|cell| cell.style().fg);
    assert!(hidden(&grid, 0, 0));
    assert_ne!(fg(3), Some(WHITE));
    assert!(!hidden(&grid, 3, 0));
    assert_eq!(fg(9), Some(WHITE));
    let mut done = text_grid(10, 1, "0123456789");
    render(&sweep, ms(100), &mut done);
    assert!((0..10).all(|x| hidden(&done, x, 0)));
}

#[test]
fn slide_moves_content_and_blanks_the_gap() {
    let slide = Effect::slide(Direction::Left, ms(100), Ease::Linear);
    let mut clusters = ClusterTable::new();
    let mut grid = text_grid(10, 1, "ABCDEFGHIJ");
    render(&slide, ms(50), &mut grid);
    assert_eq!(&grid.row_display(0, &clusters)[..5], "FGHIJ");
    assert_eq!(&grid.row_display(0, &clusters)[5..], "     ");
    assert!((5..10).all(|x| hidden(&grid, x, 0)));
    assert!((0..5).all(|x| !hidden(&grid, x, 0)));
    let mut grid = text_grid(10, 1, "ABCDEFGHIJ");
    let down = Effect::slide(Direction::Right, ms(100), Ease::Linear).reversed();
    render(&down, ms(50), &mut grid);
    assert_eq!(&grid.row_display(0, &clusters)[5..], "ABCDE");
    clusters = ClusterTable::new();
    let mut vertical = Grid::new(3, 4);
    for (row, text) in ["aaa", "bbb", "ccc", "ddd"].into_iter().enumerate() {
        vertical.put_str(
            0,
            u16::try_from(row).unwrap_or(0),
            Style::new().fg(WHITE).bg(BLACK),
            text,
            &mut clusters,
        );
    }
    render(
        &Effect::slide(Direction::Up, ms(100), Ease::Linear),
        ms(50),
        &mut vertical,
    );
    assert_eq!(vertical.row_display(0, &clusters), "ccc");
    assert_eq!(vertical.row_display(1, &clusters), "ddd");
    assert_eq!(vertical.row_display(2, &clusters), "   ");
    render(
        &Effect::slide(Direction::Down, ms(100), Ease::Linear),
        ms(100),
        &mut vertical,
    );
    assert!((0..4).all(|y| (0..3).all(|x| hidden(&vertical, x, y))));
}

#[test]
fn slide_never_splits_a_wide_character() {
    let slide = Effect::slide(Direction::Left, ms(100), Ease::Linear);
    for elapsed in (0..=100).step_by(10) {
        let mut clusters = ClusterTable::new();
        let mut grid = Grid::new(8, 1);
        grid.put_str(
            0,
            0,
            Style::new().fg(WHITE).bg(BLACK),
            "日本語ab",
            &mut clusters,
        );
        render(&slide, ms(elapsed), &mut grid);
        for x in 0..8 {
            let width = grid.get(x, 0).map(|cell| cell.meta().width);
            if width == Some(CellWidth::Two) {
                let next = grid.get(x + 1, 0).map(|cell| cell.meta().width);
                assert_eq!(next, Some(CellWidth::Continuation), "t={elapsed} x={x}");
            }
            if width == Some(CellWidth::Continuation) {
                let prev = x
                    .checked_sub(1)
                    .and_then(|p| grid.get(p, 0))
                    .map(|cell| cell.meta().width);
                assert_eq!(prev, Some(CellWidth::Two), "t={elapsed} x={x}");
            }
        }
    }
}

#[test]
fn hsl_shift_rotates_hue_and_clamps_lightness() {
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(
        0,
        0,
        Style::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 255)),
        "ab",
        &mut clusters,
    );
    let shift = Effect::hsl_shift(120.0, 0.0, 0.0, ms(10), Ease::Linear);
    render(&shift, ms(10), &mut grid);
    let style = grid.get(0, 0).map(lunatui_core::Cell::style);
    assert_eq!(style.map(|s| s.fg), Some(Color::Rgb(0, 255, 0)));
    assert_eq!(style.map(|s| s.bg), Some(Color::Rgb(255, 0, 0)));
    let brighten = Effect::hsl_shift(0.0, 0.0, 1.0, ms(10), Ease::Linear);
    render(&brighten, ms(10), &mut grid);
    assert_eq!(grid.get(0, 0).map(|cell| cell.style().fg), Some(WHITE));
}

#[test]
fn sequence_waits_for_each_stage_and_keeps_finished_ones() {
    let fade = Effect::fade(BLACK, ms(100), Ease::Linear);
    let both = Effect::sequence(vec![
        fade.clone(),
        Effect::dissolve(3, ms(100), Ease::Linear),
    ]);
    assert_eq!(both.total(), ms(200));
    let mut alone = text_grid(20, 5, &"y".repeat(20));
    let mut staged = text_grid(20, 5, &"y".repeat(20));
    render(&fade, ms(99), &mut alone);
    render(&both, ms(99), &mut staged);
    assert_eq!(cells(&alone), cells(&staged));
    render(&both, ms(150), &mut staged);
    assert!(hidden(&staged, 0, 0));
    assert!(both.is_done(ms(200)));
}

#[test]
fn parallel_runs_together_and_lasts_as_long_as_the_longest() {
    let both = Effect::parallel(vec![
        Effect::fade(BLACK, ms(120), Ease::Linear),
        Effect::sweep(Direction::Down, 0.0, ms(120), Ease::Linear),
    ]);
    assert_eq!(both.total(), ms(120));
    let mut grid = text_grid(4, 4, "zzzz");
    render(&both, ms(60), &mut grid);
    assert!((0..4).all(|x| hidden(&grid, x, 0) && hidden(&grid, x, 1)));
    assert!(!hidden(&grid, 0, 3));
    assert_eq!(
        grid.get(0, 3).map(|cell| cell.style().fg),
        Some(Color::Rgb(99, 99, 99))
    );
}

#[test]
fn reversing_a_group_flips_every_member() {
    let out = Effect::parallel(vec![Effect::fade(BLACK, ms(10), Ease::Linear)]);
    let mut grid = text_grid(2, 1, "ab");
    let before = cells(&grid);
    render(&out.clone().reversed(), ms(10), &mut grid);
    assert_eq!(cells(&grid), before);
    render(&out.reversed(), Duration::ZERO, &mut grid);
    assert!(hidden(&grid, 0, 0));
}

#[test]
fn reduced_motion_jumps_to_the_end_state() {
    let fade = Effect::fade(BLACK, ms(100), Ease::Linear).delayed(ms(50));
    let reduced = fade.clone().with_policy(MotionPolicy::new(true));
    assert_eq!(reduced.total(), Duration::ZERO);
    let mut grid = text_grid(2, 1, "ab");
    render(&reduced, Duration::ZERO, &mut grid);
    assert!(hidden(&grid, 0, 0));
    assert_eq!(fade.clone().with_policy(MotionPolicy::new(false)), fade);
}

#[test]
fn only_the_requested_area_changes_and_oversize_areas_clip() {
    let fade = Effect::fade(BLACK, ms(10), Ease::Linear);
    let mut grid = text_grid(6, 3, "abcdef");
    let before = cells(&grid);
    fade.render(ms(10), &mut grid, Rect::new(2, 1, 100, 100), &palette());
    let after = cells(&grid);
    for y in 0..3_u16 {
        for x in 0..6_u16 {
            let index = usize::from(y) * 6 + usize::from(x);
            let inside = x >= 2 && y >= 1;
            assert_eq!(before[index] != after[index], inside, "({x},{y})");
        }
    }
    fade.render(ms(10), &mut grid, Rect::new(9, 9, 3, 3), &palette());
    assert_eq!(cells(&grid), after);
}

#[test]
fn slide_distance_scales_with_strength() {
    let slide = Effect::slide(Direction::Left, ms(100), Ease::Linear);
    let clusters = ClusterTable::new();
    for (elapsed, shown) in [(30, "DEFGHIJ"), (80, "IJ")] {
        let mut grid = text_grid(10, 1, "ABCDEFGHIJ");
        render(&slide, ms(elapsed), &mut grid);
        let row = grid.row_display(0, &clusters);
        assert_eq!(&row[..shown.len()], shown, "t={elapsed}");
        assert!(
            row[shown.len()..].chars().all(|ch| ch == ' '),
            "t={elapsed}"
        );
    }
}

#[test]
fn hsl_shift_scales_with_strength() {
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    grid.put_str(
        0,
        0,
        Style::new().fg(Color::Rgb(255, 0, 0)).bg(BLACK),
        "a",
        &mut clusters,
    );
    let shift = Effect::hsl_shift(120.0, 0.0, 0.0, ms(10), Ease::Linear);
    render(&shift, ms(5), &mut grid);
    assert_eq!(
        grid.get(0, 0).map(lunatui_core::Cell::style).map(|s| s.fg),
        Some(Color::Rgb(255, 255, 0))
    );
}

#[test]
fn lightness_and_saturation_shifts_scale_with_strength() {
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    let gray = Color::Rgb(100, 100, 100);
    grid.put_str(0, 0, Style::new().fg(gray).bg(BLACK), "a", &mut clusters);
    let brighten = Effect::hsl_shift(0.0, 0.0, 0.1, ms(10), Ease::Linear);
    render(&brighten, ms(5), &mut grid);
    let fg = grid.get(0, 0).map(lunatui_core::Cell::style).map(|s| s.fg);
    assert_eq!(fg, Some(Color::Rgb(113, 113, 113)));
    let mut tinted = Grid::new(1, 1);
    let muted = Color::Rgb(150, 100, 100);
    tinted.put_str(0, 0, Style::new().fg(muted).bg(BLACK), "a", &mut clusters);
    let desaturate = Effect::hsl_shift(0.0, -0.1, 0.0, ms(10), Ease::Linear);
    render(&desaturate, ms(5), &mut tinted);
    let fg = tinted
        .get(0, 0)
        .map(lunatui_core::Cell::style)
        .map(|s| s.fg);
    assert_eq!(fg, Some(Color::Rgb(144, 106, 106)));
}
