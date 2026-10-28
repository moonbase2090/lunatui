// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    BorderType, FocusTrap, Placement, Popup, PopupState, RenderCx, Widget, WidgetState,
};

struct Text(&'static str);

impl Widget for Text {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        cx.put_str(area.x, area.y, self.0, Style::new());
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(3, 1).clamp(avail)
    }
}

fn plain(text: &'static str) -> Popup<Text> {
    Popup::new(Text(text)).backdrop(0.0)
}

fn draw_on(popup: &Popup<Text>, grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    popup.render(area, &mut cx);
}

fn rows_on(popup: &Popup<Text>, width: u16, height: u16, fill: char) -> Vec<String> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, width, height),
        Style::new(),
        fill,
        &mut clusters,
    );
    draw_on(popup, &mut grid, &mut clusters, &theme);
    (0..height)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn rows(popup: &Popup<Text>, width: u16, height: u16) -> Vec<String> {
    rows_on(popup, width, height, ' ')
}

fn style_at(popup: &Popup<Text>, x: u16, y: u16) -> Result<Style, &'static str> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(9, 5);
    let mut clusters = ClusterTable::new();
    draw_on(popup, &mut grid, &mut clusters, &theme);
    Ok(grid.get(x, y).ok_or("missing cell")?.style())
}

#[test]
fn a_titled_popup_is_centred_with_the_odd_cell_after_it() {
    let popup = plain("Hello World")
        .size(Size::new(11, 1))
        .title("Title")
        .border_type(BorderType::Plain);
    assert_eq!(
        rows(&popup, 20, 5),
        [
            "                    ",
            "   ┌Title──────┐    ",
            "   │Hello World│    ",
            "   └───────────┘    ",
            "                    ",
        ]
    );
}

#[test]
fn the_default_border_is_rounded_and_the_size_comes_from_measure() {
    assert_eq!(
        rows(&plain("abc"), 9, 5),
        [
            "         ",
            "  ╭───╮  ",
            "  │abc│  ",
            "  ╰───╯  ",
            "         "
        ]
    );
}

#[test]
fn the_panel_clears_what_is_behind_it() {
    let popup = plain("ab").size(Size::new(5, 1));
    assert_eq!(
        rows_on(&popup, 11, 5, '#'),
        [
            "###########",
            "##╭─────╮##",
            "##│ab   │##",
            "##╰─────╯##",
            "###########",
        ]
    );
}

#[test]
fn below_an_anchor_with_left_edges_aligned() {
    let popup = plain("abc").placement(Placement::Below(Rect::new(2, 1, 4, 1)));
    assert_eq!(popup.area(Rect::new(0, 0, 12, 8)), Rect::new(2, 2, 5, 3));
}

#[test]
fn below_flips_above_when_there_is_no_room_below() {
    let popup = plain("abc").placement(Placement::Below(Rect::new(2, 6, 4, 1)));
    assert_eq!(popup.area(Rect::new(0, 0, 12, 8)), Rect::new(2, 3, 5, 3));
}

#[test]
fn above_an_anchor_and_below_it_when_there_is_no_room_above() {
    let above = plain("abc").placement(Placement::Above(Rect::new(2, 5, 4, 1)));
    assert_eq!(above.area(Rect::new(0, 0, 12, 8)), Rect::new(2, 2, 5, 3));
    let flipped = plain("abc").placement(Placement::Above(Rect::new(2, 1, 4, 1)));
    assert_eq!(flipped.area(Rect::new(0, 0, 12, 8)), Rect::new(2, 2, 5, 3));
}

#[test]
fn right_and_left_of_an_anchor_flip_when_there_is_no_room() {
    let area = Rect::new(0, 0, 12, 8);
    let right = |anchor| plain("abc").placement(Placement::Right(anchor)).area(area);
    let left = |anchor| plain("abc").placement(Placement::Left(anchor)).area(area);
    assert_eq!(right(Rect::new(1, 1, 3, 1)), Rect::new(4, 1, 5, 3));
    assert_eq!(right(Rect::new(8, 1, 3, 1)), Rect::new(3, 1, 5, 3));
    assert_eq!(left(Rect::new(8, 1, 3, 1)), Rect::new(3, 1, 5, 3));
    assert_eq!(left(Rect::new(2, 1, 3, 1)), Rect::new(5, 1, 5, 3));
}

