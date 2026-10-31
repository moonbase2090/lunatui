// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::ops::RangeInclusive;

use lunatui_core::{Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::render_cx::clip_to_coordinates;
use crate::{RenderCx, Widget, WidgetState};

const BANDS: [Token; 5] = [
    Token::Surface1,
    Token::Accent,
    Token::Ok,
    Token::Warn,
    Token::Fail,
];

/// One labeled row of borrowed readings. `None` is missing; `Some(0)` is measured zero.
#[derive(Clone, Copy, Debug)]
pub struct HeatmapRow<'a> {
    label: &'a str,
    values: &'a [Option<u64>],
}

impl<'a> HeatmapRow<'a> {
    /// Borrows a label and its readings. An empty label needs no label gutter.
    #[must_use]
    pub const fn new(label: &'a str, values: &'a [Option<u64>]) -> Self {
        Self { label, values }
    }
}

/// A logical matrix position, independent of terminal coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HeatmapCell {
    /// Zero-based row index.
    pub row: usize,
    /// Zero-based column index.
    pub column: usize,
}

/// A borrowed matrix drawn as fixed-size colored blocks with optional labels.
///
/// The longest row sets the column count. Short rows and `None` readings are missing cells,
/// drawn with a centered `·`. Present readings use five theme bands: surface, accent, ok,
/// warning, and failure. The automatic scale uses present readings only; a constant scale
/// uses the first band. [`range`](Self::range) fixes the scale for live data.
///
/// Selection belongs to the application. A selected cell retains its value color and gains
/// a `◆`, using the focus ring when focused. Disabled maps use dimmed foreground swatches
/// instead of colored backgrounds and suppress selection. Cells clip rather than shrink.
///
/// ```
/// use lunatui_core::Rect;
/// use lunatui_widgets::{Heatmap, HeatmapCell, HeatmapRow};
/// let readings = [Some(0), None, Some(100)];
/// let rows = [HeatmapRow::new("", &readings)];
/// let map = Heatmap::new(&rows).range(0..=100);
/// assert_eq!(map.shape(), (1, 3));
/// assert_eq!(map.cell_at(Rect::new(0, 0, 6, 1), 3, 0),
///            Some(HeatmapCell { row: 0, column: 1 }));
/// ```
#[derive(Clone, Debug)]
pub struct Heatmap<'a> {
    rows: &'a [HeatmapRow<'a>],
    labels: &'a [&'a str],
    cell_size: Size,
    range: Option<(u64, u64)>,
    selected: Option<HeatmapCell>,
    state: WidgetState,
}

impl<'a> Heatmap<'a> {
    /// Borrows rows, with two-column by one-row cells and an automatic scale.
    #[must_use]
    pub const fn new(rows: &'a [HeatmapRow<'a>]) -> Self {
        Self {
            rows,
            labels: &[],
            cell_size: Size::new(2, 1),
            range: None,
            selected: None,
            state: WidgetState::Normal,
        }
    }

    /// Reserves a header row. Labels clip to their cells; extra labels are ignored.
    #[must_use]
    pub const fn column_labels(mut self, labels: &'a [&'a str]) -> Self {
        self.labels = labels;
        self
    }

    /// Uses fixed terminal-cell dimensions. Each zero dimension becomes one.
    #[must_use]
    pub fn cell_size(mut self, size: Size) -> Self {
        self.cell_size = Size::new(size.width.max(1), size.height.max(1));
        self
    }

    /// Sets an inclusive scale, sorting reversed bounds and clamping readings to the bounds.
    #[must_use]
    pub fn range(mut self, range: RangeInclusive<u64>) -> Self {
        let (first, last) = (*range.start(), *range.end());
        self.range = Some((first.min(last), first.max(last)));
        self
    }

    /// Marks an app-owned logical position. Positions outside the matrix are ignored.
    #[must_use]
    pub const fn selected(mut self, cell: Option<HeatmapCell>) -> Self {
        self.selected = cell;
        self
    }

    /// Sets normal, focused, or disabled presentation without changing selection.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Returns the row count and the longest row's column count, including missing readings.
    #[must_use]
    pub fn shape(&self) -> (usize, usize) {
        (
            self.rows.len(),
            self.rows
                .iter()
                .map(|row| row.values.len())
                .max()
                .unwrap_or(0),
        )
    }

    /// Maps a terminal coordinate to a visible cell, including missing and partial cells.
    /// Label gutters and coordinates outside `area` return `None`.
    #[must_use]
    pub fn cell_at(&self, area: Rect, x: u16, y: u16) -> Option<HeatmapCell> {
        let layout = self.layout(area);
        let x = x.checked_sub(layout.data.x)?;
        let y = y.checked_sub(layout.data.y)?;
        if x >= layout.data.width || y >= layout.data.height {
            return None;
        }
        let cell = HeatmapCell {
            row: usize::from(y / self.cell_size.height),
            column: usize::from(x / self.cell_size.width),
        };
        layout.cell_area(cell).map(|_| cell)
    }

    /// Returns a cell's visible rectangle. Hidden or out-of-matrix cells return `None`.
    #[must_use]
    pub fn cell_area(&self, area: Rect, cell: HeatmapCell) -> Option<Rect> {
        self.layout(area).cell_area(cell)
    }

    fn label_width(&self) -> u16 {
        let width = self
            .rows
            .iter()
            .map(|row| width(row.label))
            .max()
            .unwrap_or(0);
        u16::try_from(width.saturating_add(usize::from(width > 0))).unwrap_or(u16::MAX)
    }

    fn layout(&self, area: Rect) -> MatrixLayout {
        let area = clip_to_coordinates(area);
        let gutter = self.label_width().min(area.width);
        let header = u16::from(!self.labels.is_empty()).min(area.height);
        let (rows, columns) = self.shape();
        MatrixLayout {
            area,
            data: Rect::new(
                area.x + gutter,
                area.y + header,
                area.width - gutter,
                area.height - header,
            ),
            rows,
            columns,
            cell_size: self.cell_size,
        }
    }

    fn scale(&self) -> (u64, u64) {
        self.range.unwrap_or_else(|| {
            let mut readings = self
                .rows
                .iter()
                .flat_map(|row| row.values.iter().flatten())
                .copied();
            let first = readings.next().unwrap_or(0);
            readings.fold((first, first), |(low, high), value| {
                (low.min(value), high.max(value))
            })
        })
    }

    fn render_cell(&self, cell: HeatmapCell, area: Rect, scale: (u64, u64), cx: &mut RenderCx<'_>) {
        let value = self.rows[cell.row]
            .values
            .get(cell.column)
            .copied()
            .flatten();
        let background = cx.token(Token::Surface0);
        let mut style = Style::new().fg(cx.token(Token::TextPrimary)).bg(background);
        let disabled = self.state == WidgetState::Disabled;
        let glyph = if let Some(value) = value {
            let color = cx.token(BANDS[band(value, scale)]);
            if disabled {
                style = style.fg(color);
                '░'
            } else {
                style = style.bg(color);
                ' '
            }
        } else {
            ' '
        };
        if disabled {
            style.attrs = style.attrs.set_dim(true);
        }
        cx.fill(area, style, glyph);
        let selected = self.selected == Some(cell) && !disabled;
        let marker = if selected {
            Some("◆")
        } else if value.is_none() {
            Some("·")
        } else {
            None
        };
        if let Some(marker) = marker {
            if selected && self.state == WidgetState::Focused {
                style = style.fg(cx.token(Token::FocusRing));
                style.attrs = style.attrs.set_bold(true);
            }
            cx.put_str(
                area.x + (area.width - 1) / 2,
                area.y + (area.height - 1) / 2,
                marker,
                style,
            );
        }
    }
}

