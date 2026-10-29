// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size, Style, clip_display_prefix, symbols::border, width};

use crate::borders::{BorderType, Borders};
use crate::padding::Padding;
use crate::{RenderCx, Widget};

/// Horizontal alignment for block titles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// Flush left in the title band.
    #[default]
    Left,
    /// Centered in the title band.
    Center,
    /// Flush right in the title band.
    Right,
}

/// Whether a title sits on the top or bottom edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitlePosition {
    /// Top edge (default for [`Block::title`]).
    #[default]
    Top,
    /// Bottom edge.
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TitleEntry {
    position: Option<TitlePosition>,
    text: String,
    alignment: Option<Alignment>,
    style: Option<Style>,
}

/// Bordered container with optional titles (ratatui `Block` parity).
#[derive(Clone, Debug)]
pub struct Block {
    titles: Vec<TitleEntry>,
    titles_style: Style,
    titles_alignment: Alignment,
    titles_position: TitlePosition,
    borders: Borders,
    border_style: Style,
    border_set: border::Set<'static>,
    style: Style,
    padding: Padding,
    title_inset: u16,
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    /// Block with no borders.
    #[must_use]
    pub fn new() -> Self {
        Self {
            titles: Vec::new(),
            titles_style: Style::new(),
            titles_alignment: Alignment::Left,
            titles_position: TitlePosition::Top,
            borders: Borders::NONE,
            border_style: Style::new(),
            border_set: BorderType::Plain.border_set(),
            style: Style::new(),
            padding: Padding::ZERO,
            title_inset: 0,
        }
    }

    /// Block with all borders shown.
    #[must_use]
    pub fn bordered() -> Self {
        Self::new().borders(Borders::ALL)
    }

