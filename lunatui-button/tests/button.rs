// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_button::{Button, ButtonSize, ButtonState, Toggle, Variant, WidgetState};
use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_theme::{Theme, Variant as ThemeVariant};
use lunatui_ui_tokens::{Role, Tone};
use lunatui_widgets::{RenderCx, Widget};

fn draw(w: u16, widget: &dyn Widget) -> (Grid, ClusterTable, Theme) {
    let theme = Theme::builtin(ThemeVariant::Dark);
    let mut grid = Grid::new(w, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, w, 1), &mut cx);
    (grid, clusters, theme)
}

#[test]
fn sizes_set_the_padding() {
    let avail = Size::new(40, 3);
    let width = |size| Button::new("Go").size(size).measure(avail).width;
    assert_eq!(width(ButtonSize::Small), 4);
    assert_eq!(width(ButtonSize::Medium), 6);
    assert_eq!(width(ButtonSize::Large), 10);
    assert_eq!(Button::new("Go").measure(Size::new(3, 1)).width, 3);
}

#[test]
fn label_is_centred_and_clipped() {
    let (grid, clusters, _) = draw(8, &Button::new("Go"));
    assert_eq!(grid.row_display(0, &clusters), "   Go   ");
    let (grid, clusters, _) = draw(4, &Button::new("Deploy"));
    assert_eq!(grid.row_display(0, &clusters), "Depl");
    let (grid, clusters, _) = draw(0, &Button::new("x"));
    assert_eq!(grid.display_text(&clusters).trim(), "");
}

#[test]
fn variants_use_token_fills() -> Result<(), &'static str> {
    let bg = |variant| -> Result<_, &'static str> {
        let (grid, _, theme) = draw(8, &Button::new("Go").variant(variant));
        let cell = grid.get(3, 0).ok_or("missing cell")?;
        Ok((cell.style().bg, theme))
    };
    let (fill, theme) = bg(Variant::Default)?;
    assert_eq!(fill, theme.resolve(Tone::Primary.fill()));
    let (fill, theme) = bg(Variant::Destructive)?;
    assert_eq!(fill, theme.resolve(Tone::Danger.fill()));
    let (fill, theme) = bg(Variant::Secondary)?;
    assert_eq!(fill, theme.resolve(Role::Card.token()));
    let (fill, theme) = bg(Variant::Ghost)?;
    assert_eq!(fill, theme.resolve(Role::Background.token()));
    Ok(())
}

#[test]
fn outline_draws_brackets() {
    let (grid, clusters, _) = draw(8, &Button::new("Go").variant(Variant::Outline));
    assert_eq!(grid.row_display(0, &clusters), "[  Go  ]");
}

#[test]
fn focus_and_press_change_the_fill() -> Result<(), &'static str> {
    let fill = |button: Button<'_>| -> Result<_, &'static str> {
        let (grid, _, theme) = draw(8, &button);
        let cell = grid.get(3, 0).ok_or("missing cell")?;
        Ok((cell.style().bg, theme))
    };
    let (focused, theme) = fill(Button::new("Go").state(WidgetState::Focused))?;
    assert_eq!(focused, theme.resolve(Role::Ring.token()));
    let (pressed, theme) = fill(Button::new("Go").pressed(true))?;
    assert_eq!(pressed, theme.resolve(Role::Background.token()));
    let (resting, _) = fill(Button::new("Go"))?;
    assert_ne!(resting, focused);
    assert_ne!(resting, pressed);
    Ok(())
}

#[test]
fn disabled_button_has_no_hit_area_and_dims() -> Result<(), &'static str> {
    let area = Rect::new(2, 1, 8, 1);
    let live = Button::new("Go");
    let off = Button::new("Go").state(WidgetState::Disabled);
    assert_eq!(live.hit_area(area), area);
    assert_eq!(off.hit_area(area).width, 0);
    let (grid, _, _) = draw(8, &off);
    let cell = grid.get(3, 0).ok_or("missing cell")?;
    assert!(cell.style().attrs.dim());
    Ok(())
}

#[test]
fn disabled_button_is_muted_without_its_variant_fill() -> Result<(), &'static str> {
    let theme = Theme::builtin(ThemeVariant::Dark);
    for variant in [Variant::Default, Variant::Destructive, Variant::Secondary] {
        let live = Button::new("Go").variant(variant).on(Role::Card);
        let off = live.state(WidgetState::Disabled);
        let (grid, _, _) = draw(8, &off);
        let cell = grid.get(3, 0).ok_or("missing cell")?;
        assert_eq!(cell.style().fg, theme.resolve(Role::Muted.token()));
        assert_eq!(cell.style().bg, theme.resolve(Role::Card.token()));
        let (grid, _, _) = draw(8, &live);
        let on = grid.get(3, 0).ok_or("missing cell")?;
        assert_ne!(cell.style().bg, on.style().bg, "{variant:?} kept its fill");
    }
    Ok(())
}

#[test]
fn state_turns_a_press_into_a_click() {
    let hit = Button::new("Go").hit_area(Rect::new(0, 0, 8, 1));
    let mut state = ButtonState::new();
    assert!(state.pointer_down(2, 0, hit));
    assert!(state.pointer_up(2, 0, hit));
    assert!(state.pointer_down(2, 0, hit));
    assert!(!state.pointer_up(20, 0, hit));
}

#[test]
fn toggle_fills_only_when_on() -> Result<(), &'static str> {
    let fill = |on| -> Result<_, &'static str> {
        let (grid, _, theme) = draw(10, &Toggle::new("dry", on));
        let cell = grid.get(4, 0).ok_or("missing cell")?;
        Ok((cell.style().bg, theme))
    };
    let (on, theme) = fill(true)?;
    let (off, _) = fill(false)?;
    assert_eq!(on, theme.resolve(Tone::Primary.fill()));
    assert_eq!(off, theme.resolve(Role::Card.token()));
    assert!(Toggle::new("dry", true).is_on());
    assert_eq!(Toggle::new("dry", true).measure(Size::new(30, 1)).width, 7);
    Ok(())
}

#[test]
fn on_card_secondary_insets_and_ghost_blends() -> Result<(), &'static str> {
    let fill = |button: Button<'_>| -> Result<_, &'static str> {
        let (grid, _, theme) = draw(8, &button.on(Role::Card));
        let cell = grid.get(3, 0).ok_or("missing cell")?;
        Ok((cell.style().bg, theme))
    };
    let (secondary, theme) = fill(Button::new("Go").variant(Variant::Secondary))?;
    assert_eq!(secondary, theme.resolve(Role::Background.token()));
    let (ghost, theme) = fill(Button::new("Go").variant(Variant::Ghost))?;
    assert_eq!(ghost, theme.resolve(Role::Card.token()));
    Ok(())
}
