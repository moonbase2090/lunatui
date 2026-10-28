// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;
use lunatui_image::{
    Fit, ImageGeometry, ImageRequest, ImageSink, PixelPoint, PixelRect, PixelSize,
};

use crate::caps::{Caps, GraphicsProtocol, Multiplexer};
use crate::encode::write_cup;

const MAX_CHUNK_BYTES: usize = 3072;
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// How images reach the screen.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PixelMode {
    /// Draw with cells only; the pixel layer emits nothing.
    #[default]
    Cells,
    /// Send true pixels with the kitty graphics protocol.
    Kitty,
}

impl PixelMode {
    /// Chooses pixels only when the terminal answered the kitty graphics query and no multiplexer
    /// sits between the program and the terminal, since tmux and screen do not forward graphics.
    ///
    /// Prismattyc native pixels need a protocol extension that is not specified yet, so a native
    /// terminal without the kitty reply stays on cells.
    #[must_use]
    pub fn from_caps(caps: &Caps) -> Self {
        let direct = caps.identity.multiplexer == Multiplexer::None;
        if direct && caps.protocols.graphics == GraphicsProtocol::Kitty {
            Self::Kitty
        } else {
            Self::Cells
        }
    }

    /// Whether this mode draws true pixels.
    #[must_use]
    pub const fn is_pixels(self) -> bool {
        matches!(self, Self::Kitty)
    }
}

/// A borrowed RGBA8 image, four bytes per pixel, rows top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelImage<'a> {
    width: u32,
    height: u32,
    rgba: &'a [u8],
}