    /// Visible borders.
    #[must_use]
    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Border glyph style.
    #[must_use]
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Border symbol set from `border_type`.
    #[must_use]
    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_set = border_type.border_set();
        self
    }

    /// Background style for the block area.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Inner padding.
    #[must_use]
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Default title alignment for titles without their own.
    #[must_use]
    pub fn title_alignment(mut self, alignment: Alignment) -> Self {
        self.titles_alignment = alignment;
        self
    }

    /// Default title position for [`Block::title`].
    #[must_use]
    pub fn title_position(mut self, position: TitlePosition) -> Self {
        self.titles_position = position;
        self
    }

    /// Style applied to every title line.
    #[must_use]
    pub fn title_style(mut self, style: Style) -> Self {
        self.titles_style = style;
        self
    }

    /// Adds a title using the default position.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.titles.push(TitleEntry {
            position: None,
            text: title.into(),
            alignment: None,
            style: None,
        });
        self
    }

    /// Adds a top title with explicit alignment.
    #[must_use]
    pub fn title_top_aligned(mut self, title: impl Into<String>, alignment: Alignment) -> Self {
        self.titles.push(TitleEntry {
            position: Some(TitlePosition::Top),
            text: title.into(),
            alignment: Some(alignment),
            style: None,
        });
        self
    }

    /// Adds a title at `position` with its own alignment and style.
    ///
    /// `style` replaces [`Block::title_style`] for this title only.
    #[must_use]
    pub fn title_at(
        mut self,
        title: impl Into<String>,
        position: TitlePosition,
        alignment: Alignment,
        style: Style,
    ) -> Self {
        self.titles.push(TitleEntry {
            position: Some(position),
            text: title.into(),
            alignment: Some(alignment),
            style: Some(style),
        });
        self
    }

    /// Border cells kept between the left corner and left-aligned titles.
    ///
    /// The default of 0 starts the title right after the corner. With 1, a top
    /// title renders as `╭─ Title ─╮`. Centered and right-aligned titles are
    /// not moved.
    #[must_use]
    pub fn title_inset(mut self, cells: u16) -> Self {
        self.title_inset = cells;
        self
    }

    /// Content rectangle inside borders, titles, and padding.
    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        let mut inner = area;
        if self.borders.contains(Borders::LEFT) {
            inner.x = inner.x.saturating_add(1).min(inner.x + inner.width);
            inner.width = inner.width.saturating_sub(1);
        }
        if self.borders.contains(Borders::TOP) || self.has_title_at(TitlePosition::Top) {
            inner.y = inner.y.saturating_add(1).min(inner.y + inner.height);
            inner.height = inner.height.saturating_sub(1);
        }
        if self.borders.contains(Borders::RIGHT) {
            inner.width = inner.width.saturating_sub(1);
        }
        if self.borders.contains(Borders::BOTTOM) || self.has_title_at(TitlePosition::Bottom) {
            inner.height = inner.height.saturating_sub(1);
        }
        inner.x = inner.x.saturating_add(self.padding.left);
        inner.y = inner.y.saturating_add(self.padding.top);
        let horizontal = self.padding.left.saturating_add(self.padding.right);
        let vertical = self.padding.top.saturating_add(self.padding.bottom);
        inner.width = inner.width.saturating_sub(horizontal);
        inner.height = inner.height.saturating_sub(vertical);
        inner
    }

    fn has_title_at(&self, position: TitlePosition) -> bool {
        self.titles
            .iter()
            .any(|entry| entry.position.unwrap_or(self.titles_position) == position)
    }

    fn render_borders(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let left = area.x;
        let top = area.y;
        let right = area.x.saturating_add(area.width.saturating_sub(1));
        let bottom = area.y.saturating_add(area.height.saturating_sub(1));

        if self.borders.contains(Borders::LEFT) {
            for y in top..=bottom {
                put_symbol(
                    cx,
                    left,
                    y,
                    self.border_set.vertical_left,
                    self.border_style,
                );
            }
        }
        if self.borders.contains(Borders::TOP) {
            for x in left..=right {
                put_symbol(
                    cx,
                    x,
                    top,
                    self.border_set.horizontal_top,
                    self.border_style,
                );
            }
        }
        if self.borders.contains(Borders::RIGHT) {
            for y in top..=bottom {
                put_symbol(
                    cx,
                    right,
                    y,
                    self.border_set.vertical_right,
                    self.border_style,
                );
            }
        }
        if self.borders.contains(Borders::BOTTOM) {
            for x in left..=right {
                put_symbol(
                    cx,
                    x,
                    bottom,
                    self.border_set.horizontal_bottom,
                    self.border_style,
                );
            }
        }

        let corners = [
            (
                Borders::LEFT | Borders::TOP,
                left,
                top,
                self.border_set.top_left,
            ),
            (
                Borders::RIGHT | Borders::TOP,
                right,
                top,
                self.border_set.top_right,
            ),
            (
                Borders::LEFT | Borders::BOTTOM,
                left,
                bottom,
                self.border_set.bottom_left,
            ),
            (
                Borders::RIGHT | Borders::BOTTOM,
                right,
                bottom,
                self.border_set.bottom_right,
            ),
        ];
        for (mask, x, y, sym) in corners {
            if self.borders.contains_all(mask) {
                put_symbol(cx, x, y, sym, self.border_style);
            }
        }
    }

    fn render_titles(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.render_titles_at(TitlePosition::Top, area, cx);
        self.render_titles_at(TitlePosition::Bottom, area, cx);
    }

    fn render_titles_at(&self, position: TitlePosition, area: Rect, cx: &mut RenderCx<'_>) {
        self.render_aligned_titles(position, Alignment::Left, area, cx);
        self.render_aligned_titles(position, Alignment::Center, area, cx);
        self.render_aligned_titles(position, Alignment::Right, area, cx);
    }

    fn render_aligned_titles(
        &self,
        position: TitlePosition,
        alignment: Alignment,
        area: Rect,
        cx: &mut RenderCx<'_>,
    ) {
        let band = self.titles_band(area, position);
        if band.width == 0 {
            return;
        }
        let matching = || {
            self.titles
                .iter()
                .filter(move |entry| entry.position.unwrap_or(self.titles_position) == position)
                .filter(move |entry| entry.alignment.unwrap_or(self.titles_alignment) == alignment)
        };
        if matching().next().is_none() {
            return;
        }
        let total = || {
            matching()
                .map(|entry| title_width(entry).saturating_add(1))
                .fold(0_u16, u16::saturating_add)
                .saturating_sub(1)
        };
        let mut x = match alignment {
            Alignment::Left => band.x.saturating_add(self.title_inset.min(band.width)),
            Alignment::Center => band
                .x
                .saturating_add(band.width.saturating_sub(total()) / 2),
            Alignment::Right => band.x.saturating_add(band.width.saturating_sub(total())),
        };
        for (index, entry) in matching().enumerate() {
            if index > 0 {
                x = x.saturating_add(1);
            }
            if x >= band.x + band.width {
                break;
            }
            let remaining = band.x + band.width - x;
            let text = clip_display_prefix(&entry.text, remaining);
            let w = str_display_width(text);
            if w == 0 {
                break;
            }
            put_symbol(
                cx,
                x,
                band.y,
                text,
                entry.style.unwrap_or(self.titles_style),
            );
            x = x.saturating_add(w);
        }
    }

    #[cfg(feature = "popup")]
    pub(crate) fn render_with_title(&self, area: Rect, title: &str, cx: &mut RenderCx<'_>) {
        self.render(area, cx);
        if area.width == 0 || area.height == 0 {
            return;
        }
        let band = self.titles_band(area, self.titles_position);
        if band.width == 0 {
            return;
        }
        let x = band.x.saturating_add(self.title_inset.min(band.width));
        let text = clip_display_prefix(title, band.x + band.width - x);
        put_symbol(cx, x, band.y, text, self.titles_style);
    }

    fn titles_band(&self, area: Rect, position: TitlePosition) -> Rect {
        let left_border = u16::from(self.borders.contains(Borders::LEFT));
        let right_border = u16::from(self.borders.contains(Borders::RIGHT));
        let y = match position {
            TitlePosition::Top => area.y,
            TitlePosition::Bottom => area.y.saturating_add(area.height.saturating_sub(1)),
        };
        Rect {
            x: area.x.saturating_add(left_border),
            y,
            width: area
                .width
                .saturating_sub(left_border)
                .saturating_sub(right_border),
            height: 1,
        }
    }
}

