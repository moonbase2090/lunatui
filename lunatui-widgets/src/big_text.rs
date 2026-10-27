// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::subcell::GlyphSet;
use lunatui_core::{Attrs, Rect, Size, Style};
use lunatui_theme::Token;

use crate::state::WidgetState;
use crate::{Alignment, RenderCx, Widget};

#[path = "font.rs"]
mod font;

const GLYPH_PX: u16 = 8;
const INK_COLUMNS: u8 = 8;
const GLYPH_COLUMNS: usize = 16;

/// How an 8×8 character is packed into terminal cells.
///
/// The high bit of each stored row (bit 7) is the left pixel. Lowercase letters use the uppercase
/// bitmap. Printable ASCII without its own drawing is a hollow box, and anything else is blank.
///
/// A terminal cell is about twice as tall as it is wide, so [`PixelSize::Full`] draws each pixel
/// as two columns and [`PixelSize::HalfHeight`] stacks two pixels in one cell. Those two keep the
/// pixels square. [`PixelSize::HalfWidth`] and [`PixelSize::Quadrant`] do not.
/// [`PixelSize::Sextant`] pads the bitmap with one blank pixel row. Sextant and octant sizes pack
/// as quadrants until [`BigText::subcell_glyphs`] reports that the font has those blocks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PixelSize {
    /// Two cells per pixel, full block or space. Sixteen by eight cells.
    #[default]
    Full,
    /// Two pixels stacked in each cell. Eight by four cells.
    HalfHeight,
    /// Two pixels side by side in each cell. Four by eight cells. The pixels are not square.
    HalfWidth,
    /// A two-by-two quadrant in each cell. Four by four cells. The pixels are taller than wide.
    Quadrant,
    /// A two-by-three sextant in each cell. Four by three cells.
    Sextant,
    /// A two-by-four octant in each cell. Four by two cells.
    Octant,
    /// A two-by-four braille pattern in each cell. Four by two cells.
    Braille,
}

impl PixelSize {
    /// Cell columns and rows occupied by one character, before any glyph fallback.
    #[must_use]
    pub const fn cells(self) -> (u16, u16) {
        let (px_w, px_h, repeat) = pack(self);
        let span_h = match self {
            Self::Sextant => 9,
            _ => GLYPH_PX,
        };
        (
            (GLYPH_PX / px_w as u16) * repeat as u16,
            span_h / px_h as u16,
        )
    }
}

/// Large text drawn from an 8×8 bitmap.
///
/// Glyph bitmaps for digits, capitals, and `:.-%` are derived from Press Start 2P by `CodeMan38`,
/// SIL Open Font License 1.1 (`assets/fonts/PressStart2P-OFL.txt`). Other printable ASCII is a
/// hollow box.
///
/// With [`style`](Self::style) set, that style is used as given. Otherwise the text follows theme
/// tokens: primary text at rest, accent and bold when focused, and dimmed primary text when
/// disabled. The block is top-aligned. Horizontal placement follows [`Alignment`].
///
/// # Example
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{BigText, PixelSize, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(4, 1);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// BigText::new("#")
///     .pixel_size(PixelSize::Quadrant)
///     .render(Rect::new(0, 0, 4, 1), &mut cx);
/// assert_eq!(grid.row_display(0, &clusters), "▛▀▀▜");
/// ```
#[derive(Clone, Debug)]
pub struct BigText<'a> {
    text: &'a str,
    pixel_size: PixelSize,
    subcell_glyphs: bool,
    style: Style,
    styled: bool,
    alignment: Alignment,
    state: WidgetState,
}

