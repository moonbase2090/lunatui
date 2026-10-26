// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;

use lunatui_core::{Attrs, Color, Rect, Size, Style, clip_display_prefix, width as display_width};
use lunatui_theme::Token;

use crate::{Block, RenderCx, Widget, WidgetState};

/// Line set a [`LineGauge`] draws its bar with.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineSet {
    /// Light horizontal line (`─`).
    #[default]
    Normal,
    /// Heavy horizontal line (`━`).
    Thick,
    /// Double horizontal line (`═`).
    Double,
    /// Plain hyphen (`-`) for terminals without Unicode box drawing.
    Ascii,
}

impl LineSet {
    /// The one-column symbol this set draws.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Normal => "─",
            Self::Thick => "━",
            Self::Double => "═",
            Self::Ascii => "-",
        }
    }
}

/// Single-line progress bar: a label, then a line that is filled up to `ratio`.
///
/// The optional label is on the left and the value is right-aligned. The remaining
/// track uses heavy filled lines, light unfilled lines and an eighth-block head.
/// Applications animate by supplying frame-clock values through `ratio`.
/// Colors come from theme tokens unless a style is set explicitly.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{LineGauge, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(20, 1);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// LineGauge::new().ratio(0.5).render(Rect::new(0, 0, 20, 1), &mut cx);
/// assert!(grid.row_display(0, &clusters).ends_with("50%"));
/// ```
#[derive(Clone, Debug)]
pub struct LineGauge<'a> {
    block: Option<Block>,
    ratio: f64,
    label: Option<Cow<'a, str>>,
    value_label: Option<Cow<'a, str>>,
    subcell: bool,
    style: Option<Style>,
    label_style: Option<Style>,
    filled_symbol: Cow<'a, str>,
    unfilled_symbol: Cow<'a, str>,
    filled_style: Option<Style>,
    unfilled_style: Option<Style>,
    state: WidgetState,
}

impl Default for LineGauge<'_> {
    fn default() -> Self {
        Self {
            block: None,
            ratio: 0.0,
            label: None,
            value_label: None,
            subcell: true,
            style: None,
            label_style: None,
            filled_symbol: Cow::Borrowed(LineSet::Thick.symbol()),
            unfilled_symbol: Cow::Borrowed(LineSet::Normal.symbol()),
            filled_style: None,
            unfilled_style: None,
            state: WidgetState::Normal,
        }
    }
}

impl<'a> LineGauge<'a> {
    /// An empty gauge at 0%.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Surrounds the gauge with `block`.
    #[must_use]
    pub fn block(mut self, block: Block) -> Self {
        self.block = Some(block);
        self
    }

    /// Progress in `0.0..=1.0`; values outside the range are clamped and NaN counts as zero.
    #[must_use]
    pub fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0)
        };
        self
    }

    /// Sets an optional left-aligned label.
    #[must_use]
    pub fn label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Replace the right-aligned percentage; an empty value hides it.
    #[must_use]
    pub fn value_label(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value_label = Some(value.into());
        self
    }

    /// Enable eighth-cell precision for the filled head. Enabled by default.
    #[must_use]
    pub fn subcell(mut self, enabled: bool) -> Self {
        self.subcell = enabled;
        self
    }

    /// Draws both the filled and the unfilled part with `set`.
    #[must_use]
    pub fn line_set(mut self, set: LineSet) -> Self {
        self.filled_symbol = Cow::Borrowed(set.symbol());
        self.unfilled_symbol = Cow::Borrowed(set.symbol());
        self.subcell = false;
        self
    }

    /// Symbol for the filled part; invalid or non-single-column graphemes fall back to `-`.
    #[must_use]
    pub fn filled_symbol(mut self, symbol: impl Into<Cow<'a, str>>) -> Self {
        self.filled_symbol = valid_symbol(symbol.into());
        self.subcell = false;
        self
    }

    /// Symbol for the unfilled part; invalid or non-single-column graphemes fall back to `-`.
    #[must_use]
    pub fn unfilled_symbol(mut self, symbol: impl Into<Cow<'a, str>>) -> Self {
        self.unfilled_symbol = valid_symbol(symbol.into());
        self
    }

    /// Style applied to the whole area before anything is drawn.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Style of the label, replacing the state's token color.
    #[must_use]
    pub fn label_style(mut self, style: Style) -> Self {
        self.label_style = Some(style);
        self
    }

    /// Style of the filled part, replacing the state's token color.
    #[must_use]
    pub fn filled_style(mut self, style: Style) -> Self {
        self.filled_style = Some(style);
        self
    }

    /// Style of the unfilled part, replacing the state's token color.
    #[must_use]
    pub fn unfilled_style(mut self, style: Style) -> Self {
        self.unfilled_style = Some(style);
        self
    }

    /// Focus uses the focus-ring token below warning thresholds; disabled dims every part.
    #[must_use]
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    fn styles(&self, cx: &RenderCx<'_>) -> [Style; 3] {
        let bg = self.style.map_or(Color::Default, |style| style.bg);
        let tone = |token: Token| Style::new().fg(cx.token(token)).bg(bg);
        let dim = Attrs::default().set_dim(true);
        let fill_token = if self.ratio >= 0.95 {
            Token::Fail
        } else if self.ratio >= 0.8 {
            Token::Warn
        } else if self.state == WidgetState::Focused {
            Token::FocusRing
        } else {
            Token::Accent
        };
        let [mut filled, mut unfilled, mut label] = if self.state == WidgetState::Disabled {
            [
                tone(Token::TextMuted).with_attrs(dim),
                tone(Token::Border).with_attrs(dim),
                tone(Token::TextMuted).with_attrs(dim),
            ]
        } else {
            [
                tone(fill_token),
                tone(Token::Border),
                tone(Token::TextMuted),
            ]
        };
        filled = self.filled_style.unwrap_or(filled);
        unfilled = self.unfilled_style.unwrap_or(unfilled);
        label = self.label_style.unwrap_or(label);
        if self.state == WidgetState::Disabled {
            filled.attrs = filled.attrs.set_dim(true);
            unfilled.attrs = unfilled.attrs.set_dim(true);
            label.attrs = label.attrs.set_dim(true);
        }
        [filled, unfilled, label]
    }
}

