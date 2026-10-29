// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Rect, Size};

/// The side of its anchor a floating hint or panel, such as a [`Tooltip`](crate::Tooltip), tries
/// first.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnchorSide {
    /// Under the anchor, then over it, then right, then left.
    #[default]
    Below,
    /// Over the anchor, then under it, then right, then left.
    Above,
    /// Right of the anchor, then left, then under, then over.
    Right,
    /// Left of the anchor, then right, then under, then over.
    Left,
}

impl AnchorSide {
    const fn order(self) -> [Self; 4] {
        match self {
            Self::Below => [Self::Below, Self::Above, Self::Right, Self::Left],
            Self::Above => [Self::Above, Self::Below, Self::Right, Self::Left],
            Self::Right => [Self::Right, Self::Left, Self::Below, Self::Above],
            Self::Left => [Self::Left, Self::Right, Self::Below, Self::Above],
        }
    }
}

pub(crate) fn place(anchor: Rect, size: Size, area: Rect, first: AnchorSide) -> Option<Rect> {
    first
        .order()
        .into_iter()
        .find_map(|edge| beside(anchor, size, area, edge))
}

fn beside(anchor: Rect, size: Size, area: Rect, edge: AnchorSide) -> Option<Rect> {
    let right = area.x.saturating_add(area.width);
    let bottom = area.y.saturating_add(area.height);
    let last_x = right.checked_sub(size.width).filter(|x| *x >= area.x)?;
    let last_y = bottom.checked_sub(size.height).filter(|y| *y >= area.y)?;
    let along_x = anchor.x.clamp(area.x, last_x);
    let along_y = anchor.y.clamp(area.y, last_y);
    let after_x = anchor.x.saturating_add(anchor.width);
    let after_y = anchor.y.saturating_add(anchor.height);
    let (x, y) = match edge {
        AnchorSide::Below => (along_x, after_y),
        AnchorSide::Above => (along_x, anchor.y.checked_sub(size.height)?),
        AnchorSide::Right => (after_x, along_y),
        AnchorSide::Left => (anchor.x.checked_sub(size.width)?, along_y),
    };
    (x >= area.x && x <= last_x && y >= area.y && y <= last_y)
        .then(|| Rect::new(x, y, size.width, size.height))
}
