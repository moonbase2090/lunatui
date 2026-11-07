// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fmt;

use lunatui_core::{Cell, ClusterId, ClusterTable, Color, DecodedGlyph, Grid};

use crate::native::ansi_fallback_repaint_prefix;

/// Ring ABI minor version that carries alpha and sub-cell attributes.
pub const RING_ABI_MINOR_ALPHA: u16 = 1;

/// Bit of the ACTIVE response header flags with which a host affirms `native.alpha.v1`.
pub const ACTIVE_FLAG_ALPHA: u16 = 1;

/// Bytes in one ring slot.
pub const SLOT_BYTES: usize = 1_048_576;

/// Most cells one frame may hold.
pub const MAX_FRAME_CELLS: usize = 32_768;

/// Most columns or rows one frame may hold.
pub const MAX_FRAME_AXIS: u16 = 4096;

/// Bytes in the frame header.
pub const FRAME_HEADER_BYTES: usize = 128;

/// Bytes in one attribute record of ring ABI 1.1.
pub const ATTRIBUTE_RECORD_BYTES: usize = 8;

const CELL_BYTES: usize = 16;
const CLUSTER_RECORD_BYTES: usize = 16;
const MAX_CLUSTER_BYTES: usize = 1024;
const MAX_BLOB_BYTES: usize = 262_144;
const CONTINUATION: u32 = 0x0011_0000;
const CLUSTER_TAG: u32 = 0x8000_0000;
const FULL_SNAPSHOT: u16 = 1;
const KIND_INDEXED: u32 = 1 << 30;
const KIND_RGB: u32 = 2 << 30;

/// Whether frames carry alpha and sub-cell attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub enum AttributeMode {
    /// Attributes are removed before encoding. The frame is a plain ring ABI 1.0 frame.
    #[default]
    Stripped,
    /// Attributes are encoded as ring ABI 1.1 attribute records.
    Native,
}

/// Why a frame could not be encoded. Every error ends native output for the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// Columns or rows are zero or above [`MAX_FRAME_AXIS`], or the cells exceed
    /// [`MAX_FRAME_CELLS`].
    Geometry,
    /// The encoded frame does not fit one slot, or the cluster text does not fit its blob.
    TooLarge,
    /// The generation is zero.
    Generation,
    /// The cursor lies outside the grid.
    Cursor,
    /// A cell uses a flag, a color or an attribute that ring ABI 1.0 cannot carry.
    Unsupported,
    /// A cell holds a control character.
    Control,
    /// A cell refers to a cluster the table does not hold.
    Malformed,
    /// A theme token has no resolved color.
    UnresolvedToken,
    /// The session already ended; the caller repaints with ANSI.
    Detached,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Geometry => "frame geometry outside the ring limits",
            Self::TooLarge => "frame does not fit one ring slot",
            Self::Generation => "generation must be nonzero",
            Self::Cursor => "cursor outside the grid",
            Self::Unsupported => "cell content ring ABI 1.0 cannot carry",
            Self::Control => "control character in a cell",
            Self::Malformed => "cell refers to a cluster the table does not hold",
            Self::UnresolvedToken => "theme token without a resolved color",
            Self::Detached => "native output already ended",
        })
    }
}

impl std::error::Error for EncodeError {}

/// Writes complete native grid frames in the wire format of `docs/native-protocol.md`.
///
/// The encoder starts with attributes stripped, so every frame is a ring ABI 1.0 frame. Call
/// [`offer_alpha`](Self::offer_alpha) with the discovery result, send
/// [`hello_ring_minor`](Self::hello_ring_minor) in HELLO, then call
/// [`affirm_alpha`](Self::affirm_alpha) with the flags of the host's ACTIVE response; only an
/// affirmative reply turns attributes on. A failed encode or any other session failure ends
/// native output through [`fail`](Self::fail).
#[derive(Clone, Debug, Default)]
pub struct FrameEncoder {
    mode: AttributeMode,
    offered: bool,
    detached: bool,
    palette: Vec<Color>,
    bytes: Vec<u8>,
}

impl FrameEncoder {
    /// An encoder with attributes stripped and no theme palette.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the colors theme tokens resolve to, indexed by token id.
    pub fn set_token_palette(&mut self, palette: Vec<Color>) {
        self.palette = palette;
    }

