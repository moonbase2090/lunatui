// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[cfg(feature = "image")]
use core::fmt;
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};

use lunatui_core::UnderlineStyle;

use crate::overlay::blend_rgb;
use lunatui_theme::{Theme, Token};

#[cfg(feature = "image")]
use lunatui_image::{ImageRequest, ImageSink};

#[cfg(feature = "color-profiles")]
use lunatui_theme::{PreparedPalette, Profile};

/// Grid, theme, and cluster state for [`Widget::render`](crate::Widget::render) (PRD §3.2 / DD-3).
#[cfg_attr(not(feature = "image"), derive(Debug))]
pub struct RenderCx<'a> {
    grid: &'a mut Grid,
    clusters: &'a mut ClusterTable,
    theme: &'a Theme,
    #[cfg(feature = "color-profiles")]
    palette: Option<&'a PreparedPalette>,
    #[cfg(feature = "image")]
    image_sink: Option<&'a mut dyn ImageSink>,
}

#[cfg(feature = "image")]
impl fmt::Debug for RenderCx<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("RenderCx");
        debug
            .field("grid", &self.grid)
            .field("clusters", &self.clusters)
            .field("theme", &self.theme);
        #[cfg(feature = "color-profiles")]
        debug.field("palette", &self.palette);
        debug.finish_non_exhaustive()
    }
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
            palette: None,
            #[cfg(feature = "image")]
            image_sink: None,
        }
    }

    /// Attaches the active frame's image sink. Unsupported protocols leave
    /// image widgets on their cell fallback.
    #[cfg(feature = "image")]
    #[must_use]
    pub fn image_sink(mut self, sink: &'a mut dyn ImageSink) -> Self {
        self.image_sink = Some(sink);
        self
    }

    #[cfg(feature = "image")]
    pub(crate) fn submit_image(&mut self, request: ImageRequest<'_>) -> bool {
        self.image_sink
            .as_deref_mut()
            .is_some_and(|sink| sink.supports_pixels() && sink.submit(request))
    }

    /// Borrow token colors prepared at startup or when the theme changes.
    #[cfg(feature = "color-profiles")]
    #[must_use]
    pub fn palette(mut self, palette: &'a PreparedPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// The prepared terminal color profile, if one was supplied.
    #[cfg(feature = "color-profiles")]
    #[must_use]
    pub fn color_profile(&self) -> Option<Profile> {
        self.palette.map(PreparedPalette::profile)
    }

    /// Active theme for the frame.
    ///
    /// Prefer [`token`](Self::token) for colors so profile downsampling stays canonical.
    #[must_use]
    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    /// Resolves a semantic token through the prepared palette or active theme.
    #[must_use]
    pub fn token(&self, token: Token) -> Color {
        #[cfg(feature = "color-profiles")]
        if let Some(palette) = self.palette {
            return palette.resolve(token);
        }
        self.theme.resolve(token)
    }

    /// Resolves a theme-file token name, if recognized.
    #[must_use]
    pub fn token_named(&self, name: &str) -> Option<Color> {
        Token::from_name(name).map(|token| self.token(token))
    }

    /// Writes `text` at `(x, y)` with `style` (PRD argument order: text before style).
    ///
    /// Does not clip to a widget area; callers must stay inside their `Rect` (see ADR 0004).
    /// Text attributes in `style`, such as bold, are kept on every written cell.
    pub fn put_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        self.grid.put_str(x, y, style, text, self.clusters);
        if style.attrs != Attrs::new() {
            let written = u16::try_from(lunatui_core::width(text)).unwrap_or(u16::MAX);
            self.grid.set_style(Rect::new(x, y, written, 1), style);
        }
    }

    /// Fills `area` with `ch` and `style` without building an intermediate string.
    pub fn fill(&mut self, area: Rect, style: Style, ch: char) {
        self.grid.fill(area, style, ch, self.clusters);
        if style.attrs != Attrs::new() {
            self.grid.set_style(area, style);
        }
    }

    #[cfg(any(
        feature = "barchart",
        feature = "bigtext",
        feature = "calendar",
        feature = "chart",
        feature = "image",
        feature = "sparkline"
    ))]
    pub(crate) fn put_char_patched(&mut self, x: u16, y: u16, glyph: char, style: Style) {
        let existing = self
            .grid
            .get(x, y)
            .map(lunatui_core::Cell::style)
            .unwrap_or_default();
        let patched = patch_style(existing, style);
        self.grid.put_char(x, y, patched, glyph, self.clusters);
    }

    #[cfg(any(feature = "barchart", feature = "canvas", feature = "chart"))]
    pub(crate) fn clear_area(&mut self, area: Rect, style: Style) {
        let blank = lunatui_core::GlyphSlot::inline_scalar(' ');
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                if let Some(cell) = self.grid.get(x, y)
                    && (Some(cell.glyph()) != blank || cell.style() != style)
                {
                    self.grid.put_char(x, y, style, ' ', self.clusters);
                }
            }
        }
    }

    #[cfg(feature = "calendar")]
    pub(crate) fn patch_area(&mut self, area: Rect, style: Style) {
        let end_x = area.x.saturating_add(area.width);
        let end_y = area.y.saturating_add(area.height);
        for y in area.y..end_y {
            for x in area.x..end_x {
                let existing = self
                    .grid
                    .get(x, y)
                    .map(lunatui_core::Cell::style)
                    .unwrap_or_default();
                self.set_style(Rect::new(x, y, 1, 1), patch_style(existing, style));
            }
        }
    }

    #[cfg(feature = "image")]
    pub(crate) fn keep_backgrounds(&mut self, area: Rect, draw: impl FnOnce(&mut Self)) {
        let end_x = area.x.saturating_add(area.width);
        let end_y = area.y.saturating_add(area.height);
        let cells: Vec<(u16, u16)> = (area.y..end_y)
            .flat_map(|y| (area.x..end_x).map(move |x| (x, y)))
            .collect();
        let before: Vec<Color> = cells
            .iter()
            .map(|&(x, y)| {
                self.grid
                    .get(x, y)
                    .map_or(Color::Default, |cell| cell.style().bg)
            })
            .collect();
        draw(self);
        for (&(x, y), bg) in cells.iter().zip(before) {
            let Some(cell) = self.grid.get(x, y) else {
                continue;
            };
            let style = cell.style();
            if style.bg == Color::Default && bg != Color::Default {
                self.grid.set_style(Rect::new(x, y, 1, 1), style.bg(bg));
            }
        }
    }

    /// Applies `style` to every cell in `area`, keeping glyphs unchanged.
    pub fn set_style(&mut self, area: Rect, style: Style) {
        self.grid.set_style(area, style);
    }

    /// Alpha-composites `tint` over each cell background in `area` (glyphs unchanged).
    pub fn blend_backdrop(&mut self, area: Rect, alpha: f32, tint: Color) {
        let tint_rgb = resolve_rgb(tint, self.theme);
        for row in area.y..area.y.saturating_add(area.height) {
            for col in area.x..area.x.saturating_add(area.width) {
                let Some(cell) = self.grid.get(col, row) else {
                    continue;
                };
                let style = cell.style();
                let base_rgb = resolve_rgb(style.bg, self.theme);
                let blended = blend_rgb(base_rgb, tint_rgb, alpha);
                let patch = Rect::new(col, row, 1, 1);
                self.grid
                    .set_style(patch, style.bg(Color::Rgb(blended.0, blended.1, blended.2)));
            }
        }
    }
}

