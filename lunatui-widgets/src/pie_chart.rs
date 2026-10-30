// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#[cfg(feature = "pixel-pie-chart")]
mod pixel;
#[cfg(feature = "pixel-pie-chart")]
pub use pixel::PixelPieChart;

use lunatui_core::{Attrs, Color, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::raster::{BRAILLE_BITS, QUADRANT, round_to_tick, sextant};
use crate::render_cx::clip_to_coordinates;
use crate::{Marker, RenderCx, Widget, WidgetState, blend_rgb};

const CANDIDATES: [(Token, Option<Token>); 14] = [
    (Token::Accent, None),
    (Token::FocusRing, None),
    (Token::Warn, None),
    (Token::Fail, None),
    (Token::Ok, None),
    (Token::Accent, Some(Token::Fail)),
    (Token::FocusRing, Some(Token::Warn)),
    (Token::Ok, Some(Token::Fail)),
    (Token::Accent, Some(Token::Warn)),
    (Token::FocusRing, Some(Token::Fail)),
    (Token::TextPrimary, None),
    (Token::Accent, Some(Token::FocusRing)),
    (Token::Warn, Some(Token::Fail)),
    (Token::Ok, Some(Token::FocusRing)),
];
const APART: u32 = 60 * 60;
const EXPLODE: f64 = 0.08;
const SWATCH: &str = "●";
const MAX_SUB: usize = 8;

/// One slice of a [`PieChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct PieSlice {
    label: String,
    value: f64,
    color: Option<Color>,
}

impl PieSlice {
    /// A slice with this legend label and value. Negative and NaN values count as zero.
    #[must_use]
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value: if value.is_nan() { 0.0 } else { value.max(0.0) },
            color: None,
        }
    }

    /// Fills the slice with `color` instead of the theme palette.
    #[must_use]
    pub const fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The slice's value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }
}

/// Where a [`PieChart`] draws its disc and its legend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PieLayout {
    /// The cells the disc is drawn in.
    pub pie: Rect,
    /// The legend rows, empty when there is no legend.
    pub legend: Rect,
}

/// A pie or donut chart of labelled slices, with an optional legend.
///
/// The disc is drawn with sub-cell glyphs chosen by the [`Marker`]. `HalfBlock` (the default),
/// `Quadrant`, and `Sextant` fill each cell with up to two slice colors, so edges between slices
/// are as fine as the glyph grid. `Braille` draws the disc as dots, the finest resolution, with
/// each cell in the color of the slice that covers most of it. `Block` and `Dot` take one sample
/// per cell. [`cell_aspect`](Self::cell_aspect) keeps the disc round on cells taller than they are
/// wide.
///
/// Slices run clockwise from [`start_angle`](Self::start_angle) (twelve o'clock by default).
/// [`reveal`](Self::reveal) sweeps the slices in from the start for an animated intro, leaving the
/// rest of the ring as a `surface.1` track. [`highlight`](Self::highlight) pulls one slice outward.
/// [`donut`](Self::donut) cuts a hole in the middle, where [`center`](Self::center) and
/// [`caption`](Self::caption) can sit.
///
/// Slice colors come from the theme. The candidates are `accent`, `focus.ring`, `warn`, `fail`,
/// and `ok`, then even blends of pairs of them, then `text.primary`, then more blends. Under a
/// prepared color profile the blends are downsampled like the tokens. A candidate is skipped when
/// it is too close to one already chosen or to `surface.0` or `surface.1`, so neighbouring slices
/// stay apart in any theme, and the chosen colors repeat when there are more slices; the last
/// slice skips the first color so the two never meet in the same color. A slice can set its own
/// color instead.
///
/// The legend lists each slice with its swatch, label, and a whole percentage, rounded so the
/// shown percentages add up to 100. It goes to the right of the disc, or under it when that leaves
/// a bigger disc. When focused, the highlighted legend row is marked in
/// `focus.ring`; when disabled, slice colors are muted toward `surface.0` and nothing is pulled out.
#[derive(Clone, Debug, PartialEq)]
pub struct PieChart {
    slices: Vec<PieSlice>,
    marker: Marker,
    hole: f64,
    start: f64,
    reveal: f64,
    highlight: Option<usize>,
    legend: bool,
    aspect: f64,
    center: Option<String>,
    caption: Option<String>,
    state: WidgetState,
}

