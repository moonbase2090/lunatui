// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{FieldArea, Form, FormField, RenderCx, Widget};

fn fields() -> Vec<FormField> {
    vec![
        FormField::new("Name").required(),
        FormField::new("Email").required().hint("We never share it"),
        FormField::new("Notes"),
    ]
}

fn form() -> Form {
    Form::new(fields())
}

fn draw(form: &Form, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    form.render(area, &mut cx);
    (grid, clusters)
}

fn rows(form: &Form, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(form, Rect::new(0, 0, width, height), width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn parts(area: &FieldArea) -> [Rect; 4] {
    [area.row, area.label, area.input, area.message]
}

#[test]
fn each_field_gets_a_label_an_input_and_a_message_row() {
    let areas = form().layout(Rect::new(0, 0, 30, 12));
    assert_eq!(
        parts(&areas[0]),
        [
            Rect::new(0, 0, 30, 2),
            Rect::new(0, 0, 7, 1),
            Rect::new(9, 0, 21, 1),
            Rect::new(9, 1, 21, 1),
        ]
    );
    assert_eq!(areas[1].index, 1);
    assert_eq!(parts(&areas[1])[0], Rect::new(0, 3, 30, 2));
    assert_eq!(parts(&areas[1])[3], Rect::new(9, 4, 21, 1));
    assert_eq!(areas[2].index, 2);
    assert_eq!(parts(&areas[2])[2], Rect::new(9, 6, 21, 1));
    assert_eq!(areas.len(), 3);
}

#[test]
fn the_area_offset_shifts_every_part() {
    let areas = form().layout(Rect::new(4, 2, 30, 12));
    assert_eq!(
        parts(&areas[0]),
        [
            Rect::new(4, 2, 30, 2),
            Rect::new(4, 2, 7, 1),
            Rect::new(13, 2, 21, 1),
            Rect::new(13, 3, 21, 1),
        ]
    );
    assert_eq!(parts(&areas[1])[2], Rect::new(13, 5, 21, 1));
}

#[test]
fn the_gap_and_taller_inputs_change_the_rows() {
    let tight = form().gap(0).layout(Rect::new(0, 0, 30, 12));
    assert_eq!(parts(&tight[1])[0].y, 2);
    assert_eq!(parts(&tight[2])[0].y, 4);
    let loose = form().gap(2).layout(Rect::new(0, 0, 30, 12));
    assert_eq!(parts(&loose[1])[0].y, 4);
    let tall = Form::new([FormField::new("Bio").input_rows(3), FormField::new("Name")]);
    let areas = tall.layout(Rect::new(0, 0, 30, 12));
    assert_eq!(parts(&areas[0])[2], Rect::new(6, 0, 24, 3));
    assert_eq!(parts(&areas[0])[3], Rect::new(6, 3, 24, 1));
    assert_eq!(parts(&areas[0])[0], Rect::new(0, 0, 30, 4));
    assert_eq!(parts(&areas[1])[0].y, 5);
    let zero = Form::new([FormField::new("A").input_rows(0)]);
    assert_eq!(parts(&zero.layout(Rect::new(0, 0, 20, 5))[0])[2].height, 1);
}

#[test]
fn the_label_column_fits_the_longest_label_or_the_chosen_width() {
    let plain = Form::new([FormField::new("Name"), FormField::new("Nickname")]);
    assert_eq!(plain.layout(Rect::new(0, 0, 30, 6))[0].input.x, 10);
    assert_eq!(parts(&plain.layout(Rect::new(0, 0, 30, 6))[0])[1].width, 8);
    let wide = plain.clone().label_width(12);
    assert_eq!(wide.layout(Rect::new(0, 0, 30, 6))[0].input.x, 14);
    assert_eq!(wide.layout(Rect::new(0, 0, 30, 6))[0].label.width, 12);
    let none = Form::new([FormField::new("日本")]);
    assert_eq!(none.layout(Rect::new(0, 0, 30, 6))[0].input.x, 6);
}

#[test]
fn narrow_areas_keep_one_input_cell_and_clip_the_label_column() {
    let narrow = form().layout(Rect::new(0, 0, 8, 12));
    assert_eq!(narrow[0].label.width, 7);
    assert_eq!(narrow[0].input.width, 1);
    let tiny = form().layout(Rect::new(0, 0, 4, 12));
    assert_eq!(tiny[0].label.width, 3);
    assert_eq!(tiny[0].input.x, 5);
    assert_eq!(tiny[0].input.width, 1);
    let single = form().layout(Rect::new(0, 0, 1, 12));
    assert_eq!(single[0].label.width, 0);
    assert_eq!(single[0].input.width, 1);
    assert_eq!(form().layout(Rect::new(0, 0, 0, 12)), []);
}

#[test]
fn fields_that_do_not_fit_the_height_are_left_out() {
    assert_eq!(form().layout(Rect::new(0, 0, 30, 1)), []);
    assert_eq!(form().layout(Rect::new(0, 0, 30, 2)).len(), 1);
    assert_eq!(form().layout(Rect::new(0, 0, 30, 4)).len(), 1);
    assert_eq!(form().layout(Rect::new(0, 0, 30, 5)).len(), 2);
    assert_eq!(form().layout(Rect::new(0, 0, 30, 7)).len(), 2);
    assert_eq!(form().layout(Rect::new(0, 0, 30, 8)).len(), 3);
    assert_eq!(
        Form::new(Vec::<FormField>::new()).layout(Rect::new(0, 0, 9, 9)),
        []
    );
}

#[test]
fn labels_markers_hints_and_errors_are_drawn_where_the_layout_says() {
    let with_error = Form::new([
        FormField::new("Name").required().error("Too short"),
        FormField::new("Email").required().hint("We never share it"),
        FormField::new("Notes"),
    ]);
    assert_eq!(
        rows(&with_error, 30, 8),
        [
            "Name *                        ",
            "         ✖ Too short          ",
            "                              ",
            "Email *                       ",
            "         We never share it    ",
            "                              ",
            "Notes                         ",
            "                              ",
        ]
    );
}

#[test]
fn an_error_replaces_the_hint_and_long_messages_are_clipped() {
    let form = Form::new([FormField::new("A")
        .hint("a long hint")
        .error("a long error")]);
    assert_eq!(rows(&form, 14, 2), ["A             ", "   ✖ a long er"]);
    let hint = Form::new([FormField::new("A").hint("a long hint")]);
    assert_eq!(rows(&hint, 12, 2), ["A           ", "   a long hi"]);
}

#[test]
fn the_marker_is_clipped_with_the_label_column() {
    let form = Form::new([FormField::new("Name").required()]).label_width(5);
    assert_eq!(rows(&form, 12, 2)[0], "Name        ");
    let wider = Form::new([FormField::new("Name").required()]).label_width(6);
    assert_eq!(rows(&wider, 12, 2)[0], "Name *      ");
    let long = Form::new([FormField::new("Username").required()]).label_width(6);
    assert_eq!(rows(&long, 12, 2)[0], "Userna      ");
}

#[test]
fn label_marker_and_message_colors_follow_the_theme() {
    let form = Form::new([FormField::new("Name").required().error("Bad")]).focus(Some(0));
    let (grid, _) = draw(&form, Rect::new(0, 0, 20, 3), 20, 3);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::Accent));
    assert!(style_at(&grid, 0, 0).attrs.bold());
    assert_eq!(style_at(&grid, 4, 0).fg, color(Token::Fail));
    assert_eq!(style_at(&grid, 5, 0).fg, color(Token::Fail));
    assert_eq!(style_at(&grid, 9, 1).fg, color(Token::Fail));
    assert!(!style_at(&grid, 9, 1).attrs.dim());

    let calm = Form::new([FormField::new("Name").hint("Help")]);
    let (grid, _) = draw(&calm, Rect::new(0, 0, 20, 3), 20, 3);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 0, 0).attrs.bold());
    assert!(style_at(&grid, 6, 1).attrs.dim());
    assert_eq!(style_at(&grid, 6, 1).fg, color(Token::TextPrimary));
}

