// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    RenderCx, ScrollView, ScrollViewState, ScrollbarVisibility, Widget, WidgetState,
};

struct Letters;

impl Widget for Letters {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        for row in 0..area.height {
            let line: String = (0..area.width)
                .map(|col| {
                    let index = u32::from(col) + u32::from(row) * u32::from(area.width);
                    char::from_u32(index % 26 + u32::from(b'A')).unwrap_or('?')
                })
                .collect();
            cx.put_str(area.x, area.y + row, &line, Style::new());
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Text(&'static str, Style);

impl Widget for Text {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        cx.put_str(area.x, area.y, self.0, self.1);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn draw<W: Widget>(view: &ScrollView<W>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    view.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows<W: Widget>(view: &ScrollView<W>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(view, width, height);
    (0..height)
        .map(|row| grid.row_display(row, &clusters))
        .collect()
}

fn letters() -> ScrollView<Letters> {
    ScrollView::new(Letters, Size::new(10, 10))
}

#[test]
fn content_that_fits_draws_at_the_top_left_without_scrollbars() {
    let view = ScrollView::new(Letters, Size::new(4, 2));
    assert_eq!(rows(&view, 6, 3), ["ABCD  ", "EFGH  ", "      "]);
}

#[test]
fn content_larger_on_both_axes_gets_both_scrollbars() {
    assert_eq!(
        rows(&letters(), 6, 6),
        ["ABCDE█", "KLMNO█", "UVWXY█", "EFGHI│", "OPQRS│", "━━━── "]
    );
}

#[test]
fn an_offset_scrolls_the_content_and_moves_both_thumbs() {
    assert_eq!(
        rows(&letters().offset(3, 4), 6, 6),
        ["RSTUV│", "BCDEF█", "LMNOP█", "VWXYZ█", "FGHIJ│", "─━━━─ "]
    );
}

#[test]
fn an_offset_past_the_end_stops_at_the_last_page() {
    assert_eq!(
        rows(&letters().offset(99, 99), 6, 6),
        ["DEFGH│", "NOPQR│", "XYZAB█", "HIJKL█", "RSTUV█", "──━━━ "]
    );
}

#[test]
fn the_view_draws_from_the_state_offset() {
    let mut scroll = ScrollViewState::new();
    letters().sync(Rect::new(0, 0, 6, 6), &mut scroll);
    scroll.scroll_down(1);
    scroll.scroll_right(1);
    assert_eq!(
        rows(&letters().scroll(&scroll), 6, 6),
        ["LMNOP█", "VWXYZ█", "FGHIJ█", "PQRST│", "ZABCD│", "━━━── "]
    );
}

#[test]
fn hidden_scrollbars_give_the_content_the_whole_area() {
    let view = letters()
        .scrollbars(ScrollbarVisibility::Never)
        .offset(4, 0);
    assert_eq!(rows(&view, 6, 3), ["EFGHIJ", "OPQRST", "YZABCD"]);
}

#[test]
fn a_tall_narrow_document_gets_only_the_vertical_scrollbar() {
    let view = ScrollView::new(Letters, Size::new(5, 10));
    assert_eq!(rows(&view, 6, 4), ["ABCDE█", "FGHIJ█", "KLMNO│", "PQRST│"]);
}

#[test]
fn a_horizontal_scrollbar_that_takes_the_last_row_brings_the_vertical_one() {
    let view = ScrollView::new(Letters, Size::new(7, 4));
    assert_eq!(rows(&view, 6, 4), ["ABCDE█", "HIJKL█", "OPQRS█", "━━━━─ "]);
}

#[test]
fn always_draws_a_scrollbar_for_content_that_fits() {
    let view =
        ScrollView::new(Letters, Size::new(3, 2)).vertical_scrollbar(ScrollbarVisibility::Always);
    assert_eq!(rows(&view, 5, 2), ["ABC █", "DEF █"]);
}

#[test]
fn the_viewport_leaves_room_for_the_scrollbars() {
    assert_eq!(
        letters().viewport(Rect::new(2, 3, 6, 6)),
        Rect::new(2, 3, 5, 5)
    );
    assert_eq!(
        letters()
            .scrollbars(ScrollbarVisibility::Never)
            .viewport(Rect::new(2, 3, 6, 6)),
        Rect::new(2, 3, 6, 6)
    );
}

#[test]
fn content_without_a_background_shows_the_surface_under_the_view() -> Result<(), &'static str> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(4, 1);
    let mut clusters = ClusterTable::new();
    let surface = Style::new().bg(Color::Rgb(40, 50, 60));
    grid.fill(Rect::new(0, 0, 4, 1), surface, ' ', &mut clusters);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let ink = Style::new().fg(Color::Rgb(200, 0, 0));
    ScrollView::new(Text("ab", ink), Size::new(2, 1)).render(Rect::new(1, 0, 3, 1), &mut cx);
    let cell = grid.get(1, 0).ok_or("missing cell")?;
    assert_eq!(cell.style(), ink.bg(Color::Rgb(40, 50, 60)));
    assert_eq!(grid.row_display(0, &clusters), " ab ");
    Ok(())
}

#[test]
fn a_wide_glyph_cut_by_the_viewport_edge_becomes_blank() {
    let view = || {
        ScrollView::new(Text("世界ab", Style::new()), Size::new(6, 1))
            .scrollbars(ScrollbarVisibility::Never)
    };
    assert_eq!(rows(&view().offset(1, 0), 4, 1), [" 界a"]);
    assert_eq!(rows(&view(), 3, 1), ["世 "]);
}

#[test]
fn scrollbar_colors_follow_the_widget_state() -> Result<(), &'static str> {
    let theme = Theme::builtin(Variant::Dark);
    let style_at = |state: WidgetState, row: u16| -> Result<Style, &'static str> {
        let (grid, _) = draw(&letters().state(state), 6, 6);
        Ok(grid.get(5, row).ok_or("missing cell")?.style())
    };
    let plain = |token: Token| Style::new().fg(theme.resolve(token));
    assert_eq!(style_at(WidgetState::Normal, 0)?, plain(Token::Accent));
    assert_eq!(style_at(WidgetState::Normal, 4)?, plain(Token::Surface1));
    assert_eq!(style_at(WidgetState::Focused, 0)?, plain(Token::FocusRing));
    assert_eq!(
        style_at(WidgetState::Disabled, 0)?,
        plain(Token::TextPrimary).with_attrs(Attrs::new().set_dim(true))
    );
    Ok(())
}

#[test]
fn explicit_scrollbar_styles_win_and_the_theme_ones_keep_the_background() -> Result<(), &'static str>
{
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(6, 6);
    let mut clusters = ClusterTable::new();
    let panel = Color::Rgb(1, 2, 3);
    grid.fill(
        Rect::new(0, 0, 6, 6),
        Style::new().bg(panel),
        ' ',
        &mut clusters,
    );
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let thumb = Style::new().fg(Color::Rgb(9, 9, 9)).bg(Color::Rgb(7, 7, 7));
    letters()
        .thumb_style(thumb)
        .render(Rect::new(0, 0, 6, 6), &mut cx);
    assert_eq!(grid.get(5, 0).ok_or("missing cell")?.style(), thumb);
    assert_eq!(
        grid.get(5, 4).ok_or("missing cell")?.style(),
        Style::new().fg(theme.resolve(Token::Surface1)).bg(panel)
    );
    Ok(())
}

#[test]
fn sync_records_sizes_so_paging_stops_at_the_last_page() {
    let mut scroll = ScrollViewState::new();
    assert_eq!(scroll.page_size(), None);
    letters().sync(Rect::new(0, 0, 6, 6), &mut scroll);
    assert_eq!(scroll.content_size(), Some(Size::new(10, 10)));
    assert_eq!(scroll.page_size(), Some(Size::new(5, 5)));
    scroll.page_down();
    assert_eq!(scroll.offset(), (0, 4));
    assert!(!scroll.is_at_bottom());
    scroll.page_down();
    assert_eq!(scroll.offset(), (0, 5));
    assert!(scroll.is_at_bottom());
    scroll.page_up();
    assert_eq!(scroll.offset(), (0, 1));
    scroll.scroll_right(9);
    assert_eq!(scroll.offset(), (5, 1));
    scroll.scroll_left(2);
    scroll.scroll_up(5);
    assert_eq!(scroll.offset(), (3, 0));
    scroll.scroll_to_bottom();
    assert_eq!(scroll.offset(), (3, 5));
    scroll.scroll_to_top();
    assert_eq!(scroll.offset(), (0, 0));
}

#[test]
fn sync_pulls_an_offset_back_into_range() {
    let mut scroll = ScrollViewState::with_offset(40, 40);
    letters().sync(Rect::new(0, 0, 6, 6), &mut scroll);
    assert_eq!(scroll.offset(), (5, 5));
    scroll.set_offset(2, 30);
    assert_eq!(scroll.offset(), (2, 5));
}

#[test]
fn before_the_first_sync_scrolling_is_unbounded_and_a_page_is_one_row() {
    let mut scroll = ScrollViewState::new();
    scroll.page_down();
    scroll.scroll_down(2);
    assert_eq!(scroll.offset(), (0, 3));
    assert!(scroll.is_at_bottom());
    scroll.scroll_to_bottom();
    assert_eq!(scroll.offset(), (0, u16::MAX));
}

#[test]
fn measure_is_the_content_size_within_the_space_given() {
    assert_eq!(letters().measure(Size::new(4, 20)), Size::new(4, 10));
}

#[test]
fn an_empty_area_draws_nothing() {
    assert_eq!(rows(&letters(), 0, 0), Vec::<String>::new());
}
