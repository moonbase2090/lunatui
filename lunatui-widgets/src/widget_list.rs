// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Grid, Rect, Size, Style};

use crate::{RenderCx, Widget};

/// Scroll and selection state of a [`WidgetList`], kept by the app between frames.
///
/// `offset` is the first visible item and `skip` is how many rows of that item are scrolled
/// off the top, so a tall item can scroll in and out of view one row at a time.
/// [`WidgetList::scrolled`] moves both so the selected item is on screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetListState {
    /// Focused item.
    pub selected: Option<usize>,
    /// First visible item.
    pub offset: usize,
    /// Rows of the first visible item hidden above the area.
    pub skip: u16,
}

impl WidgetListState {
    /// Nothing selected, scrolled to the top.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected: None,
            offset: 0,
            skip: 0,
        }
    }

    /// Selects the next item of `len`, or the first one when nothing is selected. The last item
    /// stays selected.
    pub fn select_next(&mut self, len: usize) {
        self.selected = match (self.selected, len) {
            (_, 0) => None,
            (None, _) => Some(0),
            (Some(index), _) => Some((index + 1).min(len - 1)),
        };
    }

    /// Selects the previous item, or the last one of `len` when nothing is selected. The first
    /// item stays selected.
    pub fn select_previous(&mut self, len: usize) {
        self.selected = match (self.selected, len) {
            (_, 0) => None,
            (None, _) => Some(len - 1),
            (Some(index), _) => Some(index.saturating_sub(1).min(len - 1)),
        };
    }
}

/// One item of a [`WidgetList`] as placed in an area, for hit testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemArea {
    /// Item index.
    pub index: usize,
    /// The visible part of the item.
    pub rect: Rect,
    /// Rows of the item cut off above `rect`.
    pub skip: u16,
    /// The item's full height.
    pub height: u16,
}

/// Vertical list whose items are different widgets with different heights.
///
/// `height(index, width)` gives the rows item `index` needs at `width`, and
/// `draw(index, area, selected, cx)` paints it into an `area` of exactly that size. Items at the
/// edges of the area are drawn whole off-screen and clipped, so a tall item scrolls in and out
/// row by row. Cells an item leaves with the default background take the list or highlight
/// background. A height of zero hides an item.
///
/// # Example
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect, Style};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{RenderCx, Widget, WidgetList, WidgetListState};
///
/// let heights = [1u16, 3, 2];
/// let list = WidgetList::new(
///     heights.len(),
///     |index, _width| heights[index],
///     |index, area, _selected, cx: &mut RenderCx<'_>| {
///         for row in 0..area.height {
///             cx.put_str(area.x, area.y + row, &format!("{index}.{row}"), Style::new());
///         }
///     },
/// )
/// .state(WidgetListState { selected: Some(2), offset: 0, skip: 0 });
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(4, 4);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// list.render(Rect::new(0, 0, 4, 4), &mut cx);
/// // Six rows of items in a four-row area: the selected item scrolls fully into view.
/// assert_eq!(grid.row_display(0, &clusters).trim_end(), "1.1");
/// assert_eq!(grid.row_display(3, &clusters).trim_end(), "2.1");
/// ```
pub struct WidgetList<H, D> {
    len: usize,
    height: H,
    draw: D,
    style: Style,
    highlight: Style,
    state: WidgetListState,
    scroll_padding: u16,
}