#[test]
fn only_the_focused_field_is_highlighted() {
    let form = form().focus(Some(1));
    let (grid, _) = draw(&form, Rect::new(0, 0, 30, 8), 30, 8);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 0, 3).fg, color(Token::Accent));
    assert!(style_at(&grid, 0, 3).attrs.bold());
    assert_eq!(style_at(&grid, 0, 6).fg, color(Token::TextPrimary));
}

#[test]
fn disabled_fields_are_dimmed_and_never_highlighted() {
    let form = Form::new([
        FormField::new("Name").required().error("Bad").disabled(),
        FormField::new("Email"),
    ])
    .focus(Some(0));
    let (grid, _) = draw(&form, Rect::new(0, 0, 20, 6), 20, 6);
    assert!(style_at(&grid, 0, 0).attrs.dim());
    assert!(!style_at(&grid, 0, 0).attrs.bold());
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::TextPrimary));
    assert!(style_at(&grid, 4, 0).attrs.dim());
    assert!(style_at(&grid, 9, 1).attrs.dim());
    assert!(!style_at(&grid, 0, 3).attrs.dim());
}

#[test]
fn step_moves_through_the_tab_order_and_wraps() {
    let form = form();
    assert_eq!(form.step(None, true), Some(0));
    assert_eq!(form.step(None, false), Some(2));
    assert_eq!(form.step(Some(0), true), Some(1));
    assert_eq!(form.step(Some(2), true), Some(0));
    assert_eq!(form.step(Some(0), false), Some(2));
    assert_eq!(form.step(Some(2), false), Some(1));
    assert_eq!(form.step(Some(3), true), Some(0));
    assert_eq!(form.step(Some(3), false), Some(2));
    assert_eq!(form.step(Some(9), true), Some(0));
}

