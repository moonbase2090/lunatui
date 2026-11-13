// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_term::{Event, KeyCode, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
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

struct FormApp {
    fields: Vec<FormField>,
    values: Vec<String>,
    disabled: Vec<usize>,
    focus: Option<usize>,
    submitted: bool,
    valid: bool,
    area: Rect,
}

impl FormApp {
    fn new(fields: Vec<FormField>) -> Self {
        let values = vec![String::new(); fields.len()];
        Self {
            fields,
            values,
            disabled: Vec::new(),
            focus: None,
            submitted: false,
            valid: false,
            area: Rect::new(0, 0, 0, 0),
        }
    }

    fn form(&self) -> Form {
        let form = Form::new(self.fields.clone()).focus(self.focus);
        if self.submitted {
            let values: Vec<&str> = self.values.iter().map(String::as_str).collect();
            form.check_required(&values)
        } else {
            form
        }
    }

    fn submit(&mut self) {
        self.submitted = true;
        let form = self.form();
        self.valid = form.is_valid();
        if let Some(index) = form.first_error() {
            self.focus = Some(index);
        }
    }

    fn click(&mut self, x: u16, y: u16) {
        let form = self.form();
        let hit = form.layout(self.area).into_iter().find(|field| {
            x >= field.row.x
                && x - field.row.x < field.row.width
                && y >= field.row.y
                && y - field.row.y < field.row.height
        });
        if let Some(field) = hit.filter(|field| !self.disabled.contains(&field.index)) {
            self.focus = Some(field.index);
        }
    }
}

impl Component for FormApp {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Tab => {
                    let forward = !key.modifiers.is_shift();
                    self.focus = self.form().step(self.focus, forward).or(self.focus);
                }
                KeyCode::Enter => self.submit(),
                KeyCode::Backspace => {
                    if let Some(value) = self.focus.and_then(|index| self.values.get_mut(index)) {
                        value.pop();
                    }
                }
                KeyCode::Char(ch) => {
                    if let Some(value) = self.focus.and_then(|index| self.values.get_mut(index)) {
                        value.push(ch);
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                self.click(mouse.column, mouse.row);
            }
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        self.area = area;
        let theme = Theme::builtin(Variant::Dark);
        let form = self.form();
        let mut cx = RenderCx::new(grid, clusters, &theme);
        form.render(area, &mut cx);
        for field in form.layout(area) {
            grid.put_str(
                field.input.x,
                field.input.y,
                Style::new(),
                &self.values[field.index],
                clusters,
            );
        }
    }

    fn focus(&self) -> Option<usize> {
        self.focus
    }

    fn value(&self) -> Option<String> {
        Some(self.values.join("|"))
    }
}

fn signup() -> Harness<FormApp> {
    Harness::mount(FormApp::new(fields()), 30, 8)
}

#[test]
fn tab_walks_the_fields_in_order_and_wraps() {
    let mut form = signup();
    form.assert_focus(None);
    let seen: Vec<Option<usize>> = (0..4)
        .map(|_| {
            form.key("Tab");
            form.component().focus()
        })
        .collect();
    assert_eq!(seen, [Some(0), Some(1), Some(2), Some(0)]);
}

#[test]
fn shift_tab_walks_backward_and_starts_at_the_last_field() {
    let mut form = signup();
    form.key("S-Tab");
    form.assert_focus(Some(2));
    form.keys("S-Tab S-Tab S-Tab");
    form.assert_focus(Some(2));
}

#[test]
fn a_focus_past_the_last_field_restarts_the_order() {
    let mut app = FormApp::new(fields());
    app.focus = Some(9);
    let mut form = Harness::mount(app, 30, 8);
    form.key("Tab");
    form.assert_focus(Some(0));
    let mut app = FormApp::new(fields());
    app.focus = Some(3);
    let mut form = Harness::mount(app, 30, 8);
    form.key("S-Tab");
    form.assert_focus(Some(2));
}