#[derive(Clone, Copy)]
struct Disc {
    cx: f64,
    cy: f64,
    radius: f64,
    hole: f64,
}

struct Sectors {
    start: f64,
    ends: Vec<f64>,
    track: f64,
    pull: Option<(usize, f64, f64)>,
}

impl PieChart {
    /// A pie of these slices, drawn with half blocks, with a legend.
    #[must_use]
    pub fn new(slices: impl IntoIterator<Item = PieSlice>) -> Self {
        Self {
            slices: slices.into_iter().collect(),
            marker: Marker::HalfBlock,
            hole: 0.0,
            start: 0.0,
            reveal: 1.0,
            highlight: None,
            legend: true,
            aspect: 2.0,
            center: None,
            caption: None,
            state: WidgetState::Normal,
        }
    }

    /// The glyphs the disc is drawn with.
    #[must_use]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    /// Cuts a hole of this fraction of the radius, from `0.0` (a full pie) up to `0.9`.
    #[must_use]
    pub fn donut(mut self, hole: f64) -> Self {
        self.hole = if hole.is_nan() {
            0.0
        } else {
            hole.clamp(0.0, 0.9)
        };
        self
    }

    /// Where the first slice starts, in degrees clockwise from twelve o'clock.
    #[must_use]
    pub fn start_angle(mut self, degrees: f64) -> Self {
        self.start = if degrees.is_finite() {
            (degrees / 360.0).rem_euclid(1.0)
        } else {
            0.0
        };
        self
    }

    /// How much of the circle the slices fill, from `0.0` to `1.0`, for sweeping them in. The rest
    /// of the ring is drawn as a track.
    #[must_use]
    pub fn reveal(mut self, reveal: f64) -> Self {
        self.reveal = if reveal.is_nan() {
            0.0
        } else {
            reveal.clamp(0.0, 1.0)
        };
        self
    }

    /// The slice pulled out from the center, or `None`.
    #[must_use]
    pub const fn highlight(mut self, index: Option<usize>) -> Self {
        self.highlight = index;
        self
    }

    /// Whether the legend is drawn. On by default.
    #[must_use]
    pub const fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// How many times taller than wide a cell is, so the disc comes out round. `2.0` by default.
    #[must_use]
    pub fn cell_aspect(mut self, aspect: f64) -> Self {
        self.aspect = if aspect.is_finite() {
            aspect.clamp(0.5, 4.0)
        } else {
            2.0
        };
        self
    }

    /// Bold text in the middle of a donut, such as a total.
    #[must_use]
    pub fn center(mut self, text: impl Into<String>) -> Self {
        self.center = Some(text.into());
        self
    }

    /// Dim text under the center text.
    #[must_use]
    pub fn caption(mut self, text: impl Into<String>) -> Self {
        self.caption = Some(text.into());
        self
    }

