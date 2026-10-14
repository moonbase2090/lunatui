// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Style};
use lunatui_theme::{Theme, Token};

/// Grid, theme, and cluster state for [`Widget::render`](crate::Widget::render) (PRD §3.2 / DD-3).
pub struct RenderCx<'a> {
    grid: &'a mut Grid,
    clusters: &'a mut ClusterTable,
    theme: &'a Theme,
}

impl<'a> RenderCx<'a> {
    /// Binds the active frame buffers and theme for one widget draw.
    pub fn new(grid: &'a mut Grid, clusters: &'a mut ClusterTable, theme: &'a Theme) -> Self {
        Self {
            grid,
            clusters,
            theme,
        }
    }

    /// Active theme (token resolution).
    #[must_use]
    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// Resolves a semantic token name from the active theme.
    #[must_use]
    pub fn token_color(&self, name: &str) -> Color {
        Token::from_name(name).map_or(Color::Default, |token| self.theme.resolve(token))
    }

    /// Writes `text` at `(x, y)` with `style`.
    pub fn put_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        self.grid.put_str(x, y, style, text, self.clusters);
    }
}
