// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, clip_display_prefix};
use lunatui_ui_tokens::{BORDER, Role, Spacing, Tone, surface, text};
use lunatui_widgets::{Block, Padding, RenderCx, Scrollbar, Table, Widget};

pub use lunatui_widgets::{Alignment, Constraint, RowSource, SortDirection, TableState, VecSource};

/// One column of a [`DataTable`]: a header title, a layout width and an alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column<'a> {
    title: &'a str,
    width: Constraint,
    alignment: Alignment,
}

impl<'a> Column<'a> {
    /// Left-aligned column with header `title` and layout width `width`.
    #[must_use]
    pub const fn new(title: &'a str, width: Constraint) -> Self {
        Self {
            title,
            width,
            alignment: Alignment::Left,
        }
    }

    /// How cell text sits inside the column.
    #[must_use]
    pub const fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

/// Bordered data table over a [`RowSource`], styled from `lunatui-ui-tokens`.
///
/// Only the rows that fit are read from the source. With a [`TableState`] the table shows
/// the state's sorted view of the source, with its cursor, selection, sort marker and
/// column widths; call [`sync`](Self::sync) with the render area to keep the state in
/// range. Without a state, [`offset`](Self::offset) and [`selected`](Self::selected)
/// position the view directly.
pub struct DataTable<'a, S> {
    source: S,
    columns: Vec<Column<'a>>,
    state: Option<&'a TableState>,
    offset: usize,
    selected: Option<usize>,
    on: Role,
    bordered: bool,
    striped: bool,
    scrollbar: bool,
    empty_text: &'a str,
}

impl<'a, S: RowSource> DataTable<'a, S> {
    /// Table over `source` with one [`Column`] per source column.
    #[must_use]
    pub fn new(source: S, columns: Vec<Column<'a>>) -> Self {
        Self {
            source,
            columns,
            state: None,
            offset: 0,
            selected: None,
            on: Role::Background,
            bordered: true,
            striped: true,
            scrollbar: true,
            empty_text: "No data",
        }
    }

    /// Shows `state`'s sorted view, cursor, selection, sort marker and column widths.
    /// Takes over from [`offset`](Self::offset) and [`selected`](Self::selected).
    #[must_use]
    pub fn state(mut self, state: &'a TableState) -> Self {
        self.state = Some(state);
        self
    }

    /// First visible row, when no [`TableState`] is set.
    #[must_use]
    pub const fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Row drawn as the cursor row, when no [`TableState`] is set.
    #[must_use]
    pub const fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Surface the table sits on, so its cells keep that background.
    #[must_use]
    pub const fn on(mut self, role: Role) -> Self {
        self.on = role;
        self
    }

    /// Whether the rounded frame is drawn. It is by default.
    #[must_use]
    pub const fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Whether odd rows sit on the alternate surface. They do by default.
    #[must_use]
    pub const fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Whether a scrollbar is drawn on the right edge when the rows overflow. It is by
    /// default, and never when every row fits.
    #[must_use]
    pub const fn scrollbar(mut self, scrollbar: bool) -> Self {
        self.scrollbar = scrollbar;
        self
    }

    /// Muted line shown under the header when the source is empty; `""` shows nothing.
    #[must_use]
    pub const fn empty_text(mut self, empty_text: &'a str) -> Self {
        self.empty_text = empty_text;
        self
    }

    /// How many data rows fit in `area`, under the header and inside the frame.
    #[must_use]
    pub fn data_rows(&self, area: Rect) -> usize {
        let inner = self.frame().inner(area);
        usize::from(inner.height.saturating_sub(1))
    }

    /// Records the row count and the data rows that fit in `area` in `state`, keeping its
    /// cursor, selection and offset in range.
    pub fn sync(&self, area: Rect, state: &mut TableState) {
        state.fit(self.source.len(), self.data_rows(area));
    }

    fn frame(&self) -> Block {
        let block = if self.bordered {
            Block::bordered().border_type(BORDER)
        } else {
            Block::new()
        };
        block.padding(Padding {
            left: Spacing::S1.y(),
            right: Spacing::S1.y(),
            top: Spacing::S0.y(),
            bottom: Spacing::S0.y(),
        })
    }

    fn effective_offset(&self) -> usize {
        self.state.map_or(self.offset, TableState::offset)
    }

    fn table<T: RowSource>(&self, source: T, cx: &RenderCx<'_>) -> Table<T> {
        let base = surface(cx, self.on);
        let header = base
            .fg(text(cx, Role::Muted).fg)
            .with_attrs(Attrs::new().set_bold(true));
        let mut table = Table::new(
            source,
            self.columns.iter().map(|column| column.width).collect(),
        )
        .style(base)
        .highlight_style(Tone::Primary.style(cx))
        .selection_style(base.bg(cx.token(Role::Accent.token())))
        .header(self.columns.iter().map(|column| (column.title, header)))
        .alignments(self.columns.iter().map(|column| column.alignment).collect());
        if self.striped {
            let alt = if matches!(self.on, Role::Card | Role::Popover) {
                Role::Background
            } else {
                Role::Card
            };
            table = table.odd_row_style(surface(cx, alt));
        }
        match self.state {
            Some(state) => table.state(state),
            None => table.offset(self.offset).selected(self.selected),
        }
    }
}

impl<S: RowSource> Widget for DataTable<'_, S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let base = surface(cx, self.on);
        let frame = self
            .frame()
            .border_style(base.fg(text(cx, Role::Border).fg))
            .style(base);
        let inner = frame.inner(area);
        frame.render(area, cx);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let rows = self.source.len();
        let visible = usize::from(inner.height.saturating_sub(1));
        let bar = self.scrollbar && rows > visible && inner.width > 1 && visible > 0;
        let table_area = Rect::new(
            inner.x,
            inner.y,
            inner.width.saturating_sub(u16::from(bar)),
            inner.height,
        );
        match self.state {
            Some(state) => self
                .table(state.view(&self.source), cx)
                .render(table_area, cx),
            None => self.table(&self.source, cx).render(table_area, cx),
        }
        if rows == 0 && !self.empty_text.is_empty() && inner.height > 1 {
            let muted = base.fg(text(cx, Role::Muted).fg);
            cx.put_str(
                table_area.x,
                inner.y + 1,
                clip_display_prefix(self.empty_text, table_area.width),
                muted,
            );
        }
        if bar {
            Scrollbar::vertical(rows, visible, self.effective_offset())
                .track_style(base.fg(text(cx, Role::Border).fg))
                .thumb_style(base.fg(text(cx, Role::Primary).fg))
                .render(
                    Rect::new(inner.x + inner.width - 1, inner.y, 1, inner.height),
                    cx,
                );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let frame_rows = u16::from(self.bordered) * 2;
        let body = u16::try_from(self.source.len()).unwrap_or(u16::MAX);
        let height = frame_rows.saturating_add(1).saturating_add(body.max(1));
        Size::new(avail.width, height).clamp(avail)
    }
}
