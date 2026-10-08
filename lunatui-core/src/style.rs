// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::attr::Attrs;
use crate::color::Color;

/// Foreground, background and attributes applied to a [`crate::Cell`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Text attributes.
    pub attrs: Attrs,
}

impl Style {
    /// A style with default colors and no attributes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a copy with `fg` set.
    #[must_use]
    pub fn fg(self, fg: Color) -> Self {
        Self { fg, ..self }
    }

    /// Returns a copy with `bg` set.
    #[must_use]
    pub fn bg(self, bg: Color) -> Self {
        Self { bg, ..self }
    }

    /// Returns a copy with `attrs` set.
    #[must_use]
    pub fn with_attrs(self, attrs: Attrs) -> Self {
        Self { attrs, ..self }
    }
}
