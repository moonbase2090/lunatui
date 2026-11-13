// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_term::{Event, KeyCode, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RadioFlow, RadioGroup, RenderCx, Widget, WidgetState};

fn sizes() -> RadioGroup {
    RadioGroup::new(["Small", "Medium", "Large"])
}

fn grid_for(group: &RadioGroup, area: Rect, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    group.render(area, &mut cx);
    (grid, clusters)
}

fn rows(group: &RadioGroup, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = grid_for(group, Rect::new(0, 0, width, height), width, height);
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

#[test]
fn options_stack_one_per_row_with_the_selected_one_filled() {
    assert_eq!(
        rows(&sizes().select(Some(1)), 10, 4),
        ["○ Small   ", "◉ Medium  ", "○ Large   ", "          "]
    );
    assert_eq!(
        rows(&sizes(), 10, 3),
        ["○ Small   ", "○ Medium  ", "○ Large   "]
    );
}

#[test]
fn a_gap_leaves_blank_rows_between_options() {
    assert_eq!(
        rows(&sizes().select(Some(2)).gap(1), 9, 5),
        [
            "○ Small  ",
            "         ",
            "○ Medium ",
            "         ",
            "◉ Large  "
        ]
    );
}

#[test]
fn horizontal_options_sit_side_by_side_separated_by_the_gap() {
    let group = sizes().select(Some(1)).flow(RadioFlow::Horizontal).gap(2);
    assert_eq!(rows(&group, 28, 1), ["○ Small  ◉ Medium  ○ Large  "]);
    let tight = sizes().flow(RadioFlow::Horizontal);
    assert_eq!(rows(&tight, 22, 1), ["○ Small○ Medium○ Large"]);
}

#[test]
fn a_selection_past_the_last_option_selects_nothing() {
    assert_eq!(
        rows(&sizes().select(Some(7)), 10, 3),
        ["○ Small   ", "○ Medium  ", "○ Large   "]
    );
}

#[test]
fn options_that_do_not_fit_are_left_out_and_labels_are_clipped() {
    assert_eq!(rows(&sizes(), 10, 2), ["○ Small   ", "○ Medium  "]);
    assert_eq!(rows(&sizes(), 5, 3), ["○ Sma", "○ Med", "○ Lar"]);
    assert_eq!(rows(&sizes(), 2, 1), ["○ "]);
    assert_eq!(rows(&sizes(), 1, 1), ["○"]);
    let group = sizes().flow(RadioFlow::Horizontal).gap(1);
    assert_eq!(rows(&group, 11, 1), ["○ Small ○ M"]);
    assert_eq!(rows(&group, 8, 1), ["○ Small "]);
}

#[test]
fn wide_labels_count_by_display_width() {
    let group = RadioGroup::new(["日本語", "ab"]);
    assert_eq!(
        group.option_areas(Rect::new(0, 0, 20, 2))[0].1,
        Rect::new(0, 0, 8, 1)
    );
    assert_eq!(
        rows(&group.select(Some(1)), 10, 2),
        ["○ 日本語  ", "◉ ab      "]
    );
}

#[test]
fn len_and_is_empty_count_the_options() {
    assert_eq!(sizes().len(), 3);
    assert!(!sizes().is_empty());
    assert!(RadioGroup::new(Vec::<String>::new()).is_empty());
}

#[test]
fn colors_follow_selection_focus_and_the_theme() {
    let group = sizes().select(Some(1)).focus(Some(2));
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 0, 1).fg, color(Token::Accent));
    assert_eq!(style_at(&grid, 2, 1).fg, color(Token::TextPrimary));
    assert_eq!(style_at(&grid, 0, 2).fg, color(Token::FocusRing));
    assert!(style_at(&grid, 0, 2).attrs.bold());
    assert!(style_at(&grid, 4, 2).attrs.bold());
    assert!(!style_at(&grid, 0, 1).attrs.bold());
    assert!(!style_at(&grid, 4, 0).attrs.bold());
}