    /// The current attribute mode.
    pub fn mode(&self) -> AttributeMode {
        self.mode
    }

    /// Whether the session ended and the caller repaints with ANSI.
    #[must_use]
    pub fn is_detached(&self) -> bool {
        self.detached
    }

    /// Records whether the host's capability reply listed `native.alpha.v1`
    /// ([`DiscoveryResult::alpha_offered`](crate::DiscoveryResult::alpha_offered)).
    ///
    /// Call it before HELLO. Without an offer the client never leaves ABI 1.0: it sends ring
    /// minor 0 and an ACTIVE reply with flags is not accepted.
    pub fn offer_alpha(&mut self, offered: bool) {
        self.offered = offered;
        self.mode = AttributeMode::Stripped;
    }

    /// The ring ABI minor version to put in HELLO: [`RING_ABI_MINOR_ALPHA`] when the host
    /// offered `native.alpha.v1`, otherwise 0.
    #[must_use]
    pub fn hello_ring_minor(&self) -> u16 {
        if self.offered {
            RING_ABI_MINOR_ALPHA
        } else {
            0
        }
    }

    /// Applies the flags of the host's ACTIVE response and returns the resulting mode.
    ///
    /// Attributes turn on only when the host offered the feature, so HELLO carried ring minor
    /// 1, and the flags are exactly [`ACTIVE_FLAG_ALPHA`]. Any other reply keeps attributes
    /// stripped.
    pub fn affirm_alpha(&mut self, active_flags: u16) -> AttributeMode {
        self.mode = if self.offered && active_flags == ACTIVE_FLAG_ALPHA && !self.detached {
            AttributeMode::Native
        } else {
            AttributeMode::Stripped
        };
        self.mode
    }