impl<'a> PixelImage<'a> {
    /// Wraps `rgba`, or returns `None` when a side is zero or the length is not `width * height * 4`.
    #[must_use]
    pub fn new(width: u32, height: u32, rgba: &'a [u8]) -> Option<Self> {
        let pixels = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?;
        if width == 0 || height == 0 || rgba.len() != pixels.checked_mul(4)? {
            return None;
        }
        Some(Self {
            width,
            height,
            rgba,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Uploaded {
    id: u32,
    generation: u64,
    source_width: u32,
    source_height: u32,
    crop_x: u32,
    crop_y: u32,
    width: u32,
    height: u32,
    opacity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placed {
    id: u32,
    rect: Rect,
    z: i32,
    fresh: bool,
}

/// Keeps images on screen with the least traffic: an image is transmitted when its content
/// changes, placed again only when its cell rectangle or depth moves, and deleted when a frame
/// stops listing it.
///
/// Write the bytes from [`finish_frame`](Self::finish_frame) to the terminal after the frame's
/// cell output. In [`PixelMode::Cells`] every call is a no-op and the caller draws a cell
/// fallback. After the first frames, nothing here allocates.
#[derive(Clone, Debug)]
pub struct PixelLayer {
    mode: PixelMode,
    uploaded: Vec<Uploaded>,
    placed: Vec<Placed>,
    frame: Vec<Placed>,
    out: Vec<u8>,
    scratch: Vec<u8>,
}

impl PixelLayer {
    /// A layer with nothing on screen.
    #[must_use]
    pub fn new(mode: PixelMode) -> Self {
        Self {
            mode,
            uploaded: Vec::new(),
            placed: Vec::new(),
            frame: Vec::new(),
            out: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// The mode this layer was built for.
    #[must_use]
    pub const fn mode(&self) -> PixelMode {
        self.mode
    }

    /// Starts a frame; call before the first [`image`](Self::image).
    pub fn begin_frame(&mut self) {
        self.out.clear();
        self.frame.clear();
    }

    /// Lists `image` for this frame at the cell rectangle `at` and depth `z` (negative draws
    /// under text).
    ///
    /// `generation` identifies the content: send a new number whenever the pixels change and
    /// keep it while they do not, so unchanged images are never transmitted again. `id` must be
    /// nonzero and is listed once per frame; a zero or repeated id is ignored.
    pub fn image(&mut self, id: u32, generation: u64, image: &PixelImage<'_>, at: Rect, z: i32) {
        if !self.mode.is_pixels()
            || id == 0
            || at.width == 0
            || at.height == 0
            || self.frame.iter().any(|placed| placed.id == id)
        {
            return;
        }
        let fresh = self.update_upload(Uploaded {
            id,
            generation,
            source_width: image.width,
            source_height: image.height,
            crop_x: 0,
            crop_y: 0,
            width: image.width,
            height: image.height,
            opacity: u8::MAX,
        });
        if fresh {
            write_transmit(&mut self.out, id, image);
        }
        self.frame.push(Placed {
            id,
            rect: at,
            z,
            fresh,
        });
    }

    /// Ends the frame and returns the bytes that bring the terminal up to date.
    pub fn finish_frame(&mut self) -> &[u8] {
        for placed in &self.frame {
            let unchanged = !placed.fresh
                && self
                    .placed
                    .iter()
                    .any(|old| old.id == placed.id && old.rect == placed.rect && old.z == placed.z);
            if !unchanged {
                write_place(&mut self.out, placed);
            }
        }
        for old in &self.placed {
            if !self.frame.iter().any(|placed| placed.id == old.id) {
                write_delete(&mut self.out, old.id);
                self.uploaded.retain(|known| known.id != old.id);
            }
        }
        core::mem::swap(&mut self.placed, &mut self.frame);
        self.frame.clear();
        &self.out
    }

    /// Deletes every image, for exit or a size change, and returns the bytes to send.
    pub fn clear(&mut self) -> &[u8] {
        self.out.clear();
        for old in &self.placed {
            write_delete(&mut self.out, old.id);
        }
        self.placed.clear();
        self.frame.clear();
        self.uploaded.clear();
        &self.out
    }
}

impl PixelLayer {
    fn update_upload(&mut self, current: Uploaded) -> bool {
        match self
            .uploaded
            .iter_mut()
            .find(|known| known.id == current.id)
        {
            Some(known) if *known == current => false,
            Some(known) => {
                write_delete(&mut self.out, current.id);
                *known = current;
                true
            }
            None => {
                self.uploaded.push(current);
                true
            }
        }
    }

    fn submit_cropped(
        &mut self,
        request: ImageRequest<'_>,
        crop_x: u32,
        crop_y: u32,
        crop_width: u32,
        crop_height: u32,
        destination: Rect,
    ) -> bool {
        let source = request.source();
        let Some(crop_len) = usize::try_from(crop_width)
            .ok()
            .and_then(|width| {
                usize::try_from(crop_height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
        else {
            return false;
        };
        let Some(row_len) = usize::try_from(crop_width)
            .ok()
            .and_then(|width| width.checked_mul(4))
        else {
            return false;
        };
        let source_width = source.size().width();
        let source_bytes = source.rgba8_pixels();
        let row_offset = |row: u32| {
            usize::try_from(row)
                .ok()?
                .checked_mul(usize::try_from(source_width).ok()?)?
                .checked_mul(4)
        };
        let current = Uploaded {
            id: request.id().get(),
            generation: request.generation(),
            source_width,
            source_height: source.size().height(),
            crop_x,
            crop_y,
            width: crop_width,
            height: crop_height,
            opacity: request.opacity(),
        };
        let fresh = self.update_upload(current);
        if fresh {
            self.scratch.clear();
            if self.scratch.capacity() < crop_len {
                self.scratch.reserve(crop_len);
            }
            for row in crop_y..crop_y.saturating_add(crop_height) {
                let Some(offset) = row_offset(row).and_then(|offset| {
                    offset.checked_add(usize::try_from(crop_x).ok()?.checked_mul(4)?)
                }) else {
                    return false;
                };
                let Some(end) = offset.checked_add(row_len) else {
                    return false;
                };
                let Some(bytes) = source_bytes.get(offset..end) else {
                    return false;
                };
                self.scratch.extend_from_slice(bytes);
            }
            if request.opacity() != u8::MAX {
                for alpha in self.scratch.iter_mut().skip(3).step_by(4) {
                    *alpha = u8::try_from(
                        u16::from(*alpha) * u16::from(request.opacity()) / u16::from(u8::MAX),
                    )
                    .unwrap_or(u8::MAX);
                }
            }
            let Some(image) = PixelImage::new(crop_width, crop_height, &self.scratch) else {
                return false;
            };
            write_transmit(&mut self.out, request.id().get(), &image);
        }
        self.frame.push(Placed {
            id: request.id().get(),
            rect: destination,
            z: request.depth(),
            fresh,
        });
        true
    }
}

impl ImageSink for PixelLayer {
    fn supports_pixels(&self) -> bool {
        self.mode.is_pixels()
    }

    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        let area = request.area();
        if !self.mode.is_pixels() || area.width == 0 || area.height == 0 {
            return false;
        }
        self.frame.retain(|placed| placed.id != request.id().get());
        let Some(viewport_height) = u32::from(area.height).checked_mul(2) else {
            return false;
        };
        let Some(viewport) = PixelSize::new(u32::from(area.width), viewport_height) else {
            return false;
        };
        let geometry = ImageGeometry::fit(request.source().size(), viewport, request.fit());
        let source = if request.fit() == Fit::Cover {
            cover_source(request.source().size(), viewport)
        } else {
            geometry.source
        };
        let destination = geometry.destination;
        let top = destination.origin.y / 2;
        let bottom = destination
            .origin
            .y
            .saturating_add(destination.size.height())
            .saturating_add(1)
            / 2;
        let Ok(x) = u16::try_from(destination.origin.x) else {
            return false;
        };
        let Ok(y) = u16::try_from(top) else {
            return false;
        };
        let Ok(width) = u16::try_from(destination.size.width()) else {
            return false;
        };
        let Ok(height) = u16::try_from(bottom.saturating_sub(top)) else {
            return false;
        };
        let placement = Rect::new(
            area.x.saturating_add(x),
            area.y.saturating_add(y),
            width,
            height,
        );
        self.submit_cropped(
            request,
            source.origin.x,
            source.origin.y,
            source.size.width(),
            source.size.height(),
            placement,
        )
    }
}

fn cover_source(source: PixelSize, viewport: PixelSize) -> PixelRect {
    let source_is_wider = u64::from(source.width()) * u64::from(viewport.height())
        > u64::from(source.height()) * u64::from(viewport.width());
    if source_is_wider {
        let width = rounded_ratio(source.height(), viewport.width(), viewport.height())
            .clamp(1, source.width());
        PixelRect {
            origin: PixelPoint {
                x: (source.width() - width) / 2,
                y: 0,
            },
            size: PixelSize::new(width, source.height()).unwrap_or(source),
        }
    } else {
        let height = rounded_ratio(source.width(), viewport.height(), viewport.width())
            .clamp(1, source.height());
        PixelRect {
            origin: PixelPoint {
                x: 0,
                y: (source.height() - height) / 2,
            },
            size: PixelSize::new(source.width(), height).unwrap_or(source),
        }
    }
}

fn rounded_ratio(value: u32, numerator: u32, denominator: u32) -> u32 {
    let product = u64::from(value) * u64::from(numerator);
    let rounded = product + u64::from(denominator / 2);
    u32::try_from(rounded / u64::from(denominator)).unwrap_or(u32::MAX)
}

fn write_transmit(out: &mut Vec<u8>, id: u32, image: &PixelImage<'_>) {
    let mut chunks = image.rgba.chunks(MAX_CHUNK_BYTES).peekable();
    let mut first = true;
    while let Some(chunk) = chunks.next() {
        out.extend_from_slice(b"\x1b_G");
        if first {
            out.extend_from_slice(b"a=t,i=");
            push_decimal(out, u64::from(id));
            out.extend_from_slice(b",f=32,s=");
            push_decimal(out, u64::from(image.width));
            out.extend_from_slice(b",v=");
            push_decimal(out, u64::from(image.height));
            out.extend_from_slice(b",q=2");
            if chunks.peek().is_some() {
                out.extend_from_slice(b",m=1");
            }
        } else if chunks.peek().is_some() {
            out.extend_from_slice(b"m=1");
        } else {
            out.extend_from_slice(b"m=0");
        }
        out.push(b';');
        push_base64(out, chunk);
        out.extend_from_slice(b"\x1b\\");
        first = false;
    }
}

fn write_place(out: &mut Vec<u8>, placed: &Placed) {
    write_cup(out, placed.rect.x, placed.rect.y);
    out.extend_from_slice(b"\x1b_Ga=p,i=");
    push_decimal(out, u64::from(placed.id));
    out.extend_from_slice(b",p=1,c=");
    push_decimal(out, u64::from(placed.rect.width));
    out.extend_from_slice(b",r=");
    push_decimal(out, u64::from(placed.rect.height));
    out.extend_from_slice(b",z=");
    if placed.z < 0 {
        out.push(b'-');
    }
    push_decimal(out, u64::from(placed.z.unsigned_abs()));
    out.extend_from_slice(b",C=1,q=2\x1b\\");
}

fn write_delete(out: &mut Vec<u8>, id: u32) {
    out.extend_from_slice(b"\x1b_Ga=d,d=I,i=");
    push_decimal(out, u64::from(id));
    out.extend_from_slice(b",q=2\x1b\\");
}

fn push_decimal(out: &mut Vec<u8>, value: u64) {
    let mut digits = [0_u8; 20];
    let mut len = 0;
    let mut rest = value;
    loop {
        digits[len] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        len += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    while len > 0 {
        len -= 1;
        out.push(digits[len]);
    }
}

fn push_base64(out: &mut Vec<u8>, data: &[u8]) {
    for group in data.chunks(3) {
        let first = u32::from(group[0]);
        let second = u32::from(group.get(1).copied().unwrap_or(0));
        let third = u32::from(group.get(2).copied().unwrap_or(0));
        let word = first << 16 | second << 8 | third;
        let sextet =
            |shift: u32| BASE64[usize::from(u8::try_from((word >> shift) & 63).unwrap_or(0))];
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if group.len() > 1 { sextet(6) } else { b'=' });
        out.push(if group.len() > 2 { sextet(0) } else { b'=' });
    }
}

#[cfg(test)]
mod tests;
