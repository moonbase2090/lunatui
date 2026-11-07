// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

/// Largest sub-cell offset magnitude, in eighths of a cell.
pub const MAX_SUBCELL_OFFSET: i8 = 7;

/// Most entries an [`ExtTable`] holds. Index 0 means "no entry".
pub const MAX_EXT_ENTRIES: usize = 65_535;

/// Extra per-cell data that only a native host draws: alpha and a sub-cell glyph offset.
///
/// Alpha is 255 for opaque. Offsets are in eighths of a cell, clamped to
/// `-MAX_SUBCELL_OFFSET..=MAX_SUBCELL_OFFSET`; the fields are private so every value goes
/// through that clamp. The all-default value is [`NativeAttrs::OPAQUE`] and is never stored
/// in an [`ExtTable`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NativeAttrs {
    fg_alpha: u8,
    bg_alpha: u8,
    dx: i8,
    dy: i8,
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

    /// Foreground opacity, 255 = opaque.
    #[must_use]
    pub fn fg_alpha(self) -> u8 {
        self.fg_alpha
    }

    /// Background opacity, 255 = opaque.
    #[must_use]
    pub fn bg_alpha(self) -> u8 {
        self.bg_alpha
    }

    /// Horizontal glyph offset in eighths of a cell, within `-7..=7`.
    #[must_use]
    pub fn dx(self) -> i8 {
        self.dx
    }

    /// Vertical glyph offset in eighths of a cell, within `-7..=7`.
    #[must_use]
    pub fn dy(self) -> i8 {
        self.dy
    }

    /// These attributes with a new foreground opacity.
    #[must_use]
    pub fn with_fg_alpha(self, fg_alpha: u8) -> Self {
        Self { fg_alpha, ..self }
    }

    /// These attributes with a new background opacity.
    #[must_use]
    pub fn with_bg_alpha(self, bg_alpha: u8) -> Self {
        Self { bg_alpha, ..self }
    }

    /// These attributes with a new offset, clamped into range.
    #[must_use]
    pub fn with_offset(self, dx: i8, dy: i8) -> Self {
        Self::new(self.fg_alpha, self.bg_alpha, dx, dy)
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
/// Index 0 means no entry. Equal attributes share one index within a frame. Slots freed when a
/// frame is seeded are reused, so a long-running app never fills the table with stale entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtTable {
    slots: Vec<Option<NativeAttrs>>,
    lookup: BTreeMap<NativeAttrs, u16>,
    free: BTreeSet<u16>,
}

impl ExtTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of live entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lookup.len()
    }

    /// Whether the table has no live entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lookup.is_empty()
    }

    /// One past the highest index in use, live or free.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// The entry at `index`, or `None` for index 0, free and unknown indices.
    #[must_use]
    pub fn get(&self, index: u16) -> Option<NativeAttrs> {
        let slot = usize::from(index).checked_sub(1)?;
        self.slots.get(slot).copied().flatten()
    }

    /// Index for `attrs`, adding an entry when needed.
    ///
    /// Freed slots are reused lowest first. Returns 0 for the defaults and when every slot is
    /// taken; the frame then stays correct without the attributes.
    pub fn intern(&mut self, attrs: NativeAttrs) -> u16 {
        if attrs.is_default() {
            return 0;
        }
        if let Some(&index) = self.lookup.get(&attrs) {
            return index;
        }
        let index = if let Some(index) = self.free.pop_first() {
            self.slots[usize::from(index) - 1] = Some(attrs);
            index
        } else if self.slots.len() < MAX_EXT_ENTRIES {
            self.slots.push(Some(attrs));
            u16::try_from(self.slots.len()).unwrap_or(0)
        } else {
            return 0;
        };
        self.lookup.insert(attrs, index);
        index
    }

    /// Removes every entry.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.lookup.clear();
        self.free.clear();
    }

    /// Replaces the entries with those of `previous` for which `live(index)` is true, keeping
    /// each at its index so equal attributes keep equal indices from one frame to the next.
    /// Entries nothing refers to any more are dropped and their slots become reusable. Call it
    /// on a table no cell refers to yet.
    pub fn seed_from(&mut self, previous: &Self, live: impl Fn(u16) -> bool) {
        self.clear();
        for (slot, entry) in previous.slots.iter().enumerate() {
            let index = u16::try_from(slot + 1).unwrap_or(0);
            let kept = entry.filter(|_| live(index));
            self.slots.push(kept);
            match kept {
                Some(attrs) => {
                    self.lookup.insert(attrs, index);
                }
                None => {
                    self.free.insert(index);
                }
            }
        }
        while matches!(self.slots.last(), Some(None)) {
            self.slots.pop();
            if let Ok(index) = u16::try_from(self.slots.len() + 1) {
                self.free.remove(&index);
            }
        }
    }
}