impl Widget for Block {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.set_style(area, self.style);
        self.render_borders(area, cx);
        self.render_titles(area, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(2, 2).clamp(avail)
    }
}

fn title_width(entry: &TitleEntry) -> u16 {
    str_display_width(&entry.text)
}

fn str_display_width(text: &str) -> u16 {
    u16::try_from(width(text)).unwrap_or(u16::MAX)
}

fn put_symbol(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    if !text.is_empty() {
        cx.put_str(x, y, text, style);
    }
}

#[cfg(all(test, feature = "popup"))]
mod tests {
    use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
    use lunatui_theme::{Theme, Variant};

    use super::{Block, TitlePosition};
    use crate::{BorderType, RenderCx, Widget};

    fn draw(paint: impl Fn(&mut RenderCx<'_>)) -> Vec<(String, Vec<Style>)> {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = Grid::new(16, 5);
        let mut clusters = ClusterTable::new();
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        for y in 0..5 {
            cx.put_str(0, y, "................", Style::new());
        }
        paint(&mut cx);
        (0..5)
            .map(|y| {
                let styles = (0..16)
                    .map(|x| grid.get(x, y).map(Cell::style).unwrap_or_default())
                    .collect();
                (grid.row_display(y, &clusters), styles)
            })
            .collect()
    }

    #[test]
    fn a_borrowed_title_draws_like_an_owned_one() {
        let mut compared = 0;
        for title in ["", "Deploy", "Deploy settings", "日本語ab"] {
            for position in [TitlePosition::Top, TitlePosition::Bottom] {
                for inset in 0..4 {
                    for width in 0..14 {
                        for height in 0..4 {
                            let area = Rect::new(1, 1, width, height);
                            let block = Block::bordered()
                                .border_type(BorderType::Rounded)
                                .title_position(position)
                                .title_inset(inset)
                                .title_style(Style::new().fg(Color::Rgb(9, 8, 7)));
                            let owned = block.clone().title(title);
                            let expected = draw(|cx| owned.render(area, cx));
                            let actual = draw(|cx| block.render_with_title(area, title, cx));
                            assert_eq!(
                                actual, expected,
                                "{title:?} {position:?} inset {inset} {width}x{height}"
                            );
                            compared += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(compared, 4 * 2 * 4 * 14 * 4);
    }
}
