// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_field::Field;
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::Role;
use lunatui_widgets::{Paragraph, RenderCx, Widget};

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
fn rows_run_label_control_help() {
    let input = Paragraph::new("ada@example.com");
    let field = Field::new("Email").help("We never share it").body(&input);
    let (grid, clusters, _) = draw(20, 4, &field);
    assert_eq!(grid.row_display(0, &clusters), "Email               ");
    assert_eq!(grid.row_display(1, &clusters), "ada@example.com     ");
    assert_eq!(grid.row_display(2, &clusters), "We never share it   ");
    assert_eq!(grid.row_display(3, &clusters), " ".repeat(20));
}

#[test]
fn error_replaces_help_in_the_destructive_color() -> Result<(), &'static str> {
    let input = Paragraph::new("ada");
    let field = Field::new("Email")
        .help("We never share it")
        .error("Not an address")
        .body(&input);
    let (grid, clusters, theme) = draw(20, 3, &field);
    assert_eq!(grid.row_display(2, &clusters), "Not an address      ");
    assert_eq!(
        style_at(&grid, 0, 2)?.fg,
        theme.resolve(Role::Destructive.token())
    );
    let calm = Field::new("Email").help("We never share it").body(&input);
    let (grid, _, theme) = draw(20, 3, &calm);
    assert_eq!(
        style_at(&grid, 0, 2)?.fg,
        theme.resolve(Role::Muted.token())
    );
    Ok(())
}

#[test]
fn required_marks_the_label_after_its_text() -> Result<(), &'static str> {
    let (grid, clusters, theme) = draw(10, 1, &Field::new("Name").required());
    assert_eq!(grid.row_display(0, &clusters), "Name *    ");
    assert_eq!(
        style_at(&grid, 5, 0)?.fg,
        theme.resolve(Role::Destructive.token())
    );
    assert_ne!(style_at(&grid, 0, 0)?.fg, style_at(&grid, 5, 0)?.fg);
    Ok(())
}

#[test]
fn focus_colors_the_label_with_the_ring() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(10, 1, &Field::new("Name").focused(true));
    assert_eq!(style_at(&grid, 0, 0)?.fg, theme.resolve(Role::Ring.token()));
    assert!(style_at(&grid, 0, 0)?.attrs.bold());
    let (grid, _, _) = draw(10, 1, &Field::new("Name"));
    assert_ne!(style_at(&grid, 0, 0)?.fg, theme.resolve(Role::Ring.token()));
    Ok(())
}

#[test]
fn disabled_dims_label_and_message_only() -> Result<(), &'static str> {
    let input = Paragraph::new("ada");
    let field = Field::new("Name")
        .help("Shown on invoices")
        .disabled(true)
        .body(&input);
    let (grid, _, _) = draw(20, 3, &field);
    assert!(style_at(&grid, 0, 0)?.attrs.dim());
    assert!(style_at(&grid, 0, 2)?.attrs.dim());
    assert!(!style_at(&grid, 0, 1)?.attrs.dim());
    let (grid, _, _) = draw(
        20,
        3,
        &Field::new("Name").help("Shown on invoices").body(&input),
    );
    assert!(!style_at(&grid, 0, 0)?.attrs.dim());
    Ok(())
}

#[test]
fn measure_counts_label_control_and_message_rows() {
    let avail = Size::new(40, 10);
    let input = Paragraph::new("ada@example.com");
    assert_eq!(Field::new("Name").measure(avail), Size::new(4, 1));
    assert_eq!(
        Field::new("Name").required().measure(avail),
        Size::new(6, 1)
    );
    let field = Field::new("Email").help("We never share it").body(&input);
    assert_eq!(field.measure(avail).height, 3);
    assert_eq!(field.measure(avail).width, 40);
    let bare = Field::new("Email").help("We never share it");
    assert_eq!(bare.measure(avail), Size::new(17, 2));
    assert_eq!(Field::new("Email").body(&input).measure(avail).height, 2);
    assert_eq!(field.measure(Size::new(8, 2)), Size::new(8, 2));
}

#[test]
fn small_areas_keep_the_label_then_the_control_then_the_message() {
    let input = Paragraph::new("ada");
    let field = Field::new("Email").help("hint").body(&input);
    let (grid, clusters, _) = draw(8, 1, &field);
    assert_eq!(grid.row_display(0, &clusters), "Email   ");
    let (grid, clusters, _) = draw(8, 2, &field);
    assert_eq!(grid.row_display(0, &clusters), "Email   ");
    assert_eq!(grid.row_display(1, &clusters), "ada     ");
    let (grid, clusters, _) = draw(3, 3, &field);
    assert_eq!(grid.row_display(0, &clusters), "Ema");
    assert_eq!(grid.row_display(2, &clusters), "hin");
    let (grid, clusters, _) = draw(0, 0, &field);
    assert_eq!(grid.display_text(&clusters).trim(), "");
}

#[test]
fn message_without_a_control_sits_under_the_label() {
    let field = Field::new("Name").error("Required");
    let (grid, clusters, _) = draw(10, 3, &field);
    assert_eq!(grid.row_display(0, &clusters), "Name      ");
    assert_eq!(grid.row_display(1, &clusters), "Required  ");
}

#[test]
fn on_sets_the_surface_behind_the_field() -> Result<(), &'static str> {
    let (grid, _, theme) = draw(10, 2, &Field::new("Name").on(Role::Card));
    assert_eq!(style_at(&grid, 9, 1)?.bg, theme.resolve(Role::Card.token()));
    let (grid, _, theme) = draw(10, 2, &Field::new("Name"));
    assert_eq!(
        style_at(&grid, 9, 1)?.bg,
        theme.resolve(Role::Background.token())
    );
    Ok(())
}