#[test]
fn a_focused_selected_option_shows_the_focus_ring() {
    let group = sizes().select(Some(0)).focus(Some(0));
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    assert_eq!(style_at(&grid, 0, 0).fg, color(Token::FocusRing));
}

#[test]
fn disabled_options_and_groups_are_dimmed_and_lose_focus() {
    let group = sizes().disabled_options([1]).focus(Some(1));
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    assert!(style_at(&grid, 0, 1).attrs.dim());
    assert!(style_at(&grid, 4, 1).attrs.dim());
    assert!(!style_at(&grid, 0, 1).attrs.bold());
    assert_eq!(style_at(&grid, 0, 1).fg, color(Token::TextPrimary));
    assert!(!style_at(&grid, 0, 0).attrs.dim());

    let all = sizes().state(WidgetState::Disabled);
    let (grid, _) = grid_for(&all, Rect::new(0, 0, 10, 3), 10, 3);
    for y in 0..3 {
        assert!(style_at(&grid, 0, y).attrs.dim(), "row {y}");
        assert!(style_at(&grid, 4, y).attrs.dim(), "row {y}");
    }
}

#[test]
fn an_explicit_style_replaces_the_theme_colors() {
    let style = Style::new().fg(Color::Rgb(1, 2, 3));
    let group = sizes().select(Some(0)).focus(Some(1)).style(style);
    let (grid, _) = grid_for(&group, Rect::new(0, 0, 10, 3), 10, 3);
    for (x, y) in [(0, 0), (4, 0), (0, 1), (4, 1), (0, 2)] {
        assert_eq!(style_at(&grid, x, y), style, "({x}, {y})");
    }
}

#[test]
fn nothing_is_drawn_outside_the_area() {
    let group = sizes().select(Some(0));
    let (grid, clusters) = grid_for(&group, Rect::new(2, 1, 6, 2), 12, 5);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let full = ["  ◉ Small   ", "  ○ Medium  "];
    for (row, line) in (1_u16..).zip(full) {
        let shown: String = line.chars().take(8).collect();
        let drawn = grid.row_display(row, &clusters);
        assert_eq!(drawn.chars().take(8).collect::<String>(), shown);
        assert_eq!(drawn.chars().skip(8).collect::<String>().trim(), "");
    }
    assert_eq!(grid.row_display(3, &clusters).trim(), "");
}

#[test]
fn empty_areas_draw_nothing() {
    let (grid, clusters) = grid_for(&sizes(), Rect::new(0, 0, 0, 3), 8, 3);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
    let (grid, clusters) = grid_for(&sizes(), Rect::new(0, 0, 8, 0), 8, 3);
    assert_eq!(grid.row_display(0, &clusters).trim(), "");
}

#[test]
fn measure_is_the_bounding_box_of_the_options() {
    assert_eq!(sizes().measure(Size::new(20, 10)), Size::new(8, 3));
    assert_eq!(sizes().gap(1).measure(Size::new(20, 10)), Size::new(8, 5));
    assert_eq!(sizes().measure(Size::new(20, 2)), Size::new(8, 2));
    assert_eq!(sizes().measure(Size::new(5, 10)), Size::new(5, 3));
    let across = sizes().flow(RadioFlow::Horizontal).gap(2);
    assert_eq!(across.measure(Size::new(40, 10)), Size::new(26, 1));
    assert_eq!(across.measure(Size::new(10, 10)), Size::new(10, 1));
    assert_eq!(sizes().measure(Size::new(0, 5)), Size::new(0, 0));
    assert_eq!(sizes().measure(Size::new(5, 0)), Size::new(0, 0));
    assert_eq!(
        RadioGroup::new(Vec::<String>::new()).measure(Size::new(5, 5)),
        Size::new(0, 0)
    );
}

struct RadioApp {
    labels: &'static [&'static str],
    disabled: Vec<usize>,
    flow: RadioFlow,
    gap: u16,
    focus: Option<usize>,
    selected: Option<usize>,
    area: Rect,
}