#[test]
fn step_skips_disabled_fields() {
    let form = Form::new([
        FormField::new("A"),
        FormField::new("B").disabled(),
        FormField::new("C"),
    ]);
    assert_eq!(form.step(Some(0), true), Some(2));
    assert_eq!(form.step(Some(2), false), Some(0));
    let all = Form::new([FormField::new("A").disabled()]);
    assert_eq!(all.step(None, true), None);
    assert_eq!(all.step(Some(0), false), None);
    assert_eq!(Form::new(Vec::<FormField>::new()).step(None, true), None);
}

#[test]
fn check_required_flags_empty_and_blank_required_fields_only() {
    let checked = form().check_required(&["", "a@b.c", ""]);
    assert!(!checked.is_valid());
    assert_eq!(checked.first_error(), Some(0));
    assert_eq!(rows(&checked, 30, 8)[1], "         ✖ Required           ");
    let blank = form().check_required(&["   ", "  ", "x"]);
    assert_eq!(blank.first_error(), Some(0));
    let ok = form().check_required(&["Ada", "a@b.c", ""]);
    assert!(ok.is_valid());
    assert_eq!(ok.first_error(), None);
}

#[test]
fn check_required_keeps_an_existing_error_and_ignores_extra_or_missing_values() {
    let form = Form::new([
        FormField::new("Name").required().error("Too short"),
        FormField::new("Email").required(),
    ])
    .check_required(&["", ""]);
    assert_eq!(rows(&form, 30, 5)[1], "         ✖ Too short          ");
    assert_eq!(rows(&form, 30, 5)[4], "         ✖ Required           ");
    let short = Form::new([
        FormField::new("A").required(),
        FormField::new("B").required(),
    ])
    .check_required(&[""]);
    assert_eq!(short.first_error(), Some(0));
    assert!(!short.is_valid());
    let one = Form::new([
        FormField::new("A").required(),
        FormField::new("B").required(),
    ])
    .check_required(&["x"]);
    assert!(one.is_valid());
}

#[test]
fn fields_report_their_message_and_the_form_its_size() {
    assert_eq!(FormField::new("A").error_message(), None);
    assert_eq!(
        FormField::new("A").error("Bad").error_message(),
        Some("Bad")
    );
    assert_eq!(form().len(), 3);
    assert!(!form().is_empty());
    assert!(Form::new(Vec::<FormField>::new()).is_empty());
}

#[test]
fn measure_is_the_width_and_the_rows_the_fields_use() {
    assert_eq!(form().measure(Size::new(30, 20)), Size::new(30, 8));
    assert_eq!(form().gap(0).measure(Size::new(30, 20)), Size::new(30, 6));
    assert_eq!(form().measure(Size::new(30, 5)), Size::new(30, 5));
    assert_eq!(form().measure(Size::new(30, 1)), Size::new(0, 0));
    assert_eq!(form().measure(Size::new(0, 9)), Size::new(0, 0));
    assert_eq!(
        Form::new(Vec::<FormField>::new()).measure(Size::new(9, 9)),
        Size::new(0, 0)
    );
}

#[test]
fn nothing_is_drawn_outside_the_area_or_for_empty_areas() {
    let (grid, clusters) = draw(&form(), Rect::new(3, 1, 12, 2), 20, 6);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    assert_eq!(grid.row_display(1, &clusters), "   Name *           ");
    assert_eq!(grid.row_display(3, &clusters).trim(), "");
    let (grid, clusters) = draw(&form(), Rect::new(0, 0, 0, 5), 10, 5);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
}
