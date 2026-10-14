// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Style};
use lunatui_theme::{Theme, Token};

#[cfg(feature = "color-profiles")]
use lunatui_theme::Profile;

/// Grid, theme, and cluster state for [`Widget::render`](crate::Widget::render) (PRD §3.2 / DD-3).
pub struct RenderCx<'a> {
    grid: &'a mut Grid,
    clusters: &'a mut ClusterTable,
    theme: &'a Theme,
    #[cfg(feature = "color-profiles")]
    profile: Option<Profile>,
}

impl<'a> RenderCx<'a> {
    /// Binds the active frame buffers and theme for one widget draw.
    #[must_use]
    pub fn new(grid: &'a mut Grid, clusters: &'a mut ClusterTable, theme: &'a Theme) -> Self {
        Self {
            grid,
            clusters,
            theme,
            #[cfg(feature = "color-profiles")]
            profile: None,
        }
    }

    /// Downsamples resolved token colors through `profile` when the `color-profiles` feature is on.
    #[cfg(feature = "color-profiles")]
    #[must_use]
    pub fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Active theme (token resolution).
    #[must_use]
    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// Resolves a semantic token name from the active theme.
    #[must_use]
    pub fn token_color(&self, name: &str) -> Color {
        let base = Token::from_name(name).map_or(Color::Default, |token| self.theme.resolve(token));
        #[cfg(feature = "color-profiles")]
        if let Some(profile) = self.profile {
            return profile.downsample(base);
        }
        base
    }

    /// Writes `text` at `(x, y)` with `style`.
    pub fn put_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        self.grid.put_str(x, y, style, text, self.clusters);
    }
}
