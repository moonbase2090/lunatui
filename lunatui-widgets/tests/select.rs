// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_term::{Event, KeyCode, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{RenderCx, Select, Widget, WidgetState};

const FRUIT: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];

fn draw(select: &Select<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    select.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(select: &Select<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(select, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters).trim_end().to_owned())
        .collect()
}

fn grid(select: &Select<'_>, width: u16, height: u16) -> Grid {
    draw(select, width, height).0
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn color(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

#[test]
fn a_closed_select_draws_the_chosen_option_and_a_down_arrow() {
    let select = Select::new(&FRUIT, Some(2));
    assert_eq!(rows(&select, 20, 3), [" banana           ▾", "", ""]);
}

#[test]
fn no_choice_draws_the_placeholder_dim() {
    let select = Select::new(&FRUIT, None).placeholder("Pick a fruit");
    let cells = grid(&select, 20, 1);
    assert_eq!(rows(&select, 20, 1), [" Pick a fruit     ▾"]);
    assert!(style_at(&cells, 1, 0).attrs.dim());
    assert_eq!(style_at(&cells, 1, 0).fg, color(Token::TextPrimary));
}

#[test]
fn an_out_of_range_choice_draws_the_placeholder() {
    let select = Select::new(&FRUIT, Some(9)).placeholder("Pick a fruit");
    assert_eq!(rows(&select, 20, 1), [" Pick a fruit     ▾"]);
}

#[test]
fn an_open_select_lists_every_option_with_a_check_on_the_chosen_one() {
    let select = Select::new(&FRUIT, Some(2)).open(true);
    assert_eq!(
        rows(&select, 20, 6),
        [
            " banana           ▴",
            "   apple",
            "   apricot",
            " ✓ banana",
            "   blueberry",
            "   cherry",
        ]
    );
}

#[test]
fn the_active_row_is_drawn_on_the_accent_color() {
    let select = Select::new(&FRUIT, Some(2)).open(true).active(Some(1));
    let cells = grid(&select, 20, 6);
    assert_eq!(style_at(&cells, 3, 2).bg, color(Token::Accent));
    assert_eq!(style_at(&cells, 3, 2).fg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 3, 1).bg, color(Token::Surface0));
}

#[test]
fn an_out_of_range_active_row_highlights_nothing() {
    let select = Select::new(&FRUIT, Some(0)).open(true).active(Some(9));
    let cells = grid(&select, 20, 3);
    assert_eq!(style_at(&cells, 3, 1).bg, color(Token::Surface0));
    assert_eq!(style_at(&cells, 3, 2).bg, color(Token::Surface0));
}

#[test]
fn the_list_scrolls_to_keep_the_active_row_in_view() {
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .active(Some(4))
        .max_rows(2);
    assert_eq!(rows(&select, 20, 3)[1..], ["   blueberry", "   cherry"]);
}

#[test]
fn max_rows_of_zero_still_shows_one_row() {
    let select = Select::new(&FRUIT, Some(0)).open(true).max_rows(0);
    assert_eq!(rows(&select, 20, 4)[1..], [" ✓ apple", "", ""]);
}

#[test]
fn an_open_select_with_no_options_says_so() {
    let none: [&str; 0] = [];
    let select = Select::new(&none, None).open(true);
    assert_eq!(rows(&select, 20, 3)[1], " No options");
    assert_eq!(select.option_at(Rect::new(0, 0, 20, 3), 3, 1), None);
}

#[test]
fn long_options_are_cut_to_the_field_and_list_width() {
    let words = ["abcdefghijklmnop"];
    let select = Select::new(&words, Some(0)).open(true);
    assert_eq!(rows(&select, 12, 2), [" abcdefgh ▴", " ✓ abcdefghi"]);
}

#[test]
fn narrow_fields_drop_the_arrow() {
    let select = Select::new(&FRUIT, Some(0));
    assert_eq!(rows(&select, 7, 1), [" apple"]);
    assert_eq!(rows(&select, 8, 1), [" appl ▾"]);
}

#[test]
fn disabled_dims_the_field_and_the_list() {
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .state(WidgetState::Disabled);
    let cells = grid(&select, 20, 3);
    assert!(style_at(&cells, 1, 0).attrs.dim());
    assert!(style_at(&cells, 3, 1).attrs.dim());
}

#[test]
fn disabled_dims_the_active_row_too() {
    let select = Select::new(&FRUIT, Some(0))
        .open(true)
        .active(Some(0))
        .state(WidgetState::Disabled);
    let cells = grid(&select, 20, 3);
    let style = style_at(&cells, 3, 1);
    assert_eq!(style.bg, color(Token::Accent));
    assert!(style.attrs.dim());
}

#[test]
fn focused_draws_the_arrow_bold_in_the_accent_color() {
    let select = Select::new(&FRUIT, Some(0)).state(WidgetState::Focused);
    let cells = grid(&select, 20, 1);
    let arrow = style_at(&cells, 18, 0);
    assert!(arrow.attrs.bold());
    assert!(!arrow.attrs.dim());
    assert_eq!(arrow.fg, color(Token::Accent));
}

#[test]
fn field_and_list_areas_split_the_area() {
    let area = Rect::new(2, 4, 30, 4);
    let closed = Select::new(&FRUIT, Some(0));
    assert_eq!(closed.field_area(area), Rect::new(2, 4, 30, 1));
    assert_eq!(closed.list_area(area), Rect::new(2, 5, 30, 0));
    let open = closed.open(true);
    assert_eq!(open.list_area(area), Rect::new(2, 5, 30, 3));
}

#[test]
fn measure_is_the_field_plus_the_visible_rows() {
    let closed = Select::new(&FRUIT, Some(0));
    assert_eq!(closed.measure(Size::new(20, 9)), Size::new(20, 1));
    assert_eq!(
        closed.clone().open(true).measure(Size::new(20, 9)),
        Size::new(20, 6)
    );
    assert_eq!(closed.measure(Size::new(0, 9)), Size::new(0, 0));
    assert_eq!(closed.measure(Size::new(20, 0)), Size::new(0, 0));
}

#[test]
fn zero_sized_areas_draw_nothing() {
    let select = Select::new(&FRUIT, Some(0)).open(true);
    assert_eq!(rows(&select, 0, 3), ["", "", ""]);
    assert_eq!(rows(&select, 20, 0), Vec::<String>::new());
}

struct SelectApp {
    options: &'static [&'static str],
    selected: Option<usize>,
    open: bool,
    active: Option<usize>,
    typed: String,
    max_rows: u16,
    area: Rect,
}

