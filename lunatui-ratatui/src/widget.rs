// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::cell::RefCell;

use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_widgets::{RenderCx, Widget};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect as RatRect;
use ratatui::widgets::{StatefulWidget, Widget as RatWidget};

use crate::convert::{blit_grid_to_cx, buffer_digest};
use crate::memo::WidgetMemo;

/// Adapts a ratatui [`RatWidget`] into a lunatui [`Widget`].
///
/// Renders into a scratch `ratatui::Buffer`, converts non-[`Skip`](ratatui::buffer::CellDiffOption::Skip)
/// cells into the lunatui grid, and memoizes the conversion when the scratch buffer is unchanged.
#[derive(Debug)]
pub struct RatatuiWidget<W> {
    inner: W,
    memo: RefCell<WidgetMemo>,
}

impl<W> RatatuiWidget<W> {
    /// Wraps a ratatui widget for use inside lunatui layouts.
    #[must_use]
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            memo: RefCell::new(WidgetMemo {
                width: 0,
                height: 0,
                digest: 0,
                skip_mask: Vec::new(),
                grid: Grid::new(0, 0),
                clusters: ClusterTable::new(),
                conversions: 0,
                hits: 0,
            }),
        }
    }

    /// Borrow the inner ratatui widget.
    #[must_use]
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Memoized conversion count (scratch buffer changed since last cache).
    #[must_use]
    pub fn memo_conversions(&self) -> u32 {
        self.memo.borrow().conversions
    }

    /// Memo hit count (scratch matched cache; conversion skipped).
    #[must_use]
    pub fn memo_hits(&self) -> u32 {
        self.memo.borrow().hits
    }
}

impl<W> Widget for RatatuiWidget<W>
where
    W: RatWidget + Clone,
{
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let rat_area = RatRect::new(0, 0, area.width, area.height);
        let mut scratch = Buffer::empty(rat_area);
        self.inner.clone().render(rat_area, &mut scratch);
        let digest = buffer_digest(&scratch);
        let mut memo = self.memo.borrow_mut();
        memo.clear_if_size(area.width, area.height);
        if memo.digest == digest && digest != 0 {
            memo.hits += 1;
            blit_grid_to_cx(
                &memo.grid,
                &memo.clusters,
                &memo.skip_mask,
                area,
                cx,
            );
            return;
        }
        memo.refresh_from_buffer(&scratch);
        memo.digest = digest;
        memo.conversions += 1;
        blit_grid_to_cx(
            &memo.grid,
            &memo.clusters,
            &memo.skip_mask,
            area,
            cx,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

/// Adapts a ratatui [`StatefulWidget`] with local state.
pub struct RatatuiStatefulWidget<W>
where
    W: StatefulWidget,
    W::State: Sized,
{
    inner: W,
    state: RefCell<W::State>,
    memo: RefCell<WidgetMemo>,
}

impl<W> RatatuiStatefulWidget<W>
where
    W: StatefulWidget,
    W::State: Sized,
{
    /// Wraps a ratatui stateful widget and its state.
    #[must_use]
    pub fn new(inner: W, state: W::State) -> Self {
        Self {
            inner,
            state: RefCell::new(state),
            memo: RefCell::new(WidgetMemo {
                width: 0,
                height: 0,
                digest: 0,
                skip_mask: Vec::new(),
                grid: Grid::new(0, 0),
                clusters: ClusterTable::new(),
                conversions: 0,
                hits: 0,
            }),
        }
    }

    /// Borrow the inner widget.
    #[must_use]
    pub fn inner(&self) -> &W {
        &self.inner
    }

    /// Mutable access to ratatui widget state.
    pub fn state_mut(&self) -> std::cell::RefMut<'_, W::State> {
        self.state.borrow_mut()
    }

    /// Memoized conversion count.
    #[must_use]
    pub fn memo_conversions(&self) -> u32 {
        self.memo.borrow().conversions
    }

    /// Memo hit count.
    #[must_use]
    pub fn memo_hits(&self) -> u32 {
        self.memo.borrow().hits
    }
}

impl<W> Widget for RatatuiStatefulWidget<W>
where
    W: StatefulWidget + Clone,
    W::State: Sized,
{
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let rat_area = RatRect::new(0, 0, area.width, area.height);
        let mut scratch = Buffer::empty(rat_area);
        self.inner
            .clone()
            .render(rat_area, &mut scratch, &mut *self.state.borrow_mut());
        let digest = buffer_digest(&scratch);
        let mut memo = self.memo.borrow_mut();
        memo.clear_if_size(area.width, area.height);
        if memo.digest == digest && digest != 0 {
            memo.hits += 1;
            blit_grid_to_cx(
                &memo.grid,
                &memo.clusters,
                &memo.skip_mask,
                area,
                cx,
            );
            return;
        }
        memo.refresh_from_buffer(&scratch);
        memo.digest = digest;
        memo.conversions += 1;
        blit_grid_to_cx(
            &memo.grid,
            &memo.clusters,
            &memo.skip_mask,
            area,
            cx,
        );
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}
