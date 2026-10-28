// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

/// Non-zero pixel dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelSize {
    width: u32,
    height: u32,
}

impl PixelSize {
    /// Creates dimensions, returning `None` if either side is zero.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        Some(Self { width, height })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// Pixel fitting behavior for an image viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Fit {
    /// Preserve the full image and center it, leaving unused viewport space.
    #[default]
    Contain,
    /// Fill the viewport while preserving aspect ratio, cropping the source.
    Cover,
    /// Fill the viewport without preserving the source aspect ratio.
    Stretch,
    /// Keep source dimensions and crop to the viewport, aligned at the top left.
    Native,
}

/// Pixel-space point relative to a viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelPoint {
    /// Horizontal coordinate.
    pub x: u32,
    /// Vertical coordinate.
    pub y: u32,
}

/// Non-empty pixel rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelRect {
    /// Top-left coordinate relative to the destination viewport.
    pub origin: PixelPoint,
    /// Rectangle dimensions.
    pub size: PixelSize,
}

/// Shared source crop and destination placement for a fitted image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageGeometry {
    /// Source pixels to sample.
    pub source: PixelRect,
    /// Destination pixels within the requested viewport.
    pub destination: PixelRect,
}

impl ImageGeometry {
    /// Fits `source` inside a `viewport` using `fit`.
    ///
    /// The returned rectangles are relative to their respective bounds.
    /// `Contain` and `Cover` use nearest-integer aspect calculations, with exact
    /// ties rounded up. All products fit in `u64` for `u32` input dimensions.
    #[must_use]
    pub fn fit(source: PixelSize, viewport: PixelSize, fit: Fit) -> Self {
        match fit {
            Fit::Stretch => Self {
                source: rect(0, 0, source),
                destination: rect(0, 0, viewport),
            },
            Fit::Native => {
                let width = source.width().min(viewport.width());
                let height = source.height().min(viewport.height());
                let cropped = size(width, height);
                Self {
                    source: rect(0, 0, cropped),
                    destination: rect(0, 0, cropped),
                }
            }
            Fit::Contain => contain(source, viewport),
            Fit::Cover => cover(source, viewport),
        }
    }
}

fn contain(source: PixelSize, viewport: PixelSize) -> ImageGeometry {
    let width_limited = u64::from(viewport.width()) * u64::from(source.height())
        <= u64::from(viewport.height()) * u64::from(source.width());
    let (width, height) = if width_limited {
        let height = rounded_ratio(source.height(), viewport.width(), source.width())
            .clamp(1, viewport.height());
        (viewport.width(), height)
    } else {
        let width = rounded_ratio(source.width(), viewport.height(), source.height())
            .clamp(1, viewport.width());
        (width, viewport.height())
    };
    let destination_size = size(width, height);
    ImageGeometry::geometry(
        rect(0, 0, source),
        rect(
            (viewport.width() - width) / 2,
            (viewport.height() - height) / 2,
            destination_size,
        ),
    )
}

fn cover(source: PixelSize, viewport: PixelSize) -> ImageGeometry {
    let source_is_wider = u64::from(source.width()) * u64::from(viewport.height())
        > u64::from(source.height()) * u64::from(viewport.width());
    let source_size = if source_is_wider {
        let width = rounded_ratio(source.height(), viewport.width(), viewport.height())
            .clamp(1, source.width());
        size(width, source.height())
    } else {
        let height = rounded_ratio(source.width(), viewport.height(), viewport.width())
            .clamp(1, source.height());
        size(source.width(), height)
    };
    let source_rect = rect(
        (source.width() - source_size.width()) / 2,
        (source.height() - source_size.height()) / 2,
        source_size,
    );
    ImageGeometry::geometry(source_rect, rect(0, 0, viewport))
}

impl ImageGeometry {
    fn geometry(source: PixelRect, destination: PixelRect) -> Self {
        Self {
            source,
            destination,
        }
    }
}

fn rounded_ratio(value: u32, numerator: u32, denominator: u32) -> u32 {
    let product = u64::from(value) * u64::from(numerator);
    let rounded = product + u64::from(denominator / 2);
    u32::try_from(rounded / u64::from(denominator)).unwrap_or(u32::MAX)
}

fn size(width: u32, height: u32) -> PixelSize {
    PixelSize {
        width: width.max(1),
        height: height.max(1),
    }
}

fn rect(x: u32, y: u32, size: PixelSize) -> PixelRect {
    PixelRect {
        origin: PixelPoint { x, y },
        size,
    }
}

#[cfg(test)]
mod tests {
    use super::{Fit, ImageGeometry, PixelSize};

    fn px(width: u32, height: u32) -> PixelSize {
        PixelSize {
            width: width.max(1),
            height: height.max(1),
        }
    }

    #[test]
    fn contain_centers_and_preserves_full_source() {
        let geometry = ImageGeometry::fit(px(16, 9), px(20, 20), Fit::Contain);
        assert_eq!(geometry.source.origin.x, 0);
        assert_eq!(geometry.source.origin.y, 0);
        assert_eq!(geometry.destination.origin.x, 0);
        assert_eq!(geometry.destination.origin.y, 4);
        assert_eq!(geometry.destination.size, px(20, 11));
    }

    #[test]
    fn cover_crops_centered_source_to_viewport_aspect() {
        let geometry = ImageGeometry::fit(px(16, 9), px(9, 9), Fit::Cover);
        assert_eq!(geometry.source.origin.x, 3);
        assert_eq!(geometry.source.size, px(9, 9));
        assert_eq!(geometry.destination.origin.x, 0);
        assert_eq!(geometry.destination.size, px(9, 9));
    }

    #[test]
    fn stretch_uses_both_entire_rectangles() {
        let geometry = ImageGeometry::fit(px(16, 9), px(7, 13), Fit::Stretch);
        assert_eq!(geometry.source.size, px(16, 9));
        assert_eq!(geometry.destination.size, px(7, 13));
    }

    #[test]
    fn native_crops_at_top_left_without_scaling() {
        let geometry = ImageGeometry::fit(px(16, 9), px(7, 13), Fit::Native);
        assert_eq!(geometry.source.origin.x, 0);
        assert_eq!(geometry.source.origin.y, 0);
        assert_eq!(geometry.source.size, px(7, 9));
        assert_eq!(geometry.destination.size, px(7, 9));
    }

    #[test]
    fn extreme_dimensions_keep_fit_inside_viewport() {
        let geometry = ImageGeometry::fit(px(u32::MAX, 1), px(1, u32::MAX), Fit::Contain);
        assert!(geometry.destination.size.width() <= 1);
        assert_eq!(geometry.destination.size.height(), 1);
    }
}