impl RadioApp {
    fn new(labels: &'static [&'static str]) -> Self {
        Self {
            labels,
            disabled: Vec::new(),
            flow: RadioFlow::Vertical,
            gap: 0,
            focus: None,
            selected: None,
            area: Rect::new(0, 0, 0, 0),
        }
    }

    fn group(&self) -> RadioGroup {
        RadioGroup::new(self.labels.iter().copied())
            .select(self.selected)
            .focus(self.focus)
            .disabled_options(self.disabled.iter().copied())
            .flow(self.flow)
            .gap(self.gap)
    }

    fn move_focus(&mut self, forward: bool) {
        if let Some(next) = self.group().step(self.focus, forward) {
            self.focus = Some(next);
            self.selected = Some(next);
        }
    }
}

impl Component for RadioApp {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Down | KeyCode::Right => self.move_focus(true),
                KeyCode::Up | KeyCode::Left => self.move_focus(false),
                KeyCode::Char(' ') => self.selected = self.focus,
                _ => {}
            },
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                let hit = self
                    .group()
                    .option_areas(self.area)
                    .into_iter()
                    .find(|(_, rect)| {
                        mouse.column >= rect.x
                            && mouse.column - rect.x < rect.width
                            && mouse.row >= rect.y
                            && mouse.row - rect.y < rect.height
                    });
                if let Some((index, _)) = hit.filter(|(index, _)| !self.disabled.contains(index)) {
                    self.focus = Some(index);
                    self.selected = Some(index);
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        self.area = area;
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        self.group().render(area, &mut cx);
    }

    fn focus(&self) -> Option<usize> {
        self.focus
    }

    fn value(&self) -> Option<String> {
        self.selected
            .and_then(|index| self.labels.get(index))
            .map(|label| (*label).to_owned())
    }
}

const SIZES: [&str; 3] = ["Small", "Medium", "Large"];

fn radio(app: RadioApp, width: u16, height: u16) -> Harness<RadioApp> {
    Harness::mount(app, width, height)
}

#[test]
fn down_and_up_move_the_choice_and_wrap_around_the_ends() {
    let mut group = radio(RadioApp::new(&SIZES), 10, 3);
    group.assert_rows(&["○ Small", "○ Medium", "○ Large"]);
    group.keys("Down");
    group.assert_focus(Some(0));
    group.assert_value("Small");
    group.keys("Down Down");
    group.assert_value("Large");
    group.assert_rows(&["○ Small", "○ Medium", "◉ Large"]);
    group.keys("Down");
    group.assert_value("Small");
    group.keys("Up");
    group.assert_value("Large");
    group.keys("Up");
    group.assert_value("Medium");
}

#[test]
fn right_and_left_move_the_choice_like_down_and_up() {
    let mut group = radio(RadioApp::new(&SIZES), 10, 3);
    group.keys("Right Right Left");
    group.assert_value("Small");
}

#[test]
fn up_from_nothing_focused_starts_at_the_last_option() {
    let mut group = radio(RadioApp::new(&SIZES), 10, 3);
    group.keys("Up");
    group.assert_value("Large");
}

#[test]
fn a_focus_past_the_last_option_restarts_from_the_ends() {
    let mut app = RadioApp::new(&SIZES);
    app.focus = Some(9);
    let mut group = radio(app, 10, 3);
    group.keys("Down");
    group.assert_value("Small");
    let mut app = RadioApp::new(&SIZES);
    app.focus = Some(3);
    let mut group = radio(app, 10, 3);
    group.keys("Up");
    group.assert_value("Large");
}

#[test]
fn space_chooses_the_focused_option_without_moving() {
    let mut app = RadioApp::new(&SIZES);
    app.focus = Some(1);
    let mut group = radio(app, 10, 3);
    group.keys("Space");
    group.assert_value("Medium");
    group.assert_focus(Some(1));
}