    /// Interaction state for the legend and slice colors.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The sum of the slice values.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.slices.iter().map(|slice| slice.value).sum()
    }

    /// Whole percentages for the slices, rounded so they add up to 100 (all zero when the total is
    /// zero).
    #[must_use]
    pub fn percents(&self) -> Vec<u8> {
        let total = self.total();
        if total <= 0.0 {
            return vec![0; self.slices.len()];
        }
        let exact: Vec<f64> = self
            .slices
            .iter()
            .map(|slice| slice.value * 100.0 / total)
            .collect();
        let mut whole: Vec<u8> = exact.iter().map(|share| floor_u8(*share)).collect();
        let given: u32 = whole.iter().map(|share| u32::from(*share)).sum();
        let mut order: Vec<usize> = (0..exact.len()).collect();
        order.sort_by(|a, b| {
            let rest = |index: usize| exact[index] - f64::from(whole[index]);
            rest(*b).total_cmp(&rest(*a)).then(a.cmp(b))
        });
        for index in order
            .into_iter()
            .take(usize::try_from(100 - given).unwrap_or(0))
        {
            whole[index] += 1;
        }
        whole
    }

    /// Where the disc and legend go in `area`.
    #[must_use]
    pub fn layout(&self, area: Rect) -> PieLayout {
        let area = clip_to_coordinates(area);
        let rows = u16::try_from(self.slices.len()).unwrap_or(u16::MAX);
        let legend_width = if self.legend && rows > 0 {
            self.legend_width()
        } else {
            0
        };
        if legend_width == 0 {
            let pie = self.fit(area.width, area.height);
            return PieLayout {
                pie: centered(area, pie.0, pie.1),
                legend: Rect::new(area.x, area.y, 0, 0),
            };
        }
        let side = self.fit(area.width.saturating_sub(legend_width + 2), area.height);
        let under = self.fit(area.width, area.height.saturating_sub(rows + 1));
        if side.0 >= under.0 {
            let pie = Rect::new(area.x, area.y + (area.height - side.1) / 2, side.0, side.1);
            let shown = rows.min(area.height);
            let legend = Rect::new(
                (pie.x + pie.width + 2).min(area.x + area.width),
                area.y + (area.height - shown) / 2,
                legend_width.min(area.width.saturating_sub(pie.width + 2)),
                shown,
            );
            PieLayout { pie, legend }
        } else {
            let pie = Rect::new(
                area.x + (area.width - under.0) / 2,
                area.y,
                under.0,
                under.1,
            );
            let top = pie.y + pie.height + 1;
            let legend = Rect::new(
                area.x + area.width.saturating_sub(legend_width) / 2,
                top.min(area.y + area.height),
                legend_width.min(area.width),
                rows.min((area.y + area.height).saturating_sub(top)),
            );
            PieLayout { pie, legend }
        }
    }

    /// The slice drawn in the cell at `(x, y)`, for hover and clicks.
    #[must_use]
    pub fn slice_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let disc = self.disc(self.layout(area).pie);
        let sectors = self.sectors();
        let point = (f64::from(x) + 0.5, (f64::from(y) + 0.5) * self.aspect);
        sample(disc, &sectors, point).filter(|index| *index < self.slices.len())
    }

    fn legend_width(&self) -> u16 {
        let label = self
            .slices
            .iter()
            .map(|slice| width(&slice.label))
            .max()
            .unwrap_or(0);
        u16::try_from(label + 7).unwrap_or(u16::MAX)
    }

    fn fit(&self, columns: u16, rows: u16) -> (u16, u16) {
        let diameter = f64::from(columns).min(f64::from(rows) * self.aspect);
        let width = floor_u16(diameter).min(columns);
        let height = floor_u16(diameter / self.aspect).min(rows);
        (width, height)
    }

    fn disc(&self, pie: Rect) -> Disc {
        let radius = f64::from(pie.width).min(f64::from(pie.height) * self.aspect) / 2.0;
        let radius = if self.pulled().is_some() {
            radius / (1.0 + EXPLODE)
        } else {
            radius
        };
        Disc {
            cx: f64::from(pie.x) + f64::from(pie.width) / 2.0,
            cy: (f64::from(pie.y) + f64::from(pie.height) / 2.0) * self.aspect,
            radius,
            hole: radius * self.hole,
        }
    }

    fn pulled(&self) -> Option<usize> {
        self.highlight
            .filter(|index| *index < self.slices.len() && self.state != WidgetState::Disabled)
    }

    fn sectors(&self) -> Sectors {
        let total = self.total();
        let start = diamond_at(self.start);
        let mut done = 0.0;
        let ends = self
            .slices
            .iter()
            .map(|slice| {
                done += if total > 0.0 {
                    slice.value / total
                } else {
                    0.0
                };
                sweep(start, self.start, done.min(1.0) * self.reveal)
            })
            .collect();
        let pull = self.pulled().map(|index| {
            let before: f64 = self.slices[..index].iter().map(|slice| slice.value).sum();
            let middle = (before + self.slices[index].value / 2.0) / total * self.reveal;
            let angle = (self.start + middle) * std::f64::consts::TAU;
            (index, angle.sin(), -angle.cos())
        });
        Sectors {
            start,
            ends,
            track: if self.reveal >= 1.0 { 0.0 } else { 4.0 },
            pull,
        }
    }

    fn palette_slot(&self, index: usize, colors: usize) -> usize {
        let slot = index % colors;
        let wraps = index > 0 && index + 1 == self.slices.len() && slot == 0;
        if wraps && colors > 2 { 1 } else { slot }
    }

    fn slice_color(&self, cx: &RenderCx<'_>, palette: &[Color], index: usize) -> Color {
        let color = if index >= self.slices.len() {
            cx.token(Token::Surface1)
        } else {
            self.slices[index]
                .color
                .unwrap_or(palette[self.palette_slot(index, palette.len())])
        };
        if self.state == WidgetState::Disabled {
            mute(color, cx.token(Token::Surface0))
        } else {
            color
        }
    }

    fn draw_disc(&self, cx: &mut RenderCx<'_>, palette: &[Color], pie: Rect) {
        let disc = self.disc(pie);
        let sectors = self.sectors();
        let (columns, rows) = sub_cells(self.marker);
        let mut hits = [None; MAX_SUB];
        for y in pie.y..pie.y + pie.height {
            for x in pie.x..pie.x + pie.width {
                for row in 0..rows {
                    for column in 0..columns {
                        let point = (
                            f64::from(x) + (f64::from(column) + 0.5) / f64::from(columns),
                            (f64::from(y) + (f64::from(row) + 0.5) / f64::from(rows)) * self.aspect,
                        );
                        hits[usize::from(row * columns + column)] = sample(disc, &sectors, point);
                    }
                }
                let count = usize::from(columns * rows);
                self.draw_cell(cx, palette, (x, y), &hits[..count]);
            }
        }
    }

    fn draw_cell(
        &self,
        cx: &mut RenderCx<'_>,
        palette: &[Color],
        (x, y): (u16, u16),
        hits: &[Option<usize>],
    ) {
        let Some(main) = majority(hits, None) else {
            return;
        };
        let fg = self.slice_color(cx, palette, main);
        if self.marker == Marker::Braille {
            let mask = hits
                .iter()
                .enumerate()
                .filter(|(_, hit)| **hit == Some(main))
                .map(|(at, _)| BRAILLE_BITS[at % 2][at / 2])
                .sum::<u8>();
            let glyph = char::from_u32(0x2800 + u32::from(mask)).unwrap_or(' ');
            cx.put_char_patched(x, y, glyph, Style::new().fg(fg));
            return;
        }
        let mask = hits
            .iter()
            .enumerate()
            .filter(|(_, hit)| **hit == Some(main))
            .map(|(at, _)| 1_u8 << at)
            .sum::<u8>();
        let glyph = match self.marker {
            Marker::HalfBlock => ['█', '▀', '▄', '█'][usize::from(mask & 3)],
            Marker::Quadrant => QUADRANT[usize::from(mask & 15)],
            Marker::Sextant => sextant(mask),
            Marker::Dot => '•',
            _ => '█',
        };
        let style = match majority(hits, Some(main)) {
            Some(other) => Style::new().fg(fg).bg(self.slice_color(cx, palette, other)),
            None => Style::new().fg(fg),
        };
        cx.put_char_patched(x, y, glyph, style);
    }

    fn draw_center(&self, cx: &mut RenderCx<'_>, pie: Rect) {
        let room = floor_u16(f64::from(pie.width) * self.hole * 0.75);
        if room == 0 {
            return;
        }
        let lines = [
            self.center.as_deref().map(|text| {
                (
                    text,
                    Style::new()
                        .fg(cx.token(Token::TextPrimary))
                        .with_attrs(Attrs::new().set_bold(true)),
                )
            }),
            self.caption.as_deref().map(|text| {
                (
                    text,
                    Style::new()
                        .fg(cx.token(Token::TextPrimary))
                        .with_attrs(Attrs::new().set_dim(true)),
                )
            }),
        ];
        let shown: Vec<_> = lines.into_iter().flatten().collect();
        let count = u16::try_from(shown.len()).unwrap_or(0);
        let top = pie.y + pie.height.saturating_sub(count) / 2;
        for (line, (text, style)) in (0_u16..).zip(shown) {
            let text = clip_display_prefix(text, room);
            let used = u16::try_from(width(text)).unwrap_or(u16::MAX);
            let (x, y) = (pie.x + pie.width.saturating_sub(used) / 2, top + line);
            cx.keep_backgrounds(Rect::new(x, y, used, 1), |cx| cx.put_str(x, y, text, style));
        }
    }

    fn draw_legend(&self, cx: &mut RenderCx<'_>, palette: &[Color], legend: Rect) {
        if legend.width == 0 {
            return;
        }
        let percents = self.percents();
        let text = cx.token(Token::TextPrimary);
        let disabled = self.state == WidgetState::Disabled;
        for (row, index) in (0..legend.height).zip(0..self.slices.len()) {
            let y = legend.y + row;
            let lit = self.highlight == Some(index) && !disabled;
            let label_color = if lit && self.state == WidgetState::Focused {
                cx.token(Token::FocusRing)
            } else {
                text
            };
            let label = Style::new()
                .fg(label_color)
                .with_attrs(Attrs::new().set_bold(lit).set_dim(disabled));
            let swatch = Style::new().fg(self.slice_color(cx, palette, index));
            cx.put_str(legend.x, y, SWATCH, swatch);
            let percent = format!("{:>3}%", percents[index]);
            let end = legend.x + legend.width;
            let room = legend.width.saturating_sub(7);
            cx.put_str(
                legend.x + 2,
                y,
                clip_display_prefix(&self.slices[index].label, room),
                label,
            );
            if legend.width >= 6 {
                let dim = Style::new()
                    .fg(text)
                    .with_attrs(Attrs::new().set_dim(!lit || disabled).set_bold(lit));
                cx.put_str(end - 4, y, &percent, dim);
            }
        }
    }
}

