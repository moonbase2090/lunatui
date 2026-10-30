// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Button, ButtonState, ButtonVariant, RenderCx, Widget, WidgetState};

fn draw(button: &Button<'_>, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let dots = ".".repeat(usize::from(width));
    for y in 0..height {
        cx.put_str(0, y, &dots, Style::new().fg(Color::Rgb(200, 200, 200)));
    }
    button.render(area, &mut cx);
    (grid, clusters)
}

fn rows(button: &Button<'_>, area: Rect, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(button, area, width, height);
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

fn label_style(button: &Button<'_>) -> Style {
    let (grid, _) = draw(button, Rect::new(0, 0, 8, 1), 8, 1);
    style_at(&grid, 2, 0)
}

#[test]
fn centres_the_label_and_fills_the_area() {
    let save = Button::new("Save");
    assert_eq!(rows(&save, Rect::new(1, 0, 8, 1), 10, 1), [".  Save  ."]);
    assert_eq!(rows(&save, Rect::new(0, 0, 9, 1), 10, 1), ["  Save   ."]);
    let (grid, _) = draw(&save, Rect::new(1, 0, 8, 1), 10, 1);
    assert_eq!(style_at(&grid, 1, 0).bg, token(Token::Surface1));
    assert_eq!(style_at(&grid, 8, 0).bg, token(Token::Surface1));
    assert_ne!(style_at(&grid, 9, 0).bg, token(Token::Surface1));
}

#[test]
fn draws_the_label_on_the_middle_row_of_a_tall_button() {
    let save = Button::new("Save");
    assert_eq!(
        rows(&save, Rect::new(0, 0, 8, 3), 8, 4),
        ["        ", "  Save  ", "        ", "........"]
    );
    assert_eq!(
        rows(&save, Rect::new(0, 0, 8, 2), 8, 2),
        ["  Save  ", "        "]
    );
    let (grid, _) = draw(&save, Rect::new(0, 0, 8, 3), 8, 4);
    assert_eq!(style_at(&grid, 0, 2).bg, token(Token::Surface1));
}

#[test]
fn cuts_the_label_to_a_narrow_area_and_skips_an_empty_one() {
    let deploy = Button::new("Deploy");
    assert_eq!(rows(&deploy, Rect::new(1, 0, 4, 1), 6, 1), [".Depl."]);
    assert_eq!(rows(&deploy, Rect::new(1, 0, 7, 1), 9, 1), [".Deploy ."]);
    assert_eq!(rows(&deploy, Rect::new(1, 0, 0, 1), 4, 1), ["...."]);
    assert_eq!(rows(&deploy, Rect::new(1, 0, 4, 0), 4, 1), ["...."]);
}

#[test]
fn measures_the_label_with_padding_on_each_side() {
    let save = Button::new("Save");
    assert_eq!(save.measure(Size::new(40, 5)), Size::new(8, 1));
    assert_eq!(
        save.clone().padding(1).measure(Size::new(40, 5)),
        Size::new(6, 1)
    );
    assert_eq!(
        save.clone().padding(0).measure(Size::new(40, 5)),
        Size::new(4, 1)
    );
    assert_eq!(save.measure(Size::new(5, 5)), Size::new(5, 1));
    assert_eq!(save.measure(Size::new(5, 0)), Size::new(5, 0));
    assert_eq!(
        Button::new("Grüße").measure(Size::new(40, 1)),
        Size::new(9, 1)
    );
    assert_eq!(save.label(), "Save");
}

#[test]
fn rest_colors_follow_the_variant() {
    let normal = label_style(&Button::new("Save"));
    assert_eq!(
        (normal.fg, normal.bg),
        (token(Token::TextPrimary), token(Token::Surface1))
    );
    assert!(!normal.attrs.bold());

    let primary = label_style(&Button::new("Save").variant(ButtonVariant::Primary));
    assert_eq!(
        (primary.fg, primary.bg),
        (token(Token::Surface0), token(Token::Accent))
    );
    assert!(primary.attrs.bold());

    let destructive = label_style(&Button::new("Save").variant(ButtonVariant::Destructive));
    assert_eq!(
        (destructive.fg, destructive.bg),
        (token(Token::Fail), token(Token::Surface1))
    );
    assert!(!destructive.attrs.bold());
}

#[test]
fn focus_fills_the_button_with_the_focus_ring_or_fail() {
    for variant in [ButtonVariant::Normal, ButtonVariant::Primary] {
        let focused = label_style(
            &Button::new("Save")
                .variant(variant)
                .state(WidgetState::Focused),
        );
        assert_eq!(
            (focused.fg, focused.bg),
            (token(Token::Surface0), token(Token::FocusRing))
        );
        assert!(focused.attrs.bold());
    }
    let destructive = label_style(
        &Button::new("Save")
            .variant(ButtonVariant::Destructive)
            .state(WidgetState::Focused),
    );
    assert_eq!(
        (destructive.fg, destructive.bg),
        (token(Token::Surface0), token(Token::Fail))
    );
    assert!(destructive.attrs.bold());
}

#[test]
fn a_pressed_button_sinks_to_the_base_surface() {
    for (variant, tone) in [
        (ButtonVariant::Normal, Token::FocusRing),
        (ButtonVariant::Primary, Token::Accent),
        (ButtonVariant::Destructive, Token::Fail),
    ] {
        for state in [WidgetState::Normal, WidgetState::Focused] {
            let pressed = label_style(
                &Button::new("Save")
                    .variant(variant)
                    .state(state)
                    .pressed(true),
            );
            assert_eq!(
                (pressed.fg, pressed.bg),
                (token(tone), token(Token::Surface0))
            );
            assert!(pressed.attrs.bold());
        }
    }
}

#[test]
fn a_disabled_button_is_dimmed_and_cannot_be_pressed_or_hit() {
    for variant in [
        ButtonVariant::Normal,
        ButtonVariant::Primary,
        ButtonVariant::Destructive,
    ] {
        let disabled = Button::new("Save")
            .variant(variant)
            .state(WidgetState::Disabled)
            .pressed(true);
        let style = label_style(&disabled);
        assert_eq!(
            (style.fg, style.bg),
            (token(Token::TextPrimary), token(Token::Surface1))
        );
        assert!(style.attrs.dim());
        assert!(!style.attrs.bold());
        assert_eq!(
            disabled.hit_area(Rect::new(3, 2, 8, 1)),
            Rect::new(3, 2, 0, 0)
        );
    }
}

#[test]
fn an_explicit_style_replaces_the_theme() {
    let style = Style::new()
        .fg(Color::Rgb(1, 2, 3))
        .bg(Color::Rgb(4, 5, 6))
        .with_attrs(Attrs::new().set_italic(true));
    for state in [
        WidgetState::Normal,
        WidgetState::Focused,
        WidgetState::Disabled,
    ] {
        let drawn = label_style(&Button::new("Save").state(state).pressed(true).style(style));
        assert_eq!(drawn, style);
    }
}

#[test]
fn hit_area_is_the_drawn_area() {
    let save = Button::new("Save");
    assert_eq!(save.hit_area(Rect::new(3, 2, 8, 1)), Rect::new(3, 2, 8, 1));
    assert_eq!(
        save.clone()
            .state(WidgetState::Focused)
            .hit_area(Rect::new(3, 2, 8, 3)),
        Rect::new(3, 2, 8, 3)
    );
    assert_eq!(save.hit_area(Rect::new(3, 2, 0, 1)), Rect::new(3, 2, 0, 0));
    assert_eq!(save.hit_area(Rect::new(3, 2, 8, 0)), Rect::new(3, 2, 0, 0));
}

#[test]
fn keyboard_press_and_release_click_once() {
    let mut state = ButtonState::new();
    assert!(!state.is_pressed());
    assert!(!state.release());
    state.press();
    assert!(state.is_pressed());
    assert!(state.release());
    assert!(!state.is_pressed());
    assert!(!state.release());
    state.press();
    state.cancel();
    assert!(!state.is_pressed());
    assert!(!state.release());
}

#[test]
fn a_pointer_clicks_only_when_it_is_released_on_the_button() {
    let hit = Rect::new(4, 2, 8, 1);
    let mut state = ButtonState::new();
    assert!(!state.pointer_down(3, 2, hit));
    assert!(!state.pointer_down(12, 2, hit));
    assert!(!state.pointer_down(4, 3, hit));
    assert!(!state.pointer_down(4, 1, hit));
    assert!(!state.is_pressed());
    assert!(!state.pointer_up(5, 2, hit));

    assert!(state.pointer_down(4, 2, hit));
    assert!(state.is_pressed());
    assert!(state.pointer_up(11, 2, hit));
    assert!(!state.is_pressed());

    assert!(state.pointer_down(11, 2, hit));
    assert!(!state.pointer_up(12, 2, hit));
    assert!(!state.is_pressed());

    assert!(state.pointer_down(6, 2, hit));
    assert!(!state.pointer_down(20, 2, hit));
    assert!(!state.is_pressed());

    let disabled = Button::new("Save").state(WidgetState::Disabled);
    assert!(!state.pointer_down(6, 2, disabled.hit_area(hit)));
}

#[test]
fn padding_cells_carry_the_label_attributes() {
    let primary = Button::new("Save").variant(ButtonVariant::Primary);
    let (grid, _) = draw(&primary, Rect::new(0, 0, 8, 2), 8, 2);
    for (x, y) in [(0, 0), (7, 0), (3, 1)] {
        assert!(style_at(&grid, x, y).attrs.bold(), "({x},{y})");
    }
    let disabled = Button::new("Save").state(WidgetState::Disabled);
    let (grid, _) = draw(&disabled, Rect::new(0, 0, 8, 1), 8, 1);
    assert!(style_at(&grid, 0, 0).attrs.dim());
}
