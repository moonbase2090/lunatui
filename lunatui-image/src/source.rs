// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::PixelSize;

/// Maximum decoded RGBA8 source size accepted by [`ImageSource::rgba8`].
pub const MAX_RGBA8_BYTES: usize = 64 * 1024 * 1024;

/// An immutable decoded RGBA8 image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageSource(Arc<SourceData>);

#[derive(Debug, PartialEq, Eq)]
struct SourceData {
    size: PixelSize,
    rgba: Box<[u8]>,
}

/// Validation failure when constructing an [`ImageSource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// Width or height was zero.
    Empty,
    /// Pixel count or byte count overflowed the supported integer range.
    DimensionsOverflow,
    /// The byte slice did not contain exactly four bytes per pixel.
    InvalidDataLength {
        /// Required RGBA byte length.
        expected: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// The image exceeds [`MAX_RGBA8_BYTES`].
    TooLarge {
        /// Maximum accepted decoded size.
        max_bytes: usize,
        /// Required decoded size.
        actual_bytes: usize,
    },
}

impl ImageSource {
    /// Creates a source from top-to-bottom RGBA8 pixel data.
    ///
    /// The pixel buffer is shared cheaply when the source is cloned.
    ///
    /// # Errors
    ///
    /// Returns an error if either dimension is zero, the expected byte length
    /// overflows, the buffer length is incorrect, or it exceeds the configured
    /// maximum.
    pub fn rgba8(width: u32, height: u32, rgba: Box<[u8]>) -> Result<Self, ImageError> {
        let size = PixelSize::new(width, height).ok_or(ImageError::Empty)?;
        let expected = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ImageError::DimensionsOverflow)?;
        if expected > MAX_RGBA8_BYTES {
            return Err(ImageError::TooLarge {
                max_bytes: MAX_RGBA8_BYTES,
                actual_bytes: expected,
            });
        }
        if rgba.len() != expected {
            return Err(ImageError::InvalidDataLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self(Arc::new(SourceData { size, rgba })))
    }

    /// Pixel dimensions of the image.
    #[must_use]
    pub fn size(&self) -> PixelSize {
        self.0.size
    }

    /// Borrowed top-to-bottom RGBA8 pixels.
    #[must_use]
    pub fn rgba8_pixels(&self) -> &[u8] {
        &self.0.rgba
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageError, ImageSource, MAX_RGBA8_BYTES};

    #[test]
    fn validates_dimensions_and_exact_pixel_length() {
        assert_eq!(
            ImageSource::rgba8(0, 1, Box::new([])),
            Err(ImageError::Empty)
        );
        assert_eq!(
            ImageSource::rgba8(1, 0, Box::new([])),
            Err(ImageError::Empty)
        );
        assert_eq!(
            ImageSource::rgba8(2, 1, vec![1, 2, 3].into_boxed_slice()),
            Err(ImageError::InvalidDataLength {
                expected: 8,
                actual: 3,
            })
        );
        assert_eq!(
            ImageSource::rgba8(u32::MAX, u32::MAX, Box::new([])),
            Err(ImageError::DimensionsOverflow)
        );
    }

    #[test]
    fn checks_limit_before_retaining_pixels() {
        let width = u32::try_from(MAX_RGBA8_BYTES / 4).unwrap_or(u32::MAX);
        assert_eq!(
            ImageSource::rgba8(width, 2, Box::new([])),
            Err(ImageError::TooLarge {
                max_bytes: MAX_RGBA8_BYTES,
                actual_bytes: MAX_RGBA8_BYTES * 2,
            })
        );
    }

    #[test]
    fn clones_share_immutable_pixels() -> Result<(), ImageError> {
        let image = ImageSource::rgba8(1, 1, vec![4, 3, 2, 1].into_boxed_slice())?;
        let clone = image.clone();
        assert_eq!(image.size().width(), 1);
        assert_eq!(clone.rgba8_pixels(), &[4, 3, 2, 1]);
        assert!(std::sync::Arc::ptr_eq(&image.0, &clone.0));
        Ok(())
    }
}