#[test]
fn a_popup_at_a_point_is_kept_inside_the_area() {
    let at = |x, y| plain("abc").placement(Placement::At(x, y));
    assert_eq!(at(4, 1).area(Rect::new(0, 0, 20, 5)), Rect::new(4, 1, 5, 3));
    assert_eq!(
        at(18, 4).area(Rect::new(0, 0, 20, 5)),
        Rect::new(15, 2, 5, 3)
    );
    assert_eq!(
        at(0, 0).area(Rect::new(10, 10, 20, 5)),
        Rect::new(10, 10, 5, 3)
    );
}

#[test]
fn a_popup_larger_than_the_area_fills_it() {
    let popup = plain("abc").size(Size::new(30, 10));
    assert_eq!(popup.area(Rect::new(3, 2, 10, 4)), Rect::new(3, 2, 10, 4));
}

#[test]
fn dragging_moves_the_popup_and_reset_brings_it_back() {
    let area = Rect::new(0, 0, 20, 10);
    let popup = plain("abc");
    let mut state = PopupState::new();
    assert!(!state.press(8, 4));
    popup.sync(area, &mut state);
    assert_eq!(state.area(), Some(Rect::new(7, 3, 5, 3)));
    assert!(!state.press(6, 3));
    assert!(!state.is_dragging());
    assert!(state.press(8, 4));
    assert!(state.is_dragging());
    state.drag(12, 6);
    assert_eq!(state.moved_to(), Some((11, 5)));
    assert_eq!(state.area(), Some(Rect::new(11, 5, 5, 3)));
    assert_eq!(
        plain("abc").position(&state).area(area),
        Rect::new(11, 5, 5, 3)
    );
    state.drag(40, 40);
    plain("abc").position(&state).sync(area, &mut state);
    assert_eq!(state.moved_to(), Some((15, 7)));
    state.release();
    assert!(!state.is_dragging());
    state.drag(1, 1);
    assert_eq!(state.moved_to(), Some((15, 7)));
    state.move_by(-20, -1);
    assert_eq!(state.moved_to(), Some((0, 6)));
    state.move_to(2, 2);
    assert_eq!(state.area(), Some(Rect::new(2, 2, 5, 3)));
    state.reset();
    assert_eq!(state.moved_to(), None);
    assert_eq!(
        plain("abc").position(&state).area(area),
        Rect::new(7, 3, 5, 3)
    );
}

#[test]
fn a_moved_popup_is_drawn_where_it_was_put() {
    let mut state = PopupState::new();
    let area = Rect::new(0, 0, 9, 5);
    plain("abc").sync(area, &mut state);
    state.move_by(-2, 1);
    let popup = plain("abc").position(&state);
    assert_eq!(
        rows(&popup, 9, 5),
        [
            "         ",
            "         ",
            "╭───╮    ",
            "│abc│    ",
            "╰───╯    "
        ]
    );
}

