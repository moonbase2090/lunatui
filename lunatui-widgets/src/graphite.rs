//! Backend-neutral adapters for Graphite's pixel-based render contracts.

use core::num::NonZeroU16;

use lunatui_core::Rect as CellRect;

/// Terminal cell dimensions in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellSize {
    width_px: NonZeroU16,
    height_px: NonZeroU16,
}

impl CellSize {
    /// Creates a cell size with nonzero pixel dimensions.
    #[must_use]
    pub const fn new(width_px: NonZeroU16, height_px: NonZeroU16) -> Self {
        Self {
            width_px,
            height_px,
        }
    }

    /// Cell width in pixels.
    #[must_use]
    pub const fn width_px(self) -> u16 {
        self.width_px.get()
    }

    /// Cell height in pixels.
    #[must_use]
    pub const fn height_px(self) -> u16 {
        self.height_px.get()
    }
}

/// A Graphite-style rectangle measured in pixels, local to a cell area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Left pixel coordinate.
    pub x: usize,
    /// Top pixel coordinate.
    pub y: usize,
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
}

impl Rect {
    /// Converts this pixel rectangle to cells, rounding outward and clipping to `area`.
    ///
    /// Pixel coordinates are local to `area`; the returned rectangle uses the grid's
    /// absolute cell coordinates. Empty or wholly clipped rectangles return `None`.
    #[must_use]
    pub fn to_cell_rect(self, area: CellRect, cell_size: CellSize) -> Option<CellRect> {
        if self.w == 0 || self.h == 0 || area.width == 0 || area.height == 0 {
            return None;
        }
        let right = self.x.checked_add(self.w)?;
        let bottom = self.y.checked_add(self.h)?;
        let cw = usize::from(cell_size.width_px());
        let ch = usize::from(cell_size.height_px());
        let left_cell = self.x / cw;
        let top_cell = self.y / ch;
        let right_cell = right.checked_add(cw - 1)? / cw;
        let bottom_cell = bottom.checked_add(ch - 1)? / ch;
        let area_w = usize::from(area.width);
        let area_h = usize::from(area.height);
        let x0 = left_cell.min(area_w);
        let y0 = top_cell.min(area_h);
        let x1 = right_cell.min(area_w);
        let y1 = bottom_cell.min(area_h);
        if x0 >= x1 || y0 >= y1 {
            return None;
        }
        Some(CellRect::new(
            area.x.checked_add(u16::try_from(x0).ok()?)?,
            area.y.checked_add(u16::try_from(y0).ok()?)?,
            u16::try_from(x1 - x0).ok()?,
            u16::try_from(y1 - y0).ok()?,
        ))
    }
}

/// Text style face understood by Graphite's text measurement contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// Regular-weight proportional text.
    Regular,
    /// Semi-bold proportional text.
    SemiBold,
    /// Monospaced text.
    Mono,
}

/// Measures text in pixel units.
pub trait TextMetrics {
    /// Returns the text width at the requested face and font size, in pixels.
    fn width(&self, face: Face, px: f32, text: &str) -> f32;
}

/// Terminal-cell text metrics using Lunatui's grapheme-aware display width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellTextMetrics {
    cell_size: CellSize,
}

impl CellTextMetrics {
    /// Creates text metrics for the supplied terminal cell dimensions.
    #[must_use]
    pub const fn new(cell_size: CellSize) -> Self {
        Self { cell_size }
    }
}

impl TextMetrics for CellTextMetrics {
    fn width(&self, _face: Face, _px: f32, text: &str) -> f32 {
        lunatui_core::width(text) as f32 * f32::from(self.cell_size.width_px())
    }
}
