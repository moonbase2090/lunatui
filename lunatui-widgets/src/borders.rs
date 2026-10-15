// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::symbols::border;

/// Which edges of a [`Block`](crate::Block) draw a border.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Borders(u8);

impl Borders {
    /// Top edge.
    pub const TOP: Self = Self(0b0001);
    /// Right edge.
    pub const RIGHT: Self = Self(0b0010);
    /// Bottom edge.
    pub const BOTTOM: Self = Self(0b0100);
    /// Left edge.
    pub const LEFT: Self = Self(0b1000);
    /// All four edges.
    pub const ALL: Self = Self(0b1111);
    /// No edges.
    pub const NONE: Self = Self(0);

    /// Whether `self` includes `edge`.
    #[must_use]
    pub const fn contains(self, edge: Self) -> bool {
        self.0 & edge.0 != 0
    }

    /// Whether `self` includes every flag in `mask`.
    #[must_use]
    pub const fn contains_all(self, mask: Self) -> bool {
        self.0 & mask.0 == mask.0
    }

    #[must_use]
    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for Borders {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

/// Border glyph set (ratatui `BorderType` parity).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderType {
    /// `symbols::border::PLAIN`.
    #[default]
    Plain,
    /// `symbols::border::ROUNDED`.
    Rounded,
    /// `symbols::border::DOUBLE`.
    Double,
    /// `symbols::border::THICK`.
    Thick,
}

impl BorderType {
    /// Resolves this type to a symbol set.
    #[must_use]
    pub const fn border_set(self) -> border::Set<'static> {
        match self {
            Self::Plain => border::PLAIN,
            Self::Rounded => border::ROUNDED,
            Self::Double => border::DOUBLE,
            Self::Thick => border::THICK,
        }
    }
}