fn valid_symbol(symbol: Cow<'_, str>) -> Cow<'_, str> {
    if symbol.graphemes(true).count() == 1
        && display_width(&symbol) == 1
        && !symbol.chars().any(char::is_control)
    {
        symbol
    } else {
        Cow::Borrowed("-")
    }
}

fn paint(cx: &mut RenderCx<'_>, x: u16, y: u16, limit: u16, text: &str, style: Style) -> u16 {
    if x >= limit {
        return x;
    }
    let draw = clip_display_prefix(text, limit - x);
    let drawn = u16::try_from(display_width(draw)).unwrap_or(0);
    if drawn == 0 {
        return x;
    }
    cx.put_str(x, y, draw, style);
    x + drawn
}

fn repeat(cx: &mut RenderCx<'_>, area: Rect, symbol: &str, style: Style) {
    let mut chars = symbol.chars();
    if let Some(glyph) = chars.next()
        && chars.next().is_none()
    {
        cx.fill(area, style, glyph);
        return;
    }
    let right = area.x.saturating_add(area.width);
    for x in area.x..right {
        paint(cx, x, area.y, right, symbol, style);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Private ratio is finite and clamped to 0..=1; output fits in 524280 eighths"
)]
fn filled_eighths(columns: u16, ratio: f64) -> u32 {
    (f64::from(columns) * ratio * 8.0).floor() as u32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Private ratio is finite and clamped to 0..=1; rounded percent fits in u8"
)]
fn percent_label(ratio: f64, buffer: &mut [u8; 4]) -> &str {
    let percent = (ratio * 100.0).round() as u8;
    let start = if percent == 100 {
        *buffer = *b"100%";
        0
    } else if percent >= 10 {
        buffer[1] = b'0' + percent / 10;
        buffer[2] = b'0' + percent % 10;
        buffer[3] = b'%';
        1
    } else {
        buffer[2] = b'0' + percent;
        buffer[3] = b'%';
        2
    };
    core::str::from_utf8(&buffer[start..]).unwrap_or("")
}

impl Widget for LineGauge<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if let Some(style) = self.style {
            cx.set_style(area, style);
        }
        let inner = if let Some(block) = &self.block {
            block.render(area, cx);
            block.inner(area)
        } else {
            area
        };
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let [filled_style, unfilled_style, label_style] = self.styles(cx);
        cx.fill(
            Rect::new(inner.x, inner.y, inner.width, 1),
            self.style.unwrap_or_default(),
            ' ',
        );
        let right = inner.x.saturating_add(inner.width);
        let mut buffer = [0_u8; 4];
        let value = self
            .value_label
            .as_deref()
            .unwrap_or_else(|| percent_label(self.ratio, &mut buffer));
        let visible_value = clip_display_prefix(value, right.saturating_sub(inner.x));
        let value_width = u16::try_from(display_width(visible_value)).unwrap_or(inner.width);
        let value_start = right - value_width;
        let label_limit = value_start
            .saturating_sub(u16::from(value_width != 0))
            .max(inner.x);
        let label = self.label.as_deref().unwrap_or("");
        let label_end = paint(cx, inner.x, inner.y, label_limit, label, label_style);
        let start = if self.label.is_some() {
            label_end.saturating_add(1)
        } else {
            inner.x
        };
        let end = label_limit.max(start).min(right);
        let total = filled_eighths(end.saturating_sub(start), self.ratio);
        let full = u16::try_from(total / 8).unwrap_or(0);
        let fraction = u8::try_from(total % 8).unwrap_or(0);
        repeat(
            cx,
            Rect::new(start, inner.y, full, 1),
            &self.filled_symbol,
            filled_style,
        );
        let unfilled_start = start.saturating_add(full);
        repeat(
            cx,
            Rect::new(
                unfilled_start,
                inner.y,
                end.saturating_sub(unfilled_start),
                1,
            ),
            &self.unfilled_symbol,
            unfilled_style,
        );
        if self.subcell && fraction != 0 && start.saturating_add(full) < end {
            let mut encoded = [0_u8; 4];
            let cap = lunatui_core::subcell::horizontal_eighths(fraction).encode_utf8(&mut encoded);
            paint(cx, start + full, inner.y, end, cap, filled_style);
        }
        paint(cx, value_start, inner.y, right, visible_value, label_style);
    }

    fn measure(&self, avail: Size) -> Size {
        let height = if self.block.is_some() { 3 } else { 1 };
        Size::new(avail.width, height).clamp(avail)
    }
}