    /// Ends native output: attributes are stripped, later frames fail with
    /// [`EncodeError::Detached`], and the returned bytes reset the terminal and clear it so the
    /// caller can repaint the whole screen with ANSI.
    pub fn fail(&mut self) -> &'static [u8] {
        self.detached = true;
        self.mode = AttributeMode::Stripped;
        ansi_fallback_repaint_prefix()
    }

    /// Encodes `grid` as one full-snapshot slot payload.
    ///
    /// `cursor` is the visible cursor position, or `None` for a hidden cursor. The returned
    /// bytes are valid until the next call.
    ///
    /// # Errors
    ///
    /// Returns an [`EncodeError`] when the frame cannot be carried. The caller then calls
    /// [`fail`](Self::fail) and repaints with ANSI.
    pub fn encode(
        &mut self,
        grid: &Grid,
        clusters: &ClusterTable,
        generation: u64,
        sequence: u64,
        cursor: Option<(u16, u16)>,
    ) -> Result<&[u8], EncodeError> {
        if self.detached {
            return Err(EncodeError::Detached);
        }
        let frame = self.build(grid, clusters, generation, sequence, cursor)?;
        self.bytes = frame;
        Ok(&self.bytes)
    }

    fn build(
        &self,
        grid: &Grid,
        clusters: &ClusterTable,
        generation: u64,
        sequence: u64,
        cursor: Option<(u16, u16)>,
    ) -> Result<Vec<u8>, EncodeError> {
        let (columns, rows) = (grid.width(), grid.height());
        let cells = usize::from(columns) * usize::from(rows);
        if columns == 0
            || rows == 0
            || columns > MAX_FRAME_AXIS
            || rows > MAX_FRAME_AXIS
            || cells > MAX_FRAME_CELLS
        {
            return Err(EncodeError::Geometry);
        }
        if generation == 0 {
            return Err(EncodeError::Generation);
        }
        if cursor.is_some_and(|(x, y)| x >= columns || y >= rows) {
            return Err(EncodeError::Cursor);
        }
        let mut cell_bytes = Vec::with_capacity(cells * CELL_BYTES);
        let mut blob = Vec::new();
        let mut records = Vec::new();
        let mut remap: BTreeMap<ClusterId, u32> = BTreeMap::new();
        let mut attributes = Attributes::default();
        for y in 0..rows {
            for x in 0..columns {
                let cell = grid.get(x, y).ok_or(EncodeError::Malformed)?;
                let glyph = Self::glyph_word(cell, clusters, &mut remap, &mut blob, &mut records)?;
                let index = match self.mode {
                    AttributeMode::Native => attributes.index_of(grid, cell),
                    AttributeMode::Stripped => 0,
                };
                let meta = Self::meta_word(cell)? | (u32::from(index) << 16);
                let style = cell.style();
                push_u32(&mut cell_bytes, glyph);
                push_u32(&mut cell_bytes, self.color(style.fg)?);
                push_u32(&mut cell_bytes, self.color(style.bg)?);
                push_u32(&mut cell_bytes, meta);
            }
        }
        if blob.len() > MAX_BLOB_BYTES {
            return Err(EncodeError::TooLarge);
        }
        let cells_end = FRAME_HEADER_BYTES + cell_bytes.len();
        let clusters_at = cells_end;
        let blob_at = align8(clusters_at + records.len());
        let damage_at = align8(blob_at + blob.len());
        let attributes_at = damage_at;
        let used = if attributes.records.is_empty() {
            damage_at
        } else {
            attributes_at + attributes.records.len()
        };
        if used > SLOT_BYTES {
            return Err(EncodeError::TooLarge);
        }
        let mut out = vec![0u8; used];
        put_u32(&mut out, 0, to_u32(used)?);
        put_u16(&mut out, 4, to_u16(FRAME_HEADER_BYTES)?);
        put_u16(&mut out, 6, FULL_SNAPSHOT);
        put_u64(&mut out, 8, sequence);
        put_u64(&mut out, 16, generation);
        put_u32(&mut out, 24, u32::from(columns));
        put_u32(&mut out, 28, u32::from(rows));
        put_u32(&mut out, 32, to_u32(cells)?);
        put_u32(&mut out, 36, to_u32(FRAME_HEADER_BYTES)?);
        put_u32(&mut out, 40, to_u32(records.len() / CLUSTER_RECORD_BYTES)?);
        put_u32(&mut out, 44, to_u32(clusters_at)?);
        put_u32(&mut out, 48, to_u32(blob.len())?);
        put_u32(&mut out, 52, to_u32(blob_at)?);
        put_u32(&mut out, 56, 0);
        put_u32(&mut out, 60, to_u32(damage_at)?);
        if let Some((x, y)) = cursor {
            put_u32(&mut out, 64, u32::from(x));
            put_u32(&mut out, 68, u32::from(y));
            put_u32(&mut out, 72, 1);
        }
        if !attributes.records.is_empty() {
            put_u32(
                &mut out,
                76,
                to_u32(attributes.records.len() / ATTRIBUTE_RECORD_BYTES)?,
            );
            put_u32(&mut out, 80, to_u32(attributes_at)?);
            out[attributes_at..used].copy_from_slice(&attributes.records);
        }
        out[FRAME_HEADER_BYTES..cells_end].copy_from_slice(&cell_bytes);
        out[clusters_at..clusters_at + records.len()].copy_from_slice(&records);
        out[blob_at..blob_at + blob.len()].copy_from_slice(&blob);
        Ok(out)
    }

    fn glyph_word(
        cell: Cell,
        clusters: &ClusterTable,
        remap: &mut BTreeMap<ClusterId, u32>,
        blob: &mut Vec<u8>,
        records: &mut Vec<u8>,
    ) -> Result<u32, EncodeError> {
        match cell.glyph().decode() {
            DecodedGlyph::Continuation => Ok(CONTINUATION),
            DecodedGlyph::Inline(scalar) => {
                if scalar.is_control() {
                    Err(EncodeError::Control)
                } else {
                    Ok(u32::from(scalar))
                }
            }
            DecodedGlyph::Cluster(id) => {
                if let Some(&index) = remap.get(&id) {
                    return Ok(CLUSTER_TAG | index);
                }
                let text = clusters.cluster_text(id).ok_or(EncodeError::Malformed)?;
                let width = clusters.cluster_width(id).ok_or(EncodeError::Malformed)?;
                if text.is_empty() || text.len() > MAX_CLUSTER_BYTES {
                    return Err(EncodeError::TooLarge);
                }
                if text.chars().any(char::is_control) {
                    return Err(EncodeError::Control);
                }
                if !(1..=2).contains(&width) {
                    return Err(EncodeError::Unsupported);
                }
                let index = to_u32(records.len() / CLUSTER_RECORD_BYTES)?;
                push_u32(records, to_u32(blob.len())?);
                push_u32(records, to_u32(text.len())?);
                records.push(u8::try_from(width).map_err(|_| EncodeError::Unsupported)?);
                records.extend_from_slice(&[0; 7]);
                blob.extend_from_slice(text.as_bytes());
                remap.insert(id, index);
                Ok(CLUSTER_TAG | index)
            }
        }
    }

    fn meta_word(cell: Cell) -> Result<u32, EncodeError> {
        let meta = cell.meta();
        if meta.flags.skip || meta.flags.protected {
            return Err(EncodeError::Unsupported);
        }
        let bits = lunatui_core::Meta { ext: 0, ..meta }.to_bits();
        Ok(bits)
    }

    fn color(&self, color: Color) -> Result<u32, EncodeError> {
        let resolved = match color {
            Color::Token(id) => *self
                .palette
                .get(usize::try_from(id).map_err(|_| EncodeError::UnresolvedToken)?)
                .ok_or(EncodeError::UnresolvedToken)?,
            other => other,
        };
        match resolved {
            Color::Default => Ok(0),
            Color::Indexed(index) => Ok(KIND_INDEXED | u32::from(index)),
            Color::Rgb(red, green, blue) => {
                Ok(KIND_RGB | u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue))
            }
            Color::Token(_) => Err(EncodeError::UnresolvedToken),
        }
    }
}

