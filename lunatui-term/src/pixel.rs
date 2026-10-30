// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Rect;
use lunatui_image::{ImageGeometry, ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};

mod immediate;

use crate::caps::{Caps, CellPixels, GraphicsProtocol, Multiplexer};
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
    /// Send PNG files with the iTerm2 inline-image protocol.
    Iterm2,
    /// Send a palette-based Sixel raster.
    Sixel,
}

impl PixelMode {
    /// Chooses a positively reported protocol on a direct terminal connection.
    /// Multiplexers retain cells until a verified passthrough path is available.
    #[must_use]
    pub fn from_caps(caps: &Caps) -> Self {
        if caps.identity.multiplexer != Multiplexer::None {
            return Self::Cells;
        }
        match caps.protocols.graphics {
            GraphicsProtocol::Kitty => Self::Kitty,
            GraphicsProtocol::Iterm2 if caps.cell_pixels.is_some() => Self::Iterm2,
            GraphicsProtocol::Sixel if caps.cell_pixels.is_some() => Self::Sixel,
            _ => Self::Cells,
        }
    }

    /// Whether this mode draws true pixels.
    #[must_use]
    pub const fn is_pixels(self) -> bool {
        !matches!(self, Self::Cells)
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
enum ImageKey {
    Caller(u32),
    RasterSlot(u16),
}

impl ImageKey {
    fn wire_hint(self) -> u32 {
        match self {
            Self::Caller(id) => id,
            Self::RasterSlot(slot) => u32::from(slot) + 1,
        }
    }
}

#[derive(Clone, Debug)]
struct CachedRaster {
    source: ImageSource,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Uploaded {
    id: ImageKey,
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
    id: ImageKey,
    wire_id: u32,
    rect: Rect,
    z: i32,
    fresh: bool,
}

#[derive(Clone, Copy, Debug)]
struct StoredImage {
    image: Uploaded,
    wire_id: u32,
}

/// Keeps images on screen with the least traffic: an image is transmitted when its content
/// changes, placed again only when its cell rectangle or depth moves, and deleted when a frame
/// stops listing it.
///
/// Use [`finish_frame_parts`](Self::finish_frame_parts) to upload images before changing
/// cells, then present cells and image placements together. In [`PixelMode::Cells`] every
/// call is a no-op and the caller draws a cell fallback. After the first frames, nothing
/// here allocates.
#[derive(Clone, Debug)]
pub struct PixelLayer {
    mode: PixelMode,
    cell_pixels: Option<CellPixels>,
    background: Option<[u8; 3]>,
    uploaded: Vec<StoredImage>,
    retired: Vec<u32>,
    placed: Vec<Placed>,
    frame: Vec<Placed>,
    frame_pixels: u64,
    raster_slots: Vec<CachedRaster>,
    raster_cursor: usize,
    out: Vec<u8>,
    scratch: Vec<u8>,
}

impl PixelLayer {
    /// A layer with nothing on screen.
    #[must_use]
    pub fn new(mode: PixelMode) -> Self {
        Self {
            mode,
            cell_pixels: None,
            background: None,
            uploaded: Vec::new(),
            retired: Vec::new(),
            placed: Vec::new(),
            frame: Vec::new(),
            frame_pixels: 0,
            raster_slots: Vec::new(),
            raster_cursor: 0,
            out: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Builds a presenter with queried protocol, physical metrics, and background.
    #[must_use]
    pub fn from_caps(caps: &Caps) -> Self {
        let mut layer = Self::new(PixelMode::from_caps(caps));
        layer.cell_pixels = caps.cell_pixels;
        layer.background = caps.background_rgb;
        layer
    }

    /// Whether the runtime must repaint cells to remove immediate-mode images.
    #[must_use]
    pub fn needs_cell_repaint(&self) -> bool {
        matches!(self.mode, PixelMode::Iterm2 | PixelMode::Sixel)
            && (!self.placed.is_empty() || !self.frame.is_empty())
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
        self.frame_pixels = 0;
        self.raster_cursor = 0;
    }

    /// Lists `image` for this frame at the cell rectangle `at` and depth `z` (negative draws
    /// under text).
    ///
    /// `generation` identifies the content: send a new number whenever the pixels change and
    /// keep it while they do not, so unchanged images are never transmitted again. `id` must be
    /// nonzero and is listed once per frame; a zero or repeated id is ignored.
    pub fn image(&mut self, id: u32, generation: u64, image: &PixelImage<'_>, at: Rect, z: i32) {
        if !self.mode.is_pixels()
            || self.frame.len() >= 256
            || id == 0
            || at.width == 0
            || at.height == 0
            || self
                .frame
                .iter()
                .any(|placed| placed.id == ImageKey::Caller(id))
        {
            return;
        }
        let _accepted = self.present_image(ImageKey::Caller(id), generation, image, at, z);
    }

    /// Ends the frame and returns the bytes that bring the terminal up to date.
    pub fn finish_frame(&mut self) -> &[u8] {
        self.raster_slots.truncate(self.raster_cursor);
        if self.mode != PixelMode::Kitty {
            core::mem::swap(&mut self.placed, &mut self.frame);
            self.frame.clear();
            return &self.out;
        }
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
                write_delete(&mut self.out, old.wire_id);
                self.uploaded.retain(|known| known.image.id != old.id);
            }
        }
        for wire_id in self.retired.drain(..) {
            write_delete(&mut self.out, wire_id);
        }
        core::mem::swap(&mut self.placed, &mut self.frame);
        self.frame.clear();
        &self.out
    }

    /// Ends the frame and separates invisible uploads from visible updates.
    ///
    /// Write the first part before changing cells. Then write cells and the second part
    /// in one new synchronized frame. Kitty uploads can take longer than a terminal's
    /// synchronization timeout; retaining the old cells until uploads finish avoids a
    /// blank when replacing a cell fallback with pixels. Immediate protocols return an
    /// empty first part because their uploads also display the image.
    pub fn finish_frame_parts(&mut self) -> (&[u8], &[u8]) {
        let uploads = if self.mode == PixelMode::Kitty {
            self.out.len()
        } else {
            0
        };
        self.finish_frame().split_at(uploads)
    }

    /// Deletes every image, for exit or a size change, and returns the bytes to send.
    pub fn clear(&mut self) -> &[u8] {
        self.out.clear();
        for old in &self.placed {
            if self.mode == PixelMode::Kitty {
                write_delete(&mut self.out, old.wire_id);
            } else {
                for y in old.rect.y..old.rect.y.saturating_add(old.rect.height) {
                    write_cup(&mut self.out, old.rect.x, y);
                    self.out
                        .extend(std::iter::repeat_n(b' ', usize::from(old.rect.width)));
                }
            }
        }
        self.placed.clear();
        self.frame.clear();
        self.uploaded.clear();
        self.retired.clear();
        self.raster_slots.clear();
        self.raster_cursor = 0;
        &self.out
    }
}

impl PixelLayer {
    fn present_image(
        &mut self,
        id: ImageKey,
        generation: u64,
        image: &PixelImage<'_>,
        area: Rect,
        depth: i32,
    ) -> bool {
        if self.mode != PixelMode::Kitty {
            return self.immediate_image(id, image, area, depth);
        }
        let (wire_id, fresh) = self.update_upload(Uploaded {
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
            write_transmit(&mut self.out, wire_id, image);
        }
        self.frame_pixels += u64::from(image.width) * u64::from(image.height);
        self.frame.push(Placed {
            id,
            wire_id,
            rect: area,
            z: depth,
            fresh,
        });
        true
    }

    fn update_upload(&mut self, current: Uploaded) -> (u32, bool) {
        let previous = self
            .uploaded
            .iter()
            .position(|known| known.image.id == current.id);
        if let Some(index) = previous {
            let known = self.uploaded[index];
            if known.image == current {
                return (known.wire_id, false);
            }
        }
        let mut wire_id = current.id.wire_hint();
        while self.uploaded.iter().any(|known| known.wire_id == wire_id)
            || self.retired.contains(&wire_id)
        {
            wire_id = wire_id.wrapping_add(1).max(1);
        }
        let stored = StoredImage {
            image: current,
            wire_id,
        };
        if let Some(index) = previous {
            self.retired.push(self.uploaded[index].wire_id);
            self.uploaded[index] = stored;
        } else {
            self.uploaded.push(stored);
        }
        (wire_id, true)
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
        if self.mode != PixelMode::Kitty
            && crop_width
                .checked_mul(crop_height)
                .is_none_or(|n| n > 1_048_576)
        {
            return false;
        }
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
            id: ImageKey::Caller(request.id().get()),
            generation: request.generation(),
            source_width,
            source_height: source.size().height(),
            crop_x,
            crop_y,
            width: crop_width,
            height: crop_height,
            opacity: request.opacity(),
        };
        let (wire_id, fresh) = if self.mode == PixelMode::Kitty {
            self.update_upload(current)
        } else {
            (request.id().get(), true)
        };
        if fresh {
            self.scratch.clear();
            self.scratch.reserve(crop_len);
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
            if self.mode != PixelMode::Kitty {
                let pixels = self.scratch.clone();
                let Some(image) = PixelImage::new(crop_width, crop_height, &pixels) else {
                    return false;
                };
                return self.immediate_image(
                    ImageKey::Caller(request.id().get()),
                    &image,
                    destination,
                    request.depth(),
                );
            }
            write_transmit(&mut self.out, wire_id, &image);
        }
        self.frame_pixels += u64::from(crop_width) * u64::from(crop_height);
        self.frame.push(Placed {
            id: ImageKey::Caller(request.id().get()),
            wire_id,
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

    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        if !self.supports_pixels() || self.frame.len() >= 256 {
            return None;
        }
        let cell = self.cell_pixels?;
        let width = u32::from(area.width).checked_mul(u32::from(cell.width()))?;
        let height = u32::from(area.height).checked_mul(u32::from(cell.height()))?;
        if width > 4096
            || height > 4096
            || width.checked_mul(height)? > 1_048_576
            || self
                .frame_pixels
                .saturating_add(u64::from(width) * u64::from(height))
                > 4 * 1_048_576
        {
            return None;
        }
        Some(PixelTarget::new(
            PixelSize::new(width, height)?,
            self.mode == PixelMode::Kitty,
            self.background,
        ))
    }

    fn submit_raster(&mut self, source: &ImageSource, area: Rect, depth: i32) -> bool {
        let size = source.size();
        if size.width() > 4096
            || size.height() > 4096
            || size
                .width()
                .checked_mul(size.height())
                .is_none_or(|n| n > 1_048_576)
            || self
                .pixel_target(area)
                .is_none_or(|target| target.size() != size)
            || (self.mode != PixelMode::Kitty && depth < 0)
        {
            return false;
        }
        let Ok(slot) = u16::try_from(self.raster_cursor) else {
            return false;
        };
        let cached = self.raster_slots.get(self.raster_cursor);
        let generation = match cached {
            Some(cached) if cached.source == *source => cached.generation,
            Some(cached) => {
                let Some(generation) = cached.generation.checked_add(1) else {
                    return false;
                };
                generation
            }
            None => 0,
        };
        let Some(image) = PixelImage::new(size.width(), size.height(), source.rgba8_pixels())
        else {
            return false;
        };
        if !self.present_image(ImageKey::RasterSlot(slot), generation, &image, area, depth) {
            return false;
        }
        let cached = CachedRaster {
            source: source.clone(),
            generation,
        };
        if let Some(previous) = self.raster_slots.get_mut(self.raster_cursor) {
            *previous = cached;
        } else {
            self.raster_slots.push(cached);
        }
        self.raster_cursor += 1;
        true
    }

    fn submit(&mut self, request: ImageRequest<'_>) -> bool {
        let area = request.area();
        if !self.mode.is_pixels()
            || area.width == 0
            || area.height == 0
            || self.frame.len() >= 256
            || (self.mode != PixelMode::Kitty && request.depth() < 0)
        {
            return false;
        }
        self.frame
            .retain(|placed| placed.id != ImageKey::Caller(request.id().get()));
        let cell_width = self.cell_pixels.map_or(1, CellPixels::width);
        let cell_height = self.cell_pixels.map_or(2, CellPixels::height);
        let Some(viewport_height) = u32::from(area.height).checked_mul(u32::from(cell_height))
        else {
            return false;
        };
        let Some(viewport) = PixelSize::new(
            u32::from(area.width) * u32::from(cell_width),
            viewport_height,
        ) else {
            return false;
        };
        let geometry = ImageGeometry::fit(request.source().size(), viewport, request.fit());
        let source = geometry.source;
        let destination = geometry.destination;
        let top = destination.origin.y / u32::from(cell_height);
        let bottom = destination
            .origin
            .y
            .saturating_add(destination.size.height())
            .saturating_add(u32::from(cell_height) - 1)
            / u32::from(cell_height);
        let Ok(x) = u16::try_from(destination.origin.x / u32::from(cell_width)) else {
            return false;
        };
        let Ok(y) = u16::try_from(top) else {
            return false;
        };
        let Ok(width) = u16::try_from(destination.size.width().div_ceil(u32::from(cell_width)))
        else {
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
    push_decimal(out, u64::from(placed.wire_id));
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

#[cfg(test)]
mod raster_tests;
