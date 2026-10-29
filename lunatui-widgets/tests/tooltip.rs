// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use core::time::Duration;

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{AnchorSide, RenderCx, Tooltip, TooltipState, Widget};

fn draw(tooltip: &Tooltip, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    tooltip.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(tooltip: &Tooltip, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(tooltip, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

const SAVE: Rect = Rect::new(2, 1, 4, 1);

#[test]
fn opens_under_the_anchor_with_a_cell_of_padding_each_side() {
    let tooltip = Tooltip::new("Save changes", SAVE);
    assert_eq!(
        tooltip.area(Rect::new(0, 0, 20, 6)),
        Some(Rect::new(2, 2, 14, 1))
    );
    assert_eq!(
        rows(&tooltip, 20, 4),
        [
            "                    ",
            "                    ",
            "   Save changes     ",
            "                    ",
        ]
    );
}

#[test]
fn takes_its_colors_from_theme_tokens() {
    let (grid, _) = draw(&Tooltip::new("Save changes", SAVE), 20, 4);
    for x in [2, 3, 15] {
        assert_eq!(style_at(&grid, x, 2).bg, token(Token::Surface1), "cell {x}");
    }
    assert_eq!(style_at(&grid, 3, 2).fg, token(Token::TextPrimary));
    assert_ne!(
        style_at(&grid, 16, 2).bg,
        token(Token::Surface1),
        "past the bubble"
    );
    assert_ne!(
        style_at(&grid, 2, 1).bg,
        token(Token::Surface1),
        "the anchor row"
    );
}

#[test]
fn an_explicit_style_replaces_the_tokens() {
    let style = Style::new().fg(Color::Rgb(1, 2, 3)).bg(Color::Rgb(9, 8, 7));
    let (grid, _) = draw(&Tooltip::new("Save", SAVE).style(style), 20, 4);
    assert_eq!(style_at(&grid, 2, 2), style);
    assert_eq!(style_at(&grid, 4, 2), style);
}

#[test]
fn flips_above_when_there_is_no_room_below() {
    let anchor = Rect::new(2, 5, 4, 1);
    let tooltip = Tooltip::new("Save changes", anchor);
    assert_eq!(
        tooltip.area(Rect::new(0, 0, 20, 6)),
        Some(Rect::new(2, 4, 14, 1))
    );
    assert_eq!(rows(&tooltip, 20, 6)[4], "   Save changes     ");
}

#[test]
fn prefers_above_when_asked_and_falls_back_below() {
    let tooltip = Tooltip::new("Hi", Rect::new(2, 2, 4, 1)).side(AnchorSide::Above);
    assert_eq!(
        tooltip.area(Rect::new(0, 0, 20, 6)),
        Some(Rect::new(2, 1, 4, 1))
    );
    let top = Tooltip::new("Hi", Rect::new(2, 0, 4, 1)).side(AnchorSide::Above);
    assert_eq!(
        top.area(Rect::new(0, 0, 20, 6)),
        Some(Rect::new(2, 1, 4, 1))
    );
}

#[test]
fn goes_right_then_left_when_neither_row_has_room() {
    let area = Rect::new(0, 0, 20, 3);
    let right = Tooltip::new("Save\nnow", SAVE);
    assert_eq!(right.area(area), Some(Rect::new(6, 1, 6, 2)));
    assert_eq!(
        rows(&right, 20, 3),
        [
            "                    ",
            "       Save         ",
            "       now          ",
        ]
    );
    let left = Tooltip::new("Save\nnow", Rect::new(14, 1, 4, 1));
    assert_eq!(left.area(area), Some(Rect::new(8, 1, 6, 2)));
}

#[test]
fn a_side_first_tooltip_slides_along_the_anchor_to_stay_inside() {
    let tooltip = Tooltip::new("A\nB\nC", Rect::new(10, 3, 2, 1)).side(AnchorSide::Left);
    assert_eq!(
        tooltip.area(Rect::new(0, 0, 20, 4)),
        Some(Rect::new(7, 1, 3, 3))
    );
    let right = Tooltip::new("A", Rect::new(10, 3, 2, 1)).side(AnchorSide::Right);
    assert_eq!(
        right.area(Rect::new(0, 0, 20, 4)),
        Some(Rect::new(12, 3, 3, 1))
    );
}

#[test]
fn slides_left_to_stay_inside_the_area() {
    let tooltip = Tooltip::new("Save changes", Rect::new(16, 1, 4, 1));
    assert_eq!(
        tooltip.area(Rect::new(0, 0, 20, 6)),
        Some(Rect::new(6, 2, 14, 1))
    );
}

#[test]
fn is_not_drawn_when_no_side_has_room() {
    let tooltip = Tooltip::new("Save\nnow", Rect::new(1, 1, 6, 1));
    assert_eq!(tooltip.area(Rect::new(0, 0, 8, 3)), None);
    assert_eq!(rows(&tooltip, 8, 3), ["        ", "        ", "        "]);
    assert_eq!(Tooltip::new("", SAVE).area(Rect::new(0, 0, 20, 6)), None);
}

#[test]
fn never_covers_its_anchor_and_stays_inside_the_area() {
    let area = Rect::new(0, 0, 20, 6);
    let sides = [
        AnchorSide::Below,
        AnchorSide::Above,
        AnchorSide::Right,
        AnchorSide::Left,
    ];
    let mut drawn = 0;
    for side in sides {
        for y in 0..6 {
            for x in 0..20 {
                for (width, height) in [(1, 1), (4, 1), (3, 2), (8, 3)] {
                    let anchor = Rect::new(x, y, width, height);
                    let tooltip = Tooltip::new("Hi there", anchor).side(side);
                    let Some(bubble) = tooltip.area(area) else {
                        continue;
                    };
                    drawn += 1;
                    assert!(
                        !overlaps(bubble, anchor),
                        "{side:?} {anchor:?} -> {bubble:?}"
                    );
                    assert!(bubble.x + bubble.width <= 20 && bubble.y + bubble.height <= 6);
                }
            }
        }
    }
    assert!(drawn > 1500, "only {drawn} placements fit");
}

#[test]
fn wraps_at_spaces_and_is_as_wide_as_its_widest_row() {
    let tooltip = Tooltip::new("Saves every open file now", Rect::new(0, 0, 4, 1)).max_width(12);
    assert_eq!(
        tooltip.area(Rect::new(0, 0, 20, 6)),
        Some(Rect::new(0, 1, 12, 3))
    );
    assert_eq!(
        &rows(&tooltip, 20, 4)[1..],
        [
            " Saves              ",
            " every open         ",
            " file now           ",
        ]
    );
}

#[test]
fn a_row_may_fill_the_width_exactly() {
    let tooltip = Tooltip::new("ab cd efg", Rect::new(0, 0, 4, 1)).max_width(7);
    assert_eq!(&rows(&tooltip, 10, 3)[1..], [" ab cd    ", " efg      "]);
}

#[test]
fn breaks_a_word_too_long_for_a_row() {
    let tooltip = Tooltip::new("abcdefgh", Rect::new(0, 0, 4, 1)).max_width(6);
    assert_eq!(&rows(&tooltip, 10, 3)[1..], [" abcd     ", " efgh     "]);
}

#[test]
fn measures_its_wrapped_size() {
    let tooltip = Tooltip::new("Save changes", SAVE);
    assert_eq!(tooltip.measure(Size::new(30, 5)), Size::new(14, 1));
    assert_eq!(tooltip.measure(Size::new(6, 5)), Size::new(6, 3));
    assert_eq!(tooltip.measure(Size::new(6, 2)), Size::new(0, 0));
    assert_eq!(
        tooltip.measure(Size::new(2, 20)),
        Size::new(0, 0),
        "no room inside the padding"
    );
    let leading = Tooltip::new(" abcdef", SAVE).max_width(6);
    assert_eq!(
        leading.measure(Size::new(30, 5)),
        Size::new(6, 2),
        "no empty first row"
    );
    assert_eq!(
        tooltip.max_width(1).measure(Size::new(30, 20)),
        Size::new(3, 11)
    );
}

const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn shows_after_the_pointer_rests_on_the_anchor_for_the_delay() {
    let mut state = TooltipState::new();
    assert_eq!(state, TooltipState::default());
    assert!(!state.is_shown(ms(0)));
    state.hover(true, ms(1000));
    assert_eq!(state.shows_at(), Some(ms(1500)));
    assert!(!state.is_shown(ms(1499)));
    assert!(state.is_shown(ms(1500)));
    state.hover(true, ms(1600));
    assert_eq!(
        state.shows_at(),
        Some(ms(1500)),
        "resting does not restart the delay"
    );
    state.hover(false, ms(2000));
    assert!(!state.is_shown(ms(2000)));
    assert_eq!(state.shows_at(), None);
    state.hover(true, ms(3000));
    assert!(!state.is_shown(ms(3400)));
    assert!(state.is_shown(ms(3500)));
}

#[test]
fn a_dismissed_tooltip_stays_hidden_until_the_pointer_comes_back() {
    let mut state = TooltipState::with_delay(ms(200));
    state.hover(true, ms(0));
    assert!(state.is_shown(ms(200)));
    state.dismiss();
    assert!(!state.is_shown(ms(300)));
    assert_eq!(state.shows_at(), None);
    state.hover(true, ms(400));
    assert!(!state.is_shown(ms(1000)), "still on the anchor");
    state.hover(false, ms(1100));
    state.hover(true, ms(1200));
    assert!(!state.is_shown(ms(1399)));
    assert!(state.is_shown(ms(1400)));
}

#[test]
fn a_zero_delay_shows_at_once() {
    let mut state = TooltipState::with_delay(Duration::ZERO);
    state.hover(true, ms(50));
    assert!(state.is_shown(ms(50)));
}
