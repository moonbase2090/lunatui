// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::{error, fmt};

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
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// The encoded input was unsupported or invalid.
    Decode(String),
}

impl fmt::Display for ImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("image dimensions must be non-zero"),
            Self::DimensionsOverflow => {
                formatter.write_str("image dimensions overflow RGBA8 storage")
            }
            Self::InvalidDataLength { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} RGBA8 bytes, received {actual}"
                )
            }
            Self::TooLarge {
                max_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "decoded image is {actual_bytes} bytes; maximum is {max_bytes}"
            ),
            Self::Decode(message) => write!(formatter, "image decode failed: {message}"),
        }
    }
}

impl error::Error for ImageError {}

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

    /// Decodes the first frame of a PNG, JPEG, GIF, or WebP input.
    ///
    /// This method is available with the `decode` feature. It checks encoded
    /// dimensions before decoding and applies the same decoded RGBA byte bound
    /// as [`rgba8`](Self::rgba8). Animated inputs currently produce their first
    /// frame; frame scheduling belongs to the application.
    ///
    /// # Errors
    ///
    /// Returns an error when the format is unsupported or invalid, its dimensions
    /// overflow or exceed the decoded byte bound, or the decoder rejects the data.
    #[cfg(feature = "decode")]
    pub fn decode(encoded: &[u8]) -> Result<Self, ImageError> {
        use std::io::Cursor;

        use image::{ImageFormat, ImageReader};

        let mut reader = ImageReader::new(Cursor::new(encoded))
            .with_guessed_format()
            .map_err(|error| ImageError::Decode(error.to_string()))?;
        let format = reader
            .format()
            .filter(|format| {
                matches!(
                    format,
                    ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP
                )
            })
            .ok_or_else(|| ImageError::Decode("expected PNG, JPEG, GIF, or WebP".to_owned()))?;
        reader.limits(decode_limits());
        let dimensions = reader
            .into_dimensions()
            .map_err(|error| ImageError::Decode(error.to_string()))?;
        let expected = rgba8_len(dimensions.0, dimensions.1)?;
        if expected > MAX_RGBA8_BYTES {
            return Err(ImageError::TooLarge {
                max_bytes: MAX_RGBA8_BYTES,
                actual_bytes: expected,
            });
        }
        let mut reader = ImageReader::with_format(Cursor::new(encoded), format);
        reader.limits(decode_limits());
        let image = reader
            .decode()
            .map_err(|error| ImageError::Decode(error.to_string()))?;
        Self::rgba8(
            dimensions.0,
            dimensions.1,
            image.into_rgba8().into_raw().into_boxed_slice(),
        )
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

#[cfg(feature = "decode")]
fn rgba8_len(width: u32, height: u32) -> Result<usize, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::Empty);
    }
    usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ImageError::DimensionsOverflow)
}

#[cfg(feature = "decode")]
fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(u32::try_from(MAX_RGBA8_BYTES / 4).unwrap_or(u32::MAX));
    limits.max_image_height = Some(u32::try_from(MAX_RGBA8_BYTES / 4).unwrap_or(u32::MAX));
    limits.max_alloc = Some(u64::try_from(MAX_RGBA8_BYTES).unwrap_or(u64::MAX));
    limits
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

#[cfg(all(test, feature = "decode"))]
mod decode_tests {
    use image::ImageEncoder;

    use super::{ImageError, ImageSource};

    #[test]
    fn decodes_png_pixels_without_changing_orientation() -> Result<(), ImageError> {
        let mut encoded = Vec::new();
        image::codecs::png::PngEncoder::new(&mut encoded)
            .write_image(
                &[255, 0, 0, 255, 0, 0, 255, 128],
                1,
                2,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| ImageError::Decode(error.to_string()))?;
        let decoded = ImageSource::decode(&encoded)?;
        assert_eq!(decoded.size().width(), 1);
        assert_eq!(decoded.size().height(), 2);
        assert_eq!(decoded.rgba8_pixels(), &[255, 0, 0, 255, 0, 0, 255, 128]);
        Ok(())
    }

    #[test]
    fn rejects_unsupported_and_invalid_encoded_data() {
        assert!(matches!(
            ImageSource::decode(b"not an image"),
            Err(ImageError::Decode(_))
        ));
        assert!(matches!(
            ImageSource::decode(b"BM\0\0\0\0\0\0\0\0\0\0"),
            Err(ImageError::Decode(_))
        ));
    }
}