impl<'a> BigText<'a> {
    /// Text drawn at [`PixelSize::Full`], left aligned, in the resting theme style.
    ///
    /// Sextant and octant sizes pack as quadrants until [`subcell_glyphs`](Self::subcell_glyphs).
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            pixel_size: PixelSize::Full,
            subcell_glyphs: false,
            style: Style::new(),
            styled: false,
            alignment: Alignment::Left,
            state: WidgetState::Normal,
        }
    }

    /// Sets how many cells each font pixel occupies.
    #[must_use]
    pub fn pixel_size(mut self, pixel_size: PixelSize) -> Self {
        self.pixel_size = pixel_size;
        self
    }

    /// Reports whether sextant and octant glyphs can be used.
    ///
    /// Lunatui does not probe the terminal font. When `available` is false, [`PixelSize::Sextant`]
    /// and [`PixelSize::Octant`] use the quadrant cell size and quadrant glyphs.
    #[must_use]
    pub fn subcell_glyphs(mut self, available: bool) -> Self {
        self.subcell_glyphs = available;
        self
    }

    /// Paints with `style` and turns theme coloring off.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.styled = true;
        self
    }

    /// Horizontal placement inside the area. Overflow clips on the right.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Theme appearance used when [`style`](Self::style) was not set.
    #[must_use]
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Writes cell row `cell_row` into `out`.
    ///
    /// Returns the full row width in columns. Columns past `out.len()` are not stored.
    #[must_use]
    pub fn row_glyphs(&self, cell_row: u16, out: &mut [char]) -> usize {
        let size = self.resolved_size();
        let (_width, height) = size.cells();
        if cell_row >= height {
            return 0;
        }
        let mut written = 0_usize;
        for ch in self.glyphs() {
            let mut piece = [' '; GLYPH_COLUMNS];
            let cols = glyph_cols(ch, size, cell_row, &mut piece);
            for glyph in piece.iter().take(usize::from(cols)) {
                if let Some(slot) = out.get_mut(written) {
                    *slot = *glyph;
                }
                written = written.saturating_add(1);
            }
        }
        written
    }

    fn resolved_size(&self) -> PixelSize {
        match self.pixel_size {
            PixelSize::Sextant | PixelSize::Octant if !self.subcell_glyphs => PixelSize::Quadrant,
            other => other,
        }
    }

    fn glyphs(&self) -> impl Iterator<Item = char> + '_ {
        self.text.chars().filter(|ch| !ch.is_control())
    }

    fn columns(&self) -> u16 {
        let (cell_w, _) = self.resolved_size().cells();
        let wide = self.glyphs().count().saturating_mul(usize::from(cell_w));
        u16::try_from(wide).unwrap_or(u16::MAX)
    }

    fn paint_style(&self, cx: &RenderCx<'_>) -> Style {
        if self.styled {
            return self.style;
        }
        match self.state {
            WidgetState::Normal => Style::new().fg(cx.token(Token::TextPrimary)),
            WidgetState::Focused => Style::new()
                .fg(cx.token(Token::Accent))
                .with_attrs(Attrs::default().set_bold(true)),
            WidgetState::Disabled => Style::new()
                .fg(cx.token(Token::TextPrimary))
                .with_attrs(Attrs::default().set_dim(true)),
        }
    }

    fn paint_row(
        &self,
        area: Rect,
        cx: &mut RenderCx<'_>,
        style: Style,
        origin_x: u16,
        cell_row: u16,
    ) {
        let y = area.y.saturating_add(cell_row);
        let right = area.x.saturating_add(area.width);
        let mut x = origin_x;
        let size = self.resolved_size();
        for ch in self.glyphs() {
            if x >= right {
                break;
            }
            let mut piece = [' '; GLYPH_COLUMNS];
            let cols = glyph_cols(ch, size, cell_row, &mut piece);
            for glyph in piece.iter().take(usize::from(cols)) {
                if x >= right {
                    break;
                }
                cx.put_char_patched(x, y, *glyph, style);
                x = x.saturating_add(1);
            }
        }
    }
}