#[test]
fn tab_skips_disabled_fields() {
    let app = FormApp::new(vec![
        FormField::new("A"),
        FormField::new("B").disabled(),
        FormField::new("C"),
    ]);
    let mut form = Harness::mount(app, 20, 8);
    form.keys("Tab Tab");
    form.assert_focus(Some(2));
    form.key("S-Tab");
    form.assert_focus(Some(0));
}

#[test]
fn tab_does_nothing_when_no_field_can_take_focus() {
    let mut form = Harness::mount(FormApp::new(vec![FormField::new("A").disabled()]), 20, 4);
    form.keys("Tab S-Tab");
    form.assert_focus(None);
    let mut form = Harness::mount(FormApp::new(Vec::new()), 20, 4);
    form.keys("Tab S-Tab");
    form.assert_focus(None);
}

#[test]
fn typing_edits_only_the_focused_field() {
    let mut form = signup();
    form.keys("Tab");
    form.type_text("Ada");
    form.key("Tab");
    form.type_text("a@b.c");
    form.key("Backspace");
    form.assert_value("Ada|a@b.|");
    let rows = form.rows();
    assert_eq!(rows[0], "Name *   Ada");
    assert_eq!(rows[3], "Email *  a@b.");
}

#[test]
fn enter_with_required_fields_empty_flags_each_and_jumps_to_the_first() {
    let mut form = signup();
    form.keys("Tab Tab Tab");
    form.assert_focus(Some(2));
    assert_eq!(form.rows()[4], "         We never share it");
    form.key("Enter");
    form.assert_focus(Some(0));
    let rows = form.rows();
    assert_eq!(rows[1], "         ✖ Required");
    assert_eq!(rows[4], "         ✖ Required");
    assert!(!form.component().valid);
}

#[test]
fn a_required_field_holding_only_spaces_is_still_empty() {
    let mut form = signup();
    form.keys("Tab Space Space Enter");
    form.assert_focus(Some(0));
    assert_eq!(form.rows()[1], "         ✖ Required");
}

#[test]
fn filling_the_required_fields_and_pressing_enter_clears_every_error() {
    let mut form = signup();
    form.key("Enter");
    form.assert_focus(Some(0));
    form.type_text("Ada");
    form.key("Enter");
    form.assert_focus(Some(1));
    let rows = form.rows();
    assert_eq!(rows[1], "");
    assert_eq!(rows[4], "         ✖ Required");
    form.type_text("a@b.c");
    form.key("Enter");
    assert!(form.component().valid);
    let rows = form.rows();
    assert_eq!(rows[1], "");
    assert_eq!(rows[4], "         We never share it");
    form.assert_value("Ada|a@b.c|");
}

#[test]
fn a_click_on_a_field_focuses_it_wherever_its_rows_are_clicked() {
    for (x, y) in [(0, 3), (12, 3), (12, 4)] {
        let mut form = signup();
        form.click(x, y);
        form.assert_focus(Some(1));
    }
}

#[test]
fn a_click_on_the_gap_between_fields_does_nothing() {
    let mut form = signup();
    form.click(3, 2);
    form.click(3, 5);
    form.assert_focus(None);
    form.click(3, 0);
    form.click(3, 2);
    form.assert_focus(Some(0));
}

#[test]
fn a_click_on_a_disabled_field_does_not_focus_it() {
    let mut app = FormApp::new(vec![FormField::new("A").disabled(), FormField::new("B")]);
    app.disabled = vec![0];
    let mut form = Harness::mount(app, 20, 6);
    form.click(3, 0);
    form.assert_focus(None);
    form.click(3, 3);
    form.assert_focus(Some(1));
}

#[test]
fn a_form_with_no_room_ignores_clicks() {
    let mut form = Harness::mount(FormApp::new(fields()), 0, 8);
    form.click(0, 0);
    form.assert_focus(None);
}