impl Widget for PieChart {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let layout = self.layout(area);
        let palette = palette(cx);
        self.draw_disc(cx, &palette, layout.pie);
        self.draw_center(cx, layout.pie);
        cx.keep_backgrounds(layout.legend, |cx| {
            self.draw_legend(cx, &palette, layout.legend);
        });
    }

    fn measure(&self, avail: Size) -> Size {
        let layout = self.layout(Rect::new(0, 0, avail.width, avail.height));
        let right = (layout.pie.x + layout.pie.width).max(layout.legend.x + layout.legend.width);
        let bottom = (layout.pie.y + layout.pie.height).max(layout.legend.y + layout.legend.height);
        Size::new(right, bottom)
    }
}

fn sub_cells(marker: Marker) -> (u16, u16) {
    match marker {
        Marker::Braille => (2, 4),
        Marker::Sextant => (2, 3),
        Marker::Quadrant => (2, 2),
        Marker::HalfBlock => (1, 2),
        _ => (1, 1),
    }
}

fn majority(hits: &[Option<usize>], except: Option<usize>) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for hit in hits.iter().flatten() {
        if Some(*hit) == except {
            continue;
        }
        let count = hits.iter().filter(|other| **other == Some(*hit)).count();
        if best.is_none_or(|(_, most)| count > most) {
            best = Some((*hit, count));
        }
    }
    best.map(|(index, _)| index)
}

