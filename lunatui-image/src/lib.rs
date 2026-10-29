// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod geometry;
mod presentation;
mod source;

pub use geometry::{Fit, ImageGeometry, PixelPoint, PixelRect, PixelSize};
pub use presentation::{ImageId, ImageRequest, ImageSink, PixelTarget};
pub use source::{ImageError, ImageSource, MAX_RGBA8_BYTES};
