// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_choice::{CheckState, Checkbox, Radio, Switch, WidgetState};
use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::Role;
use lunatui_widgets::{RadioFlow, RenderCx, Widget};

fn draw(w: u16, h: u16, widget: &dyn Widget) -> (Grid, ClusterTable, Theme) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(w, h);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, w, h), &mut cx);
    (grid, clusters, theme)
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Result<Style, &'static str> {
    grid.get(x, y)
        .map(lunatui_core::Cell::style)
        .ok_or("missing cell")
}

#[test]
fn checkbox_glyph_follows_the_check_state() {
    for (state, row) in [
        (CheckState::Unchecked, "[ ] Deploys "),
        (CheckState::Checked, "[x] Deploys "),
        (CheckState::Mixed, "[-] Deploys "),
    ] {
        let (grid, clusters, _) = draw(12, 1, &Checkbox::new("Deploys").checked(state));
        assert_eq!(grid.row_display(0, &clusters), row);
    }
}

#[test]
fn a_click_checks_unchecked_and_mixed_and_unchecks_checked() {
    assert_eq!(CheckState::Unchecked.toggled(), CheckState::Checked);
    assert_eq!(CheckState::Mixed.toggled(), CheckState::Checked);
    assert_eq!(CheckState::Checked.toggled(), CheckState::Unchecked);
}

#[test]
fn checked_box_is_primary_and_unchecked_is_the_border_color() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(
        12,
        1,
        &Checkbox::new("Deploys").checked(CheckState::Checked),
    );
    assert_eq!(
        style_at(&grid, 1, 0)?.fg,
        theme.resolve(Role::Primary.token())
    );
    assert!(style_at(&grid, 1, 0)?.attrs.bold());
    let (grid, _, theme) = draw(12, 1, &Checkbox::new("Deploys"));
    assert_eq!(
        style_at(&grid, 1, 0)?.fg,
        theme.resolve(Role::Border.token())
    );
    assert!(!style_at(&grid, 1, 0)?.attrs.bold());
    let (grid, _, theme) = draw(12, 1, &Checkbox::new("Deploys").checked(CheckState::Mixed));
    assert_eq!(
        style_at(&grid, 1, 0)?.fg,
        theme.resolve(Role::Primary.token())
    );
    Ok(())
}

#[test]
fn focus_draws_the_ring_and_a_bold_label() -> Result<(), &'static str> {
    let focused = Checkbox::new("Deploys").state(WidgetState::Focused);
    let (grid, _, theme) = draw(12, 1, &focused);
    assert_eq!(style_at(&grid, 1, 0)?.fg, theme.resolve(Role::Ring.token()));
    assert!(style_at(&grid, 1, 0)?.attrs.bold());
    assert!(style_at(&grid, 5, 0)?.attrs.bold());
    let (grid, _, _) = draw(12, 1, &Checkbox::new("Deploys"));
    assert!(!style_at(&grid, 5, 0)?.attrs.bold());
    Ok(())
}

#[test]
fn disabled_dims_the_box_and_the_label_and_takes_no_clicks() -> Result<(), &'static str> {
    let off = Checkbox::new("Deploys").state(WidgetState::Disabled);
    let (grid, _, theme) = draw(12, 1, &off);
    assert!(style_at(&grid, 1, 0)?.attrs.dim());
    assert!(style_at(&grid, 5, 0)?.attrs.dim());
    assert_eq!(
        style_at(&grid, 5, 0)?.fg,
        theme.resolve(Role::Muted.token())
    );
    let area = Rect::new(2, 1, 12, 1);
    assert_eq!(off.hit_area(area).width, 0);
    assert_eq!(Checkbox::new("Deploys").hit_area(area), area);
    Ok(())
}

#[test]
fn switch_knob_sits_at_the_end_for_on_and_the_start_for_off() -> Result<(), &'static str> {
    let (grid, clusters, _) = draw(14, 1, &Switch::new("Slack", true));
    assert_eq!(grid.row_display(0, &clusters), "━━● Slack     ");
    let (grid, clusters, _) = draw(14, 1, &Switch::new("Slack", false));
    assert_eq!(grid.row_display(0, &clusters), "●━━ Slack     ");
    let (on, _, theme) = draw(14, 1, &Switch::new("Slack", true));
    assert_eq!(
        style_at(&on, 0, 0)?.fg,
        theme.resolve(Role::Primary.token())
    );
    let (off, _, theme) = draw(14, 1, &Switch::new("Slack", false));
    assert_eq!(
        style_at(&off, 0, 0)?.fg,
        theme.resolve(Role::Border.token())
    );
    assert!(Switch::new("Slack", true).is_on());
    assert!(!Switch::new("Slack", false).is_on());
    Ok(())
}

#[test]
fn switch_states_match_the_checkbox() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(
        14,
        1,
        &Switch::new("Slack", false).state(WidgetState::Focused),
    );
    assert_eq!(style_at(&grid, 0, 0)?.fg, theme.resolve(Role::Ring.token()));
    let off = Switch::new("Slack", true).state(WidgetState::Disabled);
    let (grid, _, _) = draw(14, 1, &off);
    assert!(style_at(&grid, 0, 0)?.attrs.dim());
    assert_eq!(off.hit_area(Rect::new(0, 0, 14, 1)).width, 0);
    Ok(())
}