fn sample(disc: Disc, sectors: &Sectors, (x, y): (f64, f64)) -> Option<usize> {
    if let Some((index, dx, dy)) = sectors.pull {
        let shift = disc.radius * EXPLODE;
        let moved = (x - dx * shift, y - dy * shift);
        if sector_of(disc, sectors, moved) == Some(index) {
            return Some(index);
        }
    }
    sector_of(disc, sectors, (x, y)).filter(|index| Some(*index) != sectors.pull.map(|p| p.0))
}

fn sector_of(disc: Disc, sectors: &Sectors, (x, y): (f64, f64)) -> Option<usize> {
    let (dx, dy) = (x - disc.cx, y - disc.cy);
    let distance = dx * dx + dy * dy;
    if distance > disc.radius * disc.radius || distance < disc.hole * disc.hole {
        return None;
    }
    let at = (diamond(dx, -dy) - sectors.start).rem_euclid(4.0);
    sectors
        .ends
        .iter()
        .position(|end| at < *end)
        .or_else(|| (at < sectors.track).then_some(sectors.ends.len()))
}

fn sweep(start: f64, turn: f64, fraction: f64) -> f64 {
    if fraction >= 1.0 {
        4.0
    } else if fraction <= 0.0 {
        0.0
    } else {
        (diamond_at(turn + fraction) - start).rem_euclid(4.0)
    }
}