#[derive(Default)]
struct Attributes {
    remap: BTreeMap<u16, u16>,
    records: Vec<u8>,
}

impl Attributes {
    fn index_of(&mut self, grid: &Grid, cell: Cell) -> u16 {
        let source = cell.meta().ext;
        if source == 0 {
            return 0;
        }
        if let Some(&index) = self.remap.get(&source) {
            return index;
        }
        let Some(attrs) = grid.ext_table().get(source) else {
            return 0;
        };
        let Ok(index) = u16::try_from(self.records.len() / ATTRIBUTE_RECORD_BYTES + 1) else {
            return 0;
        };
        self.records.push(attrs.fg_alpha());
        self.records.push(attrs.bg_alpha());
        self.records.push(attrs.dx().to_le_bytes()[0]);
        self.records.push(attrs.dy().to_le_bytes()[0]);
        self.records.extend_from_slice(&[0; 4]);
        self.remap.insert(source, index);
        index
    }
}

fn align8(value: usize) -> usize {
    value.div_ceil(8) * 8
}

fn to_u32(value: usize) -> Result<u32, EncodeError> {
    u32::try_from(value).map_err(|_| EncodeError::TooLarge)
}

fn to_u16(value: usize) -> Result<u16, EncodeError> {
    u16::try_from(value).map_err(|_| EncodeError::TooLarge)
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u16(out: &mut [u8], at: usize, value: u16) {
    out[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], at: usize, value: u64) {
    out[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use lunatui_core::{Attrs, Cell, CellFlags, GlyphSlot, Meta, Style, UnderlineStyle};

    use super::{EncodeError, FrameEncoder};

    fn cell(meta: Meta) -> Cell {
        Cell::from_parts(
            GlyphSlot::inline_scalar('a').unwrap_or_else(GlyphSlot::continuation),
            Style::new(),
            meta,
        )
    }

    #[test]
    fn skip_and_protected_cells_are_refused() {
        for flags in [
            CellFlags {
                skip: true,
                protected: false,
            },
            CellFlags {
                skip: false,
                protected: true,
            },
        ] {
            let meta = Meta {
                flags,
                ..Meta::default()
            };
            assert_eq!(
                FrameEncoder::meta_word(cell(meta)),
                Err(EncodeError::Unsupported)
            );
        }
    }

    #[test]
    fn plain_and_styled_cells_keep_their_meta_bits() {
        let attrs = Attrs::new()
            .set_bold(true)
            .set_underline(UnderlineStyle::Dashed);
        let meta = Meta {
            attrs,
            ext: 9,
            ..Meta::default()
        };
        let expected = Meta { ext: 0, ..meta }.to_bits();
        assert_eq!(FrameEncoder::meta_word(cell(meta)), Ok(expected));
        assert_eq!(expected & 1, 1);
    }
}