impl SelectApp {
    fn new(options: &'static [&'static str], selected: Option<usize>) -> Self {
        Self {
            options,
            selected,
            open: false,
            active: None,
            typed: String::new(),
            max_rows: 6,
            area: Rect::new(0, 0, 0, 0),
        }
    }

    fn widget(&self) -> Select<'static> {
        Select::new(self.options, self.selected)
            .placeholder("Pick one")
            .open(self.open)
            .active(self.active)
            .max_rows(self.max_rows)
            .state(WidgetState::Focused)
    }

    fn open_list(&mut self) {
        self.open = true;
        self.active = self.selected.filter(|index| *index < self.options.len());
    }

    fn close(&mut self) {
        self.open = false;
        self.typed.clear();
    }

    fn choose_active(&mut self) {
        if let Some(index) = self.active {
            self.selected = Some(index);
        }
        self.close();
    }

    fn type_char(&mut self, ch: char) {
        self.typed.push(ch);
        let mut found = Select::type_ahead(self.options, self.active, &self.typed);
        if found.is_none() {
            self.typed = ch.to_string();
            found = Select::type_ahead(self.options, self.active, &self.typed);
        }
        if let Some(index) = found {
            self.active = Some(index);
            if !self.open {
                self.selected = Some(index);
            }
        }
    }

    fn click(&mut self, x: u16, y: u16) {
        let field = self.widget().field_area(self.area);
        if y == field.y && x >= field.x && x - field.x < field.width {
            if self.open {
                self.close();
            } else {
                self.open_list();
            }
        } else if self.open {
            if let Some(index) = self.widget().option_at(self.area, x, y) {
                self.selected = Some(index);
            }
            self.close();
        }
    }
}

impl Component for SelectApp {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => self.close(),
                KeyCode::Enter | KeyCode::Char(' ') if self.open => self.choose_active(),
                KeyCode::Enter | KeyCode::Char(' ') => self.open_list(),
                KeyCode::Down | KeyCode::Up if !self.open => self.open_list(),
                KeyCode::Down | KeyCode::Up => {
                    let forward = key.code == KeyCode::Down;
                    self.active = Select::<&str>::step(self.active, self.options.len(), forward);
                }
                KeyCode::Char(ch) => self.type_char(ch),
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
        let mut cx = RenderCx::new(grid, clusters, &theme);
        self.widget().render(area, &mut cx);
    }

    fn focus(&self) -> Option<usize> {
        self.active
    }

    fn value(&self) -> Option<String> {
        self.selected
            .and_then(|index| self.options.get(index))
            .map(|option| (*option).to_owned())
    }
}

fn mounted(selected: Option<usize>) -> Harness<SelectApp> {
    Harness::mount(SelectApp::new(&FRUIT, selected), 20, 6)
}

#[test]
fn down_opens_the_list_on_the_chosen_option_and_arrows_move_the_highlight() {
    let mut select = mounted(Some(2));
    select.key("Down");
    select.assert_focus(Some(2));
    select.assert_rows(&[
        " banana           ▴",
        "   apple",
        "   apricot",
        " ✓ banana",
        "   blueberry",
        "   cherry",
    ]);
    select.key("Down");
    select.assert_focus(Some(3));
    let highlighted = style_at(select.grid(), 3, 4);
    assert_eq!(highlighted.bg, color(Token::Accent));
    assert_ne!(style_at(select.grid(), 3, 3).bg, color(Token::Accent));
}

