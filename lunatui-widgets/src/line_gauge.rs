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
/// Matches ratatui's `LineGauge` layout: the label starts the first row of the area, one column
/// is left empty after it, and the remaining columns hold `floor(columns * ratio)` filled cells
/// followed by unfilled cells. Colors come from theme tokens unless a style is set explicitly.
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
/// assert!(grid.row_display(0, &clusters).starts_with(" 50% "));
/// ```
#[derive(Clone, Debug)]
pub struct LineGauge<'a> {
    block: Option<Block>,
    ratio: f64,
    label: Option<Cow<'a, str>>,
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
            style: None,
            label_style: None,
            filled_symbol: Cow::Borrowed(LineSet::Normal.symbol()),
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

    /// Replaces the default `NN%` label.
    #[must_use]
    pub fn label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draws both the filled and the unfilled part with `set`.
    #[must_use]
    pub fn line_set(mut self, set: LineSet) -> Self {
        self.filled_symbol = Cow::Borrowed(set.symbol());
        self.unfilled_symbol = Cow::Borrowed(set.symbol());
        self
    }

    /// Symbol for the filled part; invalid or non-single-column graphemes fall back to `-`.
    #[must_use]
    pub fn filled_symbol(mut self, symbol: impl Into<Cow<'a, str>>) -> Self {
        self.filled_symbol = valid_symbol(symbol.into());
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

    /// Paints for `state`: focus uses the focus-ring token, disabled dims every part.
    #[must_use]
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    fn styles(&self, cx: &RenderCx<'_>) -> [Style; 3] {
        let bg = self.style.map_or(Color::Default, |style| style.bg);
        let tone = |token: Token| Style::new().fg(cx.token(token)).bg(bg);
        let dim = Attrs::default().set_dim(true);
        let [mut filled, mut unfilled, mut label] = match self.state {
            WidgetState::Normal => [
                tone(Token::Accent),
                tone(Token::Surface1),
                tone(Token::TextPrimary),
            ],
            WidgetState::Focused => [
                tone(Token::FocusRing),
                tone(Token::Surface1),
                tone(Token::TextPrimary),
            ],
            WidgetState::Disabled => [
                tone(Token::TextPrimary).with_attrs(dim),
                tone(Token::Surface1).with_attrs(dim),
                tone(Token::TextPrimary).with_attrs(dim),
            ],
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
    cx.set_style(Rect::new(x, y, drawn, 1), style);
    x + drawn
}

fn filled_columns(columns: u16, ratio: f64) -> u16 {
    let limit = f64::from(columns) * ratio;
    let count = (1..=columns)
        .take_while(|column| f64::from(*column) <= limit)
        .count();
    u16::try_from(count).unwrap_or(columns)
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
        let right = inner.x.saturating_add(inner.width);
        let default_label;
        let label = if let Some(label) = &self.label {
            label.as_ref()
        } else {
            default_label = format!("{:3.0}%", self.ratio * 100.0);
            default_label.as_str()
        };
        let label_end = paint(cx, inner.x, inner.y, right, label, label_style);
        let start = label_end.saturating_add(1);
        if start >= right {
            return;
        }
        let end = start + filled_columns(right - start, self.ratio);
        for column in start..end {
            paint(
                cx,
                column,
                inner.y,
                right,
                &self.filled_symbol,
                filled_style,
            );
        }
        for column in end..right {
            paint(
                cx,
                column,
                inner.y,
                right,
                &self.unfilled_symbol,
                unfilled_style,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let height = if self.block.is_some() { 3 } else { 1 };
        Size::new(avail.width, height).clamp(avail)
    }
}