impl Widget for BigText<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let style = self.paint_style(cx);
        let (_cell_w, cell_h) = self.resolved_size().cells();
        let origin_x =
            area.x
                .saturating_add(aligned_offset(self.alignment, area.width, self.columns()));
        let rows = cell_h.min(area.height);
        for cell_row in 0..rows {
            self.paint_row(area, cx, style, origin_x, cell_row);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let (_cell_w, cell_h) = self.resolved_size().cells();
        Size::new(self.columns().min(avail.width), cell_h.min(avail.height))
    }
}

const fn aligned_offset(alignment: Alignment, width: u16, content: u16) -> u16 {
    let spare = width.saturating_sub(content);
    match alignment {
        Alignment::Left => 0,
        Alignment::Center => spare / 2,
        Alignment::Right => spare,
    }
}

const fn pack(size: PixelSize) -> (u8, u8, u8) {
    match size {
        PixelSize::Full => (1, 1, 2),
        PixelSize::HalfHeight => (1, 2, 1),
        PixelSize::HalfWidth => (2, 1, 1),
        PixelSize::Quadrant => (2, 2, 1),
        PixelSize::Sextant => (2, 3, 1),
        PixelSize::Octant | PixelSize::Braille => (2, 4, 1),
    }
}

const QUADRANT: [char; 16] = [
    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
];

const SEXTANT: [char; 64] = [
    '\u{2588}',
    '\u{1fb3b}',
    '\u{1fb3a}',
    '\u{1fb39}',
    '\u{1fb38}',
    '\u{1fb37}',
    '\u{1fb36}',
    '\u{1fb35}',
    '\u{1fb34}',
    '\u{1fb33}',
    '\u{1fb32}',
    '\u{1fb31}',
    '\u{1fb30}',
    '\u{1fb2f}',
    '\u{1fb2e}',
    '\u{1fb2d}',
    '\u{1fb2c}',
    '\u{1fb2b}',
    '\u{1fb2a}',
    '\u{1fb29}',
    '\u{1fb28}',
    '\u{2590}',
    '\u{1fb27}',
    '\u{1fb26}',
    '\u{1fb25}',
    '\u{1fb24}',
    '\u{1fb23}',
    '\u{1fb22}',
    '\u{1fb21}',
    '\u{1fb20}',
    '\u{1fb1f}',
    '\u{1fb1e}',
    '\u{1fb1d}',
    '\u{1fb1c}',
    '\u{1fb1b}',
    '\u{1fb1a}',
    '\u{1fb19}',
    '\u{1fb18}',
    '\u{1fb17}',
    '\u{1fb16}',
    '\u{1fb15}',
    '\u{1fb14}',
    '\u{258c}',
    '\u{1fb13}',
    '\u{1fb12}',
    '\u{1fb11}',
    '\u{1fb10}',
    '\u{1fb0f}',
    '\u{1fb0e}',
    '\u{1fb0d}',
    '\u{1fb0c}',
    '\u{1fb0b}',
    '\u{1fb0a}',
    '\u{1fb09}',
    '\u{1fb08}',
    '\u{1fb07}',
    '\u{1fb06}',
    '\u{1fb05}',
    '\u{1fb04}',
    '\u{1fb03}',
    '\u{1fb02}',
    '\u{1fb01}',
    '\u{1fb00}',
    ' ',
];

fn glyph_cols(ch: char, size: PixelSize, cell_row: u16, out: &mut [char; GLYPH_COLUMNS]) -> u16 {
    let bitmap = font::rows(ch);
    let (cols, rows) = size.cells();
    if cell_row >= rows {
        return 0;
    }
    let (px_w, px_h, repeat) = pack(size);
    let y0 = u8::try_from(cell_row).unwrap_or(0).saturating_mul(px_h);
    let mut index = 0_usize;
    for cell in 0..(GLYPH_PX / u16::from(px_w)) {
        let x0 = u8::try_from(cell).unwrap_or(0).saturating_mul(px_w);
        let glyph = cell_glyph(*bitmap, size, x0, y0);
        for _ in 0..repeat {
            if let Some(slot) = out.get_mut(index) {
                *slot = glyph;
            }
            index = index.saturating_add(1);
        }
    }
    cols
}