pub(crate) fn patch_style(base: Style, over: Style) -> Style {
    Style {
        fg: if over.fg == Color::Default {
            base.fg
        } else {
            over.fg
        },
        bg: if over.bg == Color::Default {
            base.bg
        } else {
            over.bg
        },
        attrs: merge_attrs(base.attrs, over.attrs),
    }
}

fn merge_attrs(base: Attrs, over: Attrs) -> Attrs {
    if over.to_bits() == 0 {
        return base;
    }
    let mut merged = base
        .set_bold(base.bold() || over.bold())
        .set_dim(base.dim() || over.dim())
        .set_italic(base.italic() || over.italic())
        .set_blink(base.blink() || over.blink())
        .set_reverse(base.reverse() || over.reverse())
        .set_hidden(base.hidden() || over.hidden())
        .set_strikethrough(base.strikethrough() || over.strikethrough())
        .set_overline(base.overline() || over.overline());
    if over.underline() != UnderlineStyle::None {
        merged = merged.set_underline(over.underline());
    }
    merged
}

fn resolve_rgb(color: Color, theme: &Theme) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(index) => ansi_rgb(index),
        Color::Token(id) => resolve_rgb(theme.resolve_id(id), theme),
        Color::Default => resolve_rgb(theme.resolve(Token::Surface0), theme),
    }
}

fn ansi_rgb(index: u8) -> (u8, u8, u8) {
    const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        0..=15 => {
            let base = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];
            base[usize::from(index)]
        }
        16..=231 => {
            let index = index - 16;
            let r = CUBE[usize::from(index / 36 % 6)];
            let g = CUBE[usize::from(index / 6 % 6)];
            let b = CUBE[usize::from(index % 6)];
            (r, g, b)
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}
