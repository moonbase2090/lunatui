// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Largest sub-cell offset magnitude, in eighths of a cell.
pub const MAX_SUBCELL_OFFSET: i8 = 7;

/// Most entries an [`ExtTable`] holds. Index 0 means "no entry".
pub const MAX_EXT_ENTRIES: usize = 65_535;

/// Extra per-cell data that only a native host draws: alpha and a sub-cell glyph offset.
///
/// Alpha is 255 for opaque. Offsets are in eighths of a cell, clamped to
/// `-MAX_SUBCELL_OFFSET..=MAX_SUBCELL_OFFSET`. The all-default value is
/// [`NativeAttrs::OPAQUE`] and is never stored in an [`ExtTable`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NativeAttrs {
    /// Foreground opacity, 255 = opaque.
    pub fg_alpha: u8,
    /// Background opacity, 255 = opaque.
    pub bg_alpha: u8,
    /// Horizontal glyph offset in eighths of a cell.
    pub dx: i8,
    /// Vertical glyph offset in eighths of a cell.
    pub dy: i8,
}

impl NativeAttrs {
    /// Opaque colors and no offset.
    pub const OPAQUE: Self = Self {
        fg_alpha: 255,
        bg_alpha: 255,
        dx: 0,
        dy: 0,
    };

    /// Attributes with the offsets clamped into range.
    #[must_use]
    pub fn new(fg_alpha: u8, bg_alpha: u8, dx: i8, dy: i8) -> Self {
        Self {
            fg_alpha,
            bg_alpha,
            dx: dx.clamp(-MAX_SUBCELL_OFFSET, MAX_SUBCELL_OFFSET),
            dy: dy.clamp(-MAX_SUBCELL_OFFSET, MAX_SUBCELL_OFFSET),
        }
    }

    /// Whether these are the opaque, unshifted defaults.
    #[must_use]
    pub fn is_default(self) -> bool {
        self == Self::OPAQUE
    }
}

impl Default for NativeAttrs {
    fn default() -> Self {
        Self::OPAQUE
    }
}

/// Per-frame table of [`NativeAttrs`] that cells refer to by a 16-bit index (ADR 0001, ADR 0009).
///
/// Index 0 means no entry. Equal attributes share one index within a frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtTable {
    entries: Vec<NativeAttrs>,
    lookup: BTreeMap<NativeAttrs, u16>,
}

impl ExtTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry at `index`, or `None` for index 0 and unknown indices.
    #[must_use]
    pub fn get(&self, index: u16) -> Option<NativeAttrs> {
        let slot = usize::from(index).checked_sub(1)?;
        self.entries.get(slot).copied()
    }

    /// Index for `attrs`, adding an entry when needed.
    ///
    /// Returns 0 for the defaults and when the table is full; the frame then stays correct
    /// without the attributes.
    pub fn intern(&mut self, attrs: NativeAttrs) -> u16 {
        if attrs.is_default() {
            return 0;
        }
        if let Some(&index) = self.lookup.get(&attrs) {
            return index;
        }
        if self.entries.len() >= MAX_EXT_ENTRIES {
            return 0;
        }
        self.entries.push(attrs);
        let index = u16::try_from(self.entries.len()).unwrap_or(0);
        self.lookup.insert(attrs, index);
        index
    }

    /// Removes every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lookup.clear();
    }

    /// Replaces the entries with a copy of `previous`, so equal attributes keep equal indices
    /// from one frame to the next. Call it on a table no cell refers to yet.
    pub fn seed_from(&mut self, previous: &Self) {
        self.entries.clone_from(&previous.entries);
        self.lookup.clone_from(&previous.lookup);
    }
}