fn diamond_at(turn: f64) -> f64 {
    let angle = turn * std::f64::consts::TAU;
    diamond(angle.sin(), angle.cos())
}

fn diamond(right: f64, up: f64) -> f64 {
    if right >= 0.0 && up > 0.0 {
        right / (right + up)
    } else if right > 0.0 {
        1.0 + -up / (right - up)
    } else if up < 0.0 {
        2.0 + -right / (-right - up)
    } else if right < 0.0 {
        3.0 + up / (up - right)
    } else {
        0.0
    }
}

fn palette(cx: &RenderCx<'_>) -> Vec<Color> {
    let surfaces = [cx.token(Token::Surface0), cx.token(Token::Surface1)];
    let mut picked: Vec<Color> = Vec::new();
    for (base, with) in CANDIDATES {
        let color = match with {
            None => Some(cx.token(base)),
            Some(other) => {
                let theme = cx.theme();
                mix(theme.resolve(base), theme.resolve(other)).map(|blend| shown(cx, blend))
            }
        };
        if let Some(color) = color
            && picked
                .iter()
                .chain(&surfaces)
                .all(|seen| apart(color, *seen))
        {
            picked.push(color);
        }
    }
    if picked.is_empty() {
        picked.push(cx.token(Token::Accent));
    }
    picked
}

fn shown(cx: &RenderCx<'_>, color: Color) -> Color {
    #[cfg(feature = "color-profiles")]
    if let Some(profile) = cx.color_profile() {
        return profile.downsample(color);
    }
    let _ = cx;
    color
}

fn apart(a: Color, b: Color) -> bool {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let gap = |x: u8, y: u8| u32::from(x.abs_diff(y)).pow(2);
            gap(ar, br) + gap(ag, bg) + gap(ab, bb) >= APART
        }
        _ => a != b,
    }
}

fn mix(a: Color, b: Color) -> Option<Color> {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let (r, g, b) = blend_rgb((ar, ag, ab), (br, bg, bb), 0.5);
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn mute(color: Color, toward: Color) -> Color {
    match (color, toward) {
        (Color::Rgb(r, g, b), Color::Rgb(tr, tg, tb)) => {
            let (r, g, b) = blend_rgb((r, g, b), (tr, tg, tb), 0.6);
            Color::Rgb(r, g, b)
        }
        _ => color,
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn floor_u16(value: f64) -> u16 {
    u16::try_from(round_to_tick(value.max(0.0).floor())).unwrap_or(u16::MAX)
}

fn floor_u8(value: f64) -> u8 {
    u8::try_from(round_to_tick(value.max(0.0).floor())).unwrap_or(u8::MAX)
}
