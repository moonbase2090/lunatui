// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Alignment, BigText, BigTextFont, PixelSize, RenderCx, Widget, WidgetState};

fn row_string(text: &str, size: PixelSize, row: u16) -> String {
    let widget = BigText::new(text).pixel_size(size);
    let mut glyphs = ['?'; 64];
    let count = widget.row_glyphs(row, &mut glyphs);
    glyphs.iter().take(count).collect()
}

fn face(text: &str, size: PixelSize) -> String {
    let height = size.cells().1;
    (0..height)
        .map(|row| row_string(text, size, row))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(widget: &BigText<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    widget.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

#[test]
fn pixel_sizes_pack_the_hollow_box() {
    assert_eq!(row_string("#", PixelSize::Full, 0), "████████████████");
    assert_eq!(row_string("#", PixelSize::HalfHeight, 0), "█▀▀▀▀▀▀█");
    assert_eq!(row_string("#", PixelSize::HalfWidth, 0), "████");
    assert_eq!(row_string("#", PixelSize::Quadrant, 0), "▛▀▀▜");
    let braille = row_string("#", PixelSize::Braille, 0);
    let expected: String = ['\u{284F}', '\u{2809}', '\u{2809}', '\u{28B9}']
        .into_iter()
        .collect();
    assert_eq!(braille, expected);
}

#[test]
fn five_by_seven_clock_font_measures_and_shades_rows() {
    let top = Style::new().fg(Color::Rgb(220, 210, 249));
    let bottom = Style::new().fg(Color::Rgb(143, 124, 224));
    let widget = BigText::new("23:47")
        .font(BigTextFont::FiveBySeven)
        .row_styles(&[top, top, top, top, top, top, bottom]);
    assert_eq!(widget.measure(Size::new(100, 20)), Size::new(52, 7));

    let mut glyphs = ['?'; 52];
    assert_eq!(widget.row_glyphs(0, &mut glyphs), 52);
    assert_eq!(
        glyphs.iter().collect::<String>(),
        concat!(
            "  ██████    ",
            "██████████  ",
            "    ",
            "      ██    ",
            "██████████  "
        )
    );

    let (grid, _) = render(&widget, 52, 7);
    assert_eq!(grid.get(2, 0).map(|cell| cell.style().fg), Some(top.fg));
    assert_eq!(grid.get(0, 6).map(|cell| cell.style().fg), Some(bottom.fg));
}

#[test]
fn lowercase_uses_the_uppercase_bitmap() {
    assert_eq!(
        row_string("a", PixelSize::Full, 0),
        row_string("A", PixelSize::Full, 0)
    );
}

#[test]
fn unknown_and_control_characters_do_not_invent_ink() {
    assert_eq!(row_string("é", PixelSize::Full, 0), "                ");
    assert_eq!(
        row_string("A\nB", PixelSize::Quadrant, 0).chars().count(),
        8
    );
    assert_eq!(
        BigText::new("#")
            .pixel_size(PixelSize::Full)
            .row_glyphs(12, &mut ['x'; 4]),
        0
    );
}

#[test]
fn short_buffer_keeps_the_full_width() {
    let mut tiny = ['x'; 2];
    let count = BigText::new("#")
        .pixel_size(PixelSize::Full)
        .row_glyphs(0, &mut tiny);
    assert_eq!(count, 16);
    assert_eq!(tiny, ['█', '█']);
}

#[test]
fn measure_follows_the_cell_size_and_clips() {
    assert_eq!(
        BigText::new("AB")
            .pixel_size(PixelSize::Quadrant)
            .measure(Size::new(80, 40)),
        Size::new(8, 4)
    );
    assert_eq!(
        BigText::new("AB")
            .pixel_size(PixelSize::Braille)
            .measure(Size::new(80, 40)),
        Size::new(8, 2)
    );
    assert_eq!(
        BigText::new("A").measure(Size::new(3, 100)),
        Size::new(3, 8)
    );
    assert_eq!(BigText::new("A").measure(Size::new(0, 5)), Size::new(0, 0));
}

#[test]
fn alignment_places_the_first_inked_column() {
    let left = render(&BigText::new("#").pixel_size(PixelSize::Full), 32, 1);
    let center = render(
        &BigText::new("#")
            .pixel_size(PixelSize::Full)
            .alignment(Alignment::Center),
        32,
        1,
    );
    let right = render(
        &BigText::new("#")
            .pixel_size(PixelSize::Full)
            .alignment(Alignment::Right),
        32,
        1,
    );
    assert_eq!(left.0.row_display(0, &left.1).chars().next(), Some('█'));
    assert_eq!(center.0.row_display(0, &center.1).chars().nth(8), Some('█'));
    assert_eq!(right.0.row_display(0, &right.1).chars().nth(16), Some('█'));
}

#[test]
fn state_picks_theme_tokens_until_a_style_is_set() {
    let theme = Theme::builtin(Variant::Dark);
    let focused = render(
        &BigText::new("#")
            .pixel_size(PixelSize::Full)
            .state(WidgetState::Focused),
        8,
        1,
    );
    let disabled = render(
        &BigText::new("#")
            .pixel_size(PixelSize::Full)
            .state(WidgetState::Disabled),
        8,
        1,
    );
    let explicit = render(
        &BigText::new("#")
            .pixel_size(PixelSize::Full)
            .style(Style::new().fg(Color::Indexed(5)))
            .state(WidgetState::Focused),
        8,
        1,
    );
    assert_eq!(
        focused.0.get(0, 0).map(|cell| cell.style().fg),
        Some(theme.resolve(Token::Accent))
    );
    assert_eq!(
        focused.0.get(0, 0).map(|cell| cell.style().attrs.bold()),
        Some(true)
    );
    assert_eq!(
        disabled.0.get(0, 0).map(|cell| cell.style().fg),
        Some(theme.resolve(Token::TextPrimary))
    );
    assert_eq!(
        disabled.0.get(1, 0).map(|cell| cell.style().attrs.dim()),
        Some(true)
    );
    assert_eq!(
        explicit
            .0
            .get(0, 0)
            .map(|cell| (cell.style().fg, cell.style().attrs.bold())),
        Some((Color::Indexed(5), false))
    );
}

#[test]
fn zero_area_draws_nothing() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    BigText::new("#").render(Rect::new(0, 0, 0, 0), &mut cx);
    assert_eq!(grid.row_display(0, &clusters), "  ");
}

#[test]
fn lookalike_pairs_do_not_share_a_bitmap() {
    let size = PixelSize::Quadrant;
    for (left, right) in [("S", "5"), ("O", "0"), ("I", "1"), ("Z", "2"), ("B", "8")] {
        assert_ne!(face(left, size), face(right, size), "{left} / {right}");
    }
    for (left, right) in [("U", "V"), ("V", "W"), ("W", "M"), ("M", "N"), ("U", "N")] {
        assert_ne!(face(left, size), face(right, size), "{left} / {right}");
    }
}

#[test]
fn sextant_and_octant_fall_back_until_the_font_is_reported() {
    let quadrant = face("#", PixelSize::Quadrant);
    assert_eq!(
        BigText::new("#")
            .pixel_size(PixelSize::Sextant)
            .measure(Size::new(80, 40)),
        BigText::new("#")
            .pixel_size(PixelSize::Quadrant)
            .measure(Size::new(80, 40))
    );
    let sextant = BigText::new("#")
        .pixel_size(PixelSize::Sextant)
        .subcell_glyphs(true);
    assert_eq!(sextant.measure(Size::new(80, 40)), Size::new(4, 3));
    let mut glyphs = ['?'; 8];
    let count = sextant.row_glyphs(0, &mut glyphs);
    assert_eq!(count, 4);
    assert_eq!(glyphs[1], '\u{1FB02}');
    assert_ne!(
        glyphs.iter().take(count).collect::<String>(),
        quadrant.lines().next().unwrap_or("")
    );
    assert_eq!(
        BigText::new("#")
            .pixel_size(PixelSize::Octant)
            .subcell_glyphs(true)
            .measure(Size::new(80, 40)),
        Size::new(4, 2)
    );
}
