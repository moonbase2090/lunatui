// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::num::NonZeroU32;

use lunatui_core::Rect;

use crate::{Fit, ImageSource};

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

/// Receives image requests during the same widget render pass as their cell fallback.
pub trait ImageSink {
    /// True when this frame can present images using a pixel protocol.
    fn supports_pixels(&self) -> bool;

    /// Accepts an image for this frame. Returns false when the sink cannot present it.
    fn submit(&mut self, request: ImageRequest<'_>) -> bool;
}