impl<H, D> WidgetList<H, D>
where
    H: Fn(usize, u16) -> u16,
    D: Fn(usize, Rect, bool, &mut RenderCx<'_>),
{
    /// A list of `len` items.
    #[must_use]
    pub fn new(len: usize, height: H, draw: D) -> Self {
        Self {
            len,
            height,
            draw,
            style: Style::new(),
            highlight: Style::new(),
            state: WidgetListState::new(),
            scroll_padding: 0,
        }
    }

    /// Style of the cells behind the items.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style filled behind the selected item before it is drawn.
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = style;
        self
    }

    /// Scroll position and selection.
    #[must_use]
    pub fn state(mut self, state: WidgetListState) -> Self {
        self.state = state;
        self
    }

    /// Rows kept visible past the selected item when scrolling reveals it, so the next item
    /// shows before the selection reaches the edge.
    #[must_use]
    pub fn scroll_padding(mut self, rows: u16) -> Self {
        self.scroll_padding = rows;
        self
    }

    fn item_height(&self, index: usize, width: u16) -> u16 {
        (self.height)(index, width)
    }

    /// The state with `offset` and `skip` moved the least that puts the selected item in `area`.
    /// An item taller than the area shows its top. Persist the result so the next frame scrolls
    /// from here.
    #[must_use]
    pub fn scrolled(&self, area: Rect) -> WidgetListState {
        let mut state = self.state;
        if self.len == 0 || area.width == 0 || area.height == 0 {
            return WidgetListState::new();
        }
        state.offset = state.offset.min(self.len - 1);
        let Some(selected) = state.selected.map(|index| index.min(self.len - 1)) else {
            return state;
        };
        state.selected = Some(selected);
        let selected_height = self.item_height(selected, area.width);
        if selected <= state.offset {
            state.offset = selected;
            state.skip = 0;
            return self.pad_top(state, area);
        }
        let padding = self
            .scroll_padding
            .min(area.height.saturating_sub(selected_height) / 2);
        let mut bottom = 0u32;
        for index in state.offset..=selected {
            bottom += u32::from(self.item_height(index, area.width));
        }
        bottom = bottom.saturating_sub(u32::from(state.skip));
        if selected_height >= area.height {
            state.offset = selected;
            state.skip = 0;
            return state;
        }
        let want = bottom + u32::from(self.padding_below(selected, area.width, padding));
        let mut excess = want.saturating_sub(u32::from(area.height));
        while excess > 0 && state.offset < selected {
            let remaining = self
                .item_height(state.offset, area.width)
                .saturating_sub(state.skip);
            if u32::from(remaining) <= excess {
                excess -= u32::from(remaining);
                state.offset += 1;
                state.skip = 0;
            } else {
                state.skip += u16::try_from(excess).unwrap_or(u16::MAX);
                excess = 0;
            }
        }
        state
    }

    fn padding_below(&self, selected: usize, width: u16, padding: u16) -> u16 {
        let mut rows = 0u16;
        let mut index = selected + 1;
        while rows < padding && index < self.len {
            rows = rows.saturating_add(self.item_height(index, width));
            index += 1;
        }
        rows.min(padding)
    }

    fn pad_top(&self, mut state: WidgetListState, area: Rect) -> WidgetListState {
        let mut rows = 0u16;
        let padding = self.scroll_padding.min(area.height / 2);
        while rows < padding && state.offset > 0 {
            let above = self.item_height(state.offset - 1, area.width);
            if above == 0 {
                state.offset -= 1;
                continue;
            }
            let take = above.min(padding - rows);
            rows += take;
            state.offset -= 1;
            state.skip = above - take;
        }
        state
    }

    /// Where each visible item sits in `area`, top to bottom, after [`scrolled`](Self::scrolled).
    #[must_use]
    pub fn item_areas(&self, area: Rect) -> Vec<ItemArea> {
        let state = self.scrolled(area);
        let mut placed = Vec::new();
        if self.len == 0 || area.width == 0 || area.height == 0 {
            return placed;
        }
        let mut y = 0u16;
        let mut index = state.offset;
        let mut skip = state.skip;
        while y < area.height && index < self.len {
            let height = self.item_height(index, area.width);
            let shown = height.saturating_sub(skip).min(area.height - y);
            if shown > 0 {
                placed.push(ItemArea {
                    index,
                    rect: Rect::new(area.x, area.y + y, area.width, shown),
                    skip,
                    height,
                });
                y += shown;
            }
            skip = 0;
            index += 1;
        }
        placed
    }
}

impl<H, D> Widget for WidgetList<H, D>
where
    H: Fn(usize, u16) -> u16,
    D: Fn(usize, Rect, bool, &mut RenderCx<'_>),
{
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.fill(area, self.style, ' ');
        let selected = self.scrolled(area).selected;
        for item in self.item_areas(area) {
            let is_selected = selected == Some(item.index);
            if is_selected {
                cx.fill(item.rect, self.highlight, ' ');
            }
            let base = if is_selected {
                self.highlight
            } else {
                self.style
            };
            if item.skip == 0 && item.rect.height == item.height {
                (self.draw)(item.index, item.rect, is_selected, cx);
                cx.default_background(item.rect, base.bg);
                continue;
            }
            let mut scratch = Grid::new(item.rect.width, item.height);
            cx.offscreen(&mut scratch, |inner| {
                let full = Rect::new(0, 0, item.rect.width, item.height);
                inner.fill(full, base, ' ');
                (self.draw)(item.index, full, is_selected, inner);
                inner.default_background(full, base.bg);
            });
            cx.blit(
                &scratch,
                Rect::new(0, item.skip, item.rect.width, item.rect.height),
                item.rect.x,
                item.rect.y,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let mut rows = 0u16;
        for index in 0..self.len {
            rows = rows.saturating_add(self.item_height(index, avail.width));
            if rows >= avail.height {
                return Size::new(avail.width, avail.height);
            }
        }
        Size::new(avail.width, rows)
    }
}