#[test]
fn measure_is_glyph_gap_and_label() {
    let avail = Size::new(40, 4);
    assert_eq!(Checkbox::new("Deploys").measure(avail), Size::new(11, 1));
    assert_eq!(Switch::new("Slack", true).measure(avail), Size::new(9, 1));
    assert_eq!(
        Checkbox::new("Deploys").measure(Size::new(6, 1)),
        Size::new(6, 1)
    );
}

#[test]
fn label_is_clipped_and_tiny_areas_do_not_panic() {
    let (grid, clusters, _) = draw(8, 1, &Checkbox::new("Deploys"));
    assert_eq!(grid.row_display(0, &clusters), "[ ] Depl");
    let (grid, clusters, _) = draw(3, 1, &Switch::new("Slack", true));
    assert_eq!(grid.row_display(0, &clusters), "━━●");
    let (grid, clusters, _) = draw(0, 0, &Checkbox::new("Deploys"));
    assert_eq!(grid.display_text(&clusters).trim(), "");
}

#[test]
fn on_sets_the_surface_behind_a_choice() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(12, 1, &Checkbox::new("Deploys").on(Role::Card));
    assert_eq!(
        style_at(&grid, 11, 0)?.bg,
        theme.resolve(Role::Card.token())
    );
    let (grid, _, theme) = draw(12, 1, &Checkbox::new("Deploys"));
    assert_eq!(
        style_at(&grid, 11, 0)?.bg,
        theme.resolve(Role::Background.token())
    );
    let (grid, _, theme) = draw(12, 1, &Switch::new("Slack", true).on_surface(Role::Card));
    assert_eq!(
        style_at(&grid, 11, 0)?.bg,
        theme.resolve(Role::Card.token())
    );
    Ok(())
}

#[test]
fn radio_marks_the_selected_option_and_steps_over_disabled_ones() {
    let radio = Radio::new(["Now", "Hourly", "Daily"])
        .selected(Some(1))
        .disabled_options([2]);
    let (grid, clusters, _) = draw(10, 3, &radio);
    assert_eq!(grid.row_display(0, &clusters), "○ Now     ");
    assert_eq!(grid.row_display(1, &clusters), "◉ Hourly  ");
    assert_eq!(grid.row_display(2, &clusters), "○ Daily   ");
    assert_eq!(radio.step(Some(1), true), Some(0));
    assert_eq!(radio.step(Some(0), false), Some(1));
}

#[test]
fn radio_flows_sideways_and_reports_option_areas() {
    let radio = Radio::new(["A", "B"]).flow(RadioFlow::Horizontal).gap(2);
    let (grid, clusters, _) = draw(10, 1, &radio);
    assert_eq!(grid.row_display(0, &clusters), "○ A  ○ B  ");
    let areas = radio.option_areas(Rect::new(0, 0, 10, 1));
    assert_eq!(
        areas,
        vec![(0, Rect::new(0, 0, 3, 1)), (1, Rect::new(5, 0, 3, 1))]
    );
    assert_eq!(radio.measure(Size::new(10, 4)), Size::new(8, 1));
}

#[test]
fn radio_fills_its_area_with_the_chosen_surface() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(10, 2, &Radio::new(["A"]).on(Role::Card));
    assert_eq!(style_at(&grid, 9, 1)?.bg, theme.resolve(Role::Card.token()));
    assert_eq!(style_at(&grid, 0, 0)?.bg, theme.resolve(Role::Card.token()));
    Ok(())
}

#[test]
fn radio_glyphs_keep_the_surface_and_take_token_colors() -> Result<(), &'static str> {
    let radio = Radio::new(["Now", "Hourly", "Daily"])
        .selected(Some(1))
        .focused(Some(2))
        .disabled_options([0])
        .on(Role::Card);
    let (grid, _, theme) = draw(10, 3, &radio);
    for row in 0..3 {
        assert_eq!(
            style_at(&grid, 0, row)?.bg,
            theme.resolve(Role::Card.token())
        );
    }
    assert_eq!(
        style_at(&grid, 0, 1)?.fg,
        theme.resolve(Role::Primary.token())
    );
    assert_eq!(style_at(&grid, 0, 2)?.fg, theme.resolve(Role::Ring.token()));
    assert!(style_at(&grid, 2, 2)?.attrs.bold());
    assert!(style_at(&grid, 0, 0)?.attrs.dim());
    assert!(style_at(&grid, 2, 0)?.attrs.dim());
    assert!(!style_at(&grid, 2, 1)?.attrs.dim());
    Ok(())
}

#[test]
fn a_disabled_group_dims_every_option() -> Result<(), &'static str> {
    let radio = Radio::new(["A", "B"])
        .state(WidgetState::Disabled)
        .focused(Some(0));
    let (grid, _, _) = draw(6, 2, &radio);
    assert!(style_at(&grid, 0, 0)?.attrs.dim());
    assert!(style_at(&grid, 0, 1)?.attrs.dim());
    assert!(!style_at(&grid, 2, 0)?.attrs.bold());
    Ok(())
}