#[test]
fn arrows_wrap_around_the_ends_of_the_list() {
    let mut select = mounted(Some(4));
    select.keys("Down Down");
    select.assert_focus(Some(0));
    select.keys("Up");
    select.assert_focus(Some(4));
}

#[test]
fn from_no_choice_down_starts_at_the_first_option_and_up_at_the_last() {
    let mut select = mounted(None);
    select.keys("Down Down");
    select.assert_focus(Some(0));
    let mut select = mounted(None);
    select.keys("Up Up");
    select.assert_focus(Some(4));
}

#[test]
fn a_stale_choice_past_the_options_opens_with_nothing_highlighted() {
    let mut select = mounted(Some(9));
    select.key("Down");
    select.assert_focus(None);
    select.key("Down");
    select.assert_focus(Some(0));
}

#[test]
fn arrows_do_nothing_when_there_are_no_options() {
    let mut select = Harness::mount(SelectApp::new(&[], None), 20, 3);
    select.keys("Down Down Up");
    select.assert_focus(None);
    select.assert_rows(&[" Pick one         ▴", " No options", ""]);
}

#[test]
fn enter_chooses_the_highlighted_option_and_closes_the_list() {
    let mut select = mounted(Some(0));
    select.keys("Down Down Down Enter");
    select.assert_value("banana");
    select.assert_rows(&[" banana           ▾", "", "", "", "", ""]);
}

#[test]
fn escape_closes_the_list_and_keeps_the_choice() {
    let mut select = mounted(Some(0));
    select.keys("Down Down Esc");
    select.assert_value("apple");
    select.assert_rows(&[" apple            ▾", "", "", "", "", ""]);
}

#[test]
fn typing_jumps_to_the_next_option_with_the_prefix_and_repeats_cycle_through_them() {
    let mut select = mounted(None);
    select.type_text("b");
    select.assert_value("banana");
    select.type_text("b");
    select.assert_value("blueberry");
    select.type_text("b");
    select.assert_value("banana");
}

#[test]
fn typing_a_longer_prefix_narrows_the_match() {
    let mut select = mounted(Some(0));
    select.type_text("ap");
    select.assert_value("apricot");
    select.type_text("p");
    select.assert_value("apple");
}

#[test]
fn type_ahead_ignores_case_and_a_miss_keeps_the_choice() {
    let mut select = mounted(None);
    select.type_text("CH");
    select.assert_value("cherry");
    select.type_text("z");
    select.assert_value("cherry");
}

#[test]
fn type_ahead_matches_accented_letters_case_insensitively() {
    const WORDS: [&str; 2] = ["Éclair", "eclair"];
    let mut select = Harness::mount(SelectApp::new(&WORDS, None), 20, 3);
    select.type_text("é");
    select.assert_value("Éclair");
    select.type_text("e");
    select.assert_value("eclair");
}

#[test]
fn clicking_the_field_opens_the_list_and_clicking_it_again_closes_it() {
    let mut select = mounted(Some(0));
    select.click(5, 0);
    select.assert_focus(Some(0));
    assert_eq!(select.rows()[1], " ✓ apple");
    select.click(5, 0);
    select.assert_rows(&[" apple            ▾", "", "", "", "", ""]);
}

#[test]
fn clicking_a_row_chooses_that_option() {
    let mut select = mounted(Some(0));
    select.click(5, 0);
    select.click(19, 2);
    select.assert_value("apricot");
    select.assert_rows(&[" apricot          ▾", "", "", "", "", ""]);
}

#[test]
fn clicking_outside_the_list_closes_it_without_choosing() {
    let mut select = Harness::mount(SelectApp::new(&FRUIT, Some(0)), 20, 8);
    select.click(5, 0);
    assert_eq!(select.rows()[5], "   cherry");
    select.click(5, 7);
    select.assert_value("apple");
    assert_eq!(select.rows()[1], "");
    select.click(5, 0);
    select.click(25, 2);
    select.assert_value("apple");
    assert_eq!(select.rows()[1], "");
}

#[test]
fn clicking_where_the_list_would_be_does_nothing_while_it_is_closed() {
    let mut select = mounted(Some(0));
    select.click(5, 2);
    select.assert_value("apple");
    select.assert_rows(&[" apple            ▾", "", "", "", "", ""]);
}

#[test]
fn a_click_on_a_scrolled_list_chooses_the_option_drawn_under_it() {
    let mut app = SelectApp::new(&FRUIT, Some(0));
    app.max_rows = 2;
    let mut select = Harness::mount(app, 20, 3);
    select.click(5, 0);
    select.keys("Up");
    select.assert_focus(Some(4));
    select.assert_rows(&[" apple            ▴", "   blueberry", "   cherry"]);
    select.click(3, 1);
    select.assert_value("blueberry");
}

#[test]
fn a_zero_width_window_ignores_clicks() {
    let mut select = Harness::mount(SelectApp::new(&FRUIT, Some(0)), 0, 3);
    select.click(0, 0);
    select.click(0, 1);
    select.assert_value("apple");
}