#[test]
fn arrows_skip_disabled_options_in_both_directions() {
    let mut app = RadioApp::new(&SIZES);
    app.disabled = vec![1];
    let mut group = radio(app, 10, 3);
    group.keys("Down Down");
    group.assert_value("Large");
    group.keys("Up");
    group.assert_value("Small");
}

#[test]
fn with_one_enabled_option_the_arrows_stay_on_it() {
    let mut app = RadioApp::new(&SIZES);
    app.disabled = vec![0, 2];
    let mut group = radio(app, 10, 3);
    group.keys("Down Down Up");
    group.assert_value("Medium");
    group.assert_focus(Some(1));
}

#[test]
fn arrows_do_nothing_when_no_option_can_be_chosen() {
    let mut app = RadioApp::new(&SIZES);
    app.disabled = vec![0, 1, 2];
    let mut group = radio(app, 10, 3);
    group.keys("Down Up");
    group.assert_focus(None);
    assert_eq!(group.component().value(), None);
    let mut group = radio(RadioApp::new(&[]), 10, 3);
    group.keys("Down Up");
    group.assert_focus(None);
}

#[test]
fn a_click_on_the_glyph_the_blank_or_the_label_chooses_that_option() {
    for x in [0, 1, 2, 6] {
        let mut group = radio(RadioApp::new(&SIZES), 20, 5);
        group.click(x, 2);
        group.assert_value("Large");
        group.assert_focus(Some(2));
    }
}

#[test]
fn a_click_beside_an_option_chooses_nothing() {
    let mut group = radio(RadioApp::new(&SIZES), 20, 5);
    group.click(10, 1);
    group.click(0, 4);
    group.click(7, 0);
    assert_eq!(group.component().value(), None);
    group.click(6, 0);
    group.assert_value("Small");
    group.click(7, 1);
    group.assert_value("Medium");
    group.click(8, 1);
    group.assert_value("Medium");
    group.assert_focus(Some(1));
}

#[test]
fn rows_left_blank_by_the_gap_do_not_take_clicks() {
    let mut app = RadioApp::new(&SIZES);
    app.gap = 1;
    let mut group = radio(app, 20, 4);
    group.assert_rows(&["○ Small", "", "○ Medium", ""]);
    group.click(1, 1);
    assert_eq!(group.component().value(), None);
    group.click(1, 2);
    group.assert_value("Medium");
}

#[test]
fn side_by_side_options_take_clicks_on_their_own_cells_only() {
    let mut app = RadioApp::new(&SIZES);
    app.flow = RadioFlow::Horizontal;
    app.gap = 3;
    let mut group = radio(app, 40, 1);
    group.assert_rows(&["○ Small   ○ Medium   ○ Large"]);
    group.click(10, 0);
    group.assert_value("Medium");
    group.click(8, 0);
    group.assert_value("Medium");
    group.click(21, 0);
    group.assert_value("Large");
}

#[test]
fn an_option_cut_off_by_the_edge_takes_clicks_only_where_it_shows() {
    let mut app = RadioApp::new(&SIZES);
    app.flow = RadioFlow::Horizontal;
    let mut group = radio(app, 10, 1);
    group.assert_rows(&["○ Small○ M"]);
    group.click(9, 0);
    group.assert_value("Medium");
}

#[test]
fn clicking_a_disabled_option_chooses_nothing() {
    let mut app = RadioApp::new(&SIZES);
    app.disabled = vec![1];
    let mut group = radio(app, 20, 3);
    group.click(2, 1);
    assert_eq!(group.component().value(), None);
    group.click(2, 2);
    group.assert_value("Large");
}

#[test]
fn a_window_with_no_room_ignores_clicks() {
    let mut group = radio(RadioApp::new(&SIZES), 0, 3);
    group.click(0, 0);
    assert_eq!(group.component().value(), None);
    let mut group = radio(RadioApp::new(&SIZES), 5, 0);
    group.click(0, 0);
    assert_eq!(group.component().value(), None);
}