fn pixel(rows: [u8; 8], x: u8, y: u8) -> bool {
    if x >= INK_COLUMNS {
        return false;
    }
    let Some(row) = rows.get(usize::from(y)) else {
        return false;
    };
    let shift = 7_u32.saturating_sub(u32::from(x));
    (u32::from(*row) & (1_u32 << shift)) != 0
}

fn cell_glyph(rows: [u8; 8], size: PixelSize, x0: u8, y0: u8) -> char {
    match size {
        PixelSize::Full => {
            if pixel(rows, x0, y0) {
                '█'
            } else {
                ' '
            }
        }
        PixelSize::HalfHeight => half_height(pixel(rows, x0, y0), pixel(rows, x0, y0 + 1)),
        PixelSize::HalfWidth => half_width(pixel(rows, x0, y0), pixel(rows, x0 + 1, y0)),
        PixelSize::Quadrant => {
            let bits = u8::from(pixel(rows, x0, y0))
                | (u8::from(pixel(rows, x0 + 1, y0)) << 1)
                | (u8::from(pixel(rows, x0, y0 + 1)) << 2)
                | (u8::from(pixel(rows, x0 + 1, y0 + 1)) << 3);
            QUADRANT[usize::from(bits)]
        }
        PixelSize::Sextant => sextant(rows, x0, y0),
        PixelSize::Octant => octant(rows, x0, y0),
        PixelSize::Braille => braille(rows, x0, y0),
    }
}

const fn half_height(top: bool, bottom: bool) -> char {
    match (top, bottom) {
        (false, false) => ' ',
        (true, false) => '▀',
        (false, true) => '▄',
        (true, true) => '█',
    }
}

const fn half_width(left: bool, right: bool) -> char {
    match (left, right) {
        (false, false) => ' ',
        (true, false) => '▌',
        (false, true) => '▐',
        (true, true) => '█',
    }
}

fn sextant(rows: [u8; 8], x0: u8, y0: u8) -> char {
    let spots: [(u8, u8, u8); 6] = [
        (0, 0, 0x01),
        (1, 0, 0x02),
        (0, 1, 0x04),
        (1, 1, 0x08),
        (0, 2, 0x10),
        (1, 2, 0x20),
    ];
    let mut ink = 0_u8;
    for (dx, dy, bit) in spots {
        if pixel(rows, x0.saturating_add(dx), y0.saturating_add(dy)) {
            ink |= bit;
        }
    }
    let missing = (!ink) & 0x3f;
    SEXTANT[usize::from(missing)]
}

fn octant(rows: [u8; 8], x0: u8, y0: u8) -> char {
    let mut mask = 0_u8;
    for dy in 0..4_u8 {
        for dx in 0..2_u8 {
            if pixel(rows, x0.saturating_add(dx), y0.saturating_add(dy)) {
                let bit = u32::from(dy.saturating_mul(2).saturating_add(dx));
                mask |= 1_u8 << bit;
            }
        }
    }
    GlyphSet::Octants.glyph(mask)
}

fn braille(rows: [u8; 8], x0: u8, y0: u8) -> char {
    const DOTS: [(u8, u8, u8); 8] = [
        (0, 0, 0x01),
        (0, 1, 0x02),
        (0, 2, 0x04),
        (1, 0, 0x08),
        (1, 1, 0x10),
        (1, 2, 0x20),
        (0, 3, 0x40),
        (1, 3, 0x80),
    ];
    let mut bits = 0_u8;
    for (dx, dy, bit) in DOTS {
        if pixel(rows, x0.saturating_add(dx), y0.saturating_add(dy)) {
            bits |= bit;
        }
    }
    if bits == 0 {
        return ' ';
    }
    char::from_u32(0x2800 + u32::from(bits)).unwrap_or(' ')
}