impl Widget for Heatmap<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let layout = self.layout(area);
        let mut text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        if self.state == WidgetState::Disabled {
            text.attrs = text.attrs.set_dim(true);
        }
        cx.fill(layout.area, text, ' ');
        let scale = self.scale();
        let rows = usize::from(layout.data.height.div_ceil(self.cell_size.height)).min(layout.rows);
        let columns =
            usize::from(layout.data.width.div_ceil(self.cell_size.width)).min(layout.columns);
        for row in 0..rows {
            for column in 0..columns {
                let cell = HeatmapCell { row, column };
                if let Some(area) = layout.cell_area(cell) {
                    self.render_cell(cell, area, scale, cx);
                    if column == 0 {
                        let width = layout
                            .data
                            .x
                            .saturating_sub(layout.area.x)
                            .saturating_sub(1);
                        cx.put_str(
                            layout.area.x,
                            area.y,
                            clip_display_prefix(self.rows[row].label, width),
                            text,
                        );
                    }
                    if row == 0
                        && let Some(label) = self.labels.get(column)
                    {
                        cx.put_str(
                            area.x,
                            layout.area.y,
                            clip_display_prefix(label, area.width),
                            text,
                        );
                    }
                }
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let (rows, columns) = self.shape();
        if columns == 0 {
            return Size::new(0, 0);
        }
        let width = columns
            .saturating_mul(usize::from(self.cell_size.width))
            .saturating_add(usize::from(self.label_width()));
        let height = rows
            .saturating_mul(usize::from(self.cell_size.height))
            .saturating_add(usize::from(!self.labels.is_empty()));
        Size::new(
            u16::try_from(width).unwrap_or(u16::MAX),
            u16::try_from(height).unwrap_or(u16::MAX),
        )
        .clamp(avail)
    }
}

struct MatrixLayout {
    area: Rect,
    data: Rect,
    rows: usize,
    columns: usize,
    cell_size: Size,
}

impl MatrixLayout {
    fn cell_area(&self, cell: HeatmapCell) -> Option<Rect> {
        if cell.row >= self.rows || cell.column >= self.columns {
            return None;
        }
        let x = u16::try_from(cell.column.checked_mul(usize::from(self.cell_size.width))?).ok()?;
        let y = u16::try_from(cell.row.checked_mul(usize::from(self.cell_size.height))?).ok()?;
        if x >= self.data.width || y >= self.data.height {
            return None;
        }
        Some(Rect::new(
            self.data.x + x,
            self.data.y + y,
            self.cell_size.width.min(self.data.width - x),
            self.cell_size.height.min(self.data.height - y),
        ))
    }
}

fn band(value: u64, (low, high): (u64, u64)) -> usize {
    if low == high {
        return 0;
    }
    usize::try_from((u128::from(value.clamp(low, high) - low) * 5 / u128::from(high - low)).min(4))
        .unwrap_or(0)
}