#[test]
fn theme_colors_follow_the_widget_state() -> Result<(), &'static str> {
    let theme = Theme::builtin(Variant::Dark);
    let token = |token: Token| theme.resolve(token);
    let panel = Style::new()
        .fg(token(Token::TextPrimary))
        .bg(token(Token::Surface1));
    let bold = Attrs::new().set_bold(true);
    let dim = Attrs::new().set_dim(true);
    let titled = |state| plain("abc").title("T").state(state);
    assert_eq!(style_at(&titled(WidgetState::Normal), 3, 2)?, panel);
    assert_eq!(
        style_at(&titled(WidgetState::Normal), 2, 1)?,
        panel.fg(token(Token::Accent))
    );
    assert_eq!(
        style_at(&titled(WidgetState::Normal), 3, 1)?,
        panel.fg(token(Token::Accent)).with_attrs(bold)
    );
    assert_eq!(
        style_at(&titled(WidgetState::Focused), 2, 1)?,
        panel.fg(token(Token::FocusRing))
    );
    assert_eq!(
        style_at(&titled(WidgetState::Focused), 3, 1)?,
        panel.fg(token(Token::FocusRing)).with_attrs(bold)
    );
    assert_eq!(
        style_at(&titled(WidgetState::Disabled), 2, 1)?,
        panel.with_attrs(dim)
    );
    assert_eq!(
        style_at(&titled(WidgetState::Disabled), 3, 1)?,
        panel.with_attrs(dim)
    );
    Ok(())
}

#[test]
fn explicit_styles_win() -> Result<(), &'static str> {
    let panel = Style::new().fg(Color::Rgb(1, 1, 1)).bg(Color::Rgb(2, 2, 2));
    let border = Style::new().fg(Color::Rgb(3, 3, 3)).bg(Color::Rgb(4, 4, 4));
    let title = Style::new().fg(Color::Rgb(5, 5, 5)).bg(Color::Rgb(6, 6, 6));
    let popup = plain("abc")
        .title("T")
        .style(panel)
        .border_style(border)
        .title_style(title);
    assert_eq!(style_at(&popup, 4, 2)?.bg, Color::Rgb(2, 2, 2));
    assert_eq!(style_at(&popup, 2, 1)?, border);
    assert_eq!(style_at(&popup, 3, 1)?, title);
    Ok(())
}

#[test]
fn the_backdrop_fades_what_is_behind_toward_the_surface() -> Result<(), &'static str> {
    let mut theme = Theme::builtin(Variant::Dark);
    theme.set_token(Token::Surface0, Color::Rgb(0, 0, 0));
    theme.set_token(Token::TextPrimary, Color::Rgb(100, 100, 100));
    let mut grid = Grid::new(9, 5);
    let mut clusters = ClusterTable::new();
    grid.put_str(
        0,
        0,
        Style::new().fg(Color::Rgb(200, 100, 50)),
        "x",
        &mut clusters,
    );
    grid.put_str(1, 0, Style::new(), "y", &mut clusters);
    let popup = Popup::new(Text("abc")).backdrop(0.5);
    draw_on(&popup, &mut grid, &mut clusters, &theme);
    let style = |x, y| grid.get(x, y).map(lunatui_core::Cell::style);
    assert_eq!(
        style(0, 0),
        Some(
            Style::new()
                .fg(Color::Rgb(100, 50, 25))
                .bg(Color::Rgb(0, 0, 0))
        )
    );
    assert_eq!(
        style(1, 0),
        Some(
            Style::new()
                .fg(Color::Rgb(50, 50, 50))
                .bg(Color::Rgb(0, 0, 0))
        )
    );
    assert_eq!(grid.row_display(0, &clusters), "xy       ");
    assert_eq!(
        style(3, 2).map(|style| style.bg),
        Some(theme.resolve(Token::Surface1))
    );
    Ok(())
}

#[test]
fn no_backdrop_leaves_what_is_behind_alone() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(9, 5);
    let mut clusters = ClusterTable::new();
    let ink = Style::new().fg(Color::Rgb(200, 100, 50));
    grid.put_str(0, 0, ink, "x", &mut clusters);
    draw_on(&plain("abc"), &mut grid, &mut clusters, &theme);
    assert_eq!(grid.get(0, 0).map(lunatui_core::Cell::style), Some(ink));
}

#[test]
fn the_popup_traps_focus_unless_told_not_to() {
    assert!(plain("abc").traps_focus());
    assert!(!plain("abc").focus_trap(FocusTrap::disabled()).traps_focus());
}

#[test]
fn an_empty_area_draws_nothing() {
    assert_eq!(rows(&plain("abc"), 0, 0), Vec::<String>::new());
}
