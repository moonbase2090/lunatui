// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Style};
use lunatui_theme::{Theme, Token};

#[cfg(feature = "color-profiles")]
use lunatui_theme::Profile;

/// Grid, theme, and cluster state for [`Widget::render`](crate::Widget::render) (PRD §3.2 / DD-3).
#[derive(Debug)]
pub struct RenderCx<'a> {
    grid: &'a mut Grid,
    clusters: &'a mut ClusterTable,
    theme: &'a Theme,
    #[cfg(feature = "color-profiles")]
    profile: Option<Profile>,
}

impl<'a> RenderCx<'a> {
    /// Binds the active frame buffers and theme for one widget draw.
    ///
    /// # Example
    ///
    /// ```
    /// use lunatui_core::{ClusterTable, Grid};
    /// use lunatui_theme::{Theme, Variant};
    /// use lunatui_widgets::RenderCx;
    ///
    /// let theme = Theme::builtin(Variant::Dark);
    /// let mut grid = Grid::new(1, 1);
    /// let mut clusters = ClusterTable::new();
    /// let _cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    /// ```
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
    pub fn profile(mut self, profile: Profile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Active theme for the frame.
    ///
    /// Prefer [`token`](Self::token) for colors so profile downsampling stays canonical.
    #[must_use]
    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    /// Resolves a semantic token to a color (profile downsampling when enabled).
    #[must_use]
    pub fn token(&self, token: Token) -> Color {
        let base = self.theme.resolve(token);
        #[cfg(feature = "color-profiles")]
        if let Some(profile) = self.profile {
            return profile.downsample(base);
        }
        base
    }

    /// Resolves a theme-file token name, if recognized.
    #[must_use]
    pub fn token_named(&self, name: &str) -> Option<Color> {
        Token::from_name(name).map(|token| self.token(token))
    }

    /// Writes `text` at `(x, y)` with `style` (PRD argument order: text before style).
    ///
    /// Does not clip to a widget area; callers must stay inside their `Rect` (see ADR 0004).
    pub fn put_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        self.grid.put_str(x, y, style, text, self.clusters);
    }
}
