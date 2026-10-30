// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::num::NonZeroU32;

use lunatui_core::Rect;

use crate::{Fit, ImageSource, PixelSize};

/// Stable, nonzero identity for one image placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(NonZeroU32);

impl ImageId {
    /// Creates an image identity. Zero is reserved by terminal image protocols.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Numeric identity used by terminal image protocols.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// One image's request to the active terminal presenter.
#[derive(Clone, Copy, Debug)]
pub struct ImageRequest<'a> {
    id: ImageId,
    generation: u64,
    source: &'a ImageSource,
    area: Rect,
    fit: Fit,
    depth: i32,
    opacity: u8,
}

impl<'a> ImageRequest<'a> {
    /// Creates a request for one frame. Keep `id` stable and increment
    /// `generation` whenever the source pixels change.
    #[must_use]
    pub const fn new(
        id: ImageId,
        generation: u64,
        source: &'a ImageSource,
        area: Rect,
        fit: Fit,
        depth: i32,
        opacity: u8,
    ) -> Self {
        Self {
            id,
            generation,
            source,
            area,
            fit,
            depth,
            opacity,
        }
    }

    /// Stable placement identity.
    #[must_use]
    pub const fn id(self) -> ImageId {
        self.id
    }

    /// Source generation used to invalidate the encoded image cache.
    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }

    /// Immutable decoded source pixels.
    #[must_use]
    pub const fn source(self) -> &'a ImageSource {
        self.source
    }

    /// Cell rectangle occupied by the image.
    #[must_use]
    pub const fn area(self) -> Rect {
        self.area
    }

    /// Source fitting policy.
    #[must_use]
    pub const fn fit(self) -> Fit {
        self.fit
    }

    /// Relative drawing depth, where a smaller value is behind text.
    #[must_use]
    pub const fn depth(self) -> i32 {
        self.depth
    }

    /// Semantic image opacity, with 255 fully opaque.
    #[must_use]
    pub const fn opacity(self) -> u8 {
        self.opacity
    }
}

/// Physical raster size and composition support reported by an image presenter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelTarget {
    size: PixelSize,
    text_underlay: bool,
    background: Option<[u8; 3]>,
}

impl PixelTarget {
    /// Describes measured pixels, text-underlay support, and the terminal default background.
    #[must_use]
    pub const fn new(size: PixelSize, text_underlay: bool, background: Option<[u8; 3]>) -> Self {
        Self {
            size,
            text_underlay,
            background,
        }
    }

    /// Measured raster dimensions for the requested cell rectangle.
    #[must_use]
    pub const fn size(self) -> PixelSize {
        self.size
    }

    /// Whether negative-depth images can preserve text above the image rectangle.
    #[must_use]
    pub const fn text_underlay(self) -> bool {
        self.text_underlay
    }

    /// Queried terminal default background, when available.
    #[must_use]
    pub const fn background(self) -> Option<[u8; 3]> {
        self.background
    }
}

/// Receives image requests during the same widget render pass as their cell fallback.
pub trait ImageSink {
    /// True when this frame can present images using a pixel protocol.
    fn supports_pixels(&self) -> bool;

    /// Returns measured raster information, or `None` when unavailable or over budget.
    /// Custom sinks can omit this method and continue accepting ordinary images.
    fn pixel_target(&self, _area: Rect) -> Option<PixelTarget> {
        None
    }

    /// Presents a widget raster without caller-managed identity or revisions.
    ///
    /// The source must match the measured pixel target for `area`. Identities follow accepted
    /// raster submission order within each complete presenter frame. Submit every visible
    /// raster each frame; omitted rasters are removed. Equal decoded pixels suppress Kitty
    /// uploads, while image generation and lifetime remain owned by the presenter.
    /// Custom sinks can decline this operation and retain the widget's cell fallback.
    fn submit_raster(&mut self, _source: &ImageSource, _area: Rect, _depth: i32) -> bool {
        false
    }

    /// Accepts an image for this frame. Returns false when the sink cannot present it.
    fn submit(&mut self, request: ImageRequest<'_>) -> bool;
}
