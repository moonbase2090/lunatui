// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style};
use lunatui_theme::Token;

use crate::render_cx::patch_style;
use crate::{Block, RenderCx, Widget, WidgetState};

/// One column in a [`Sparkline`].
///
/// `None` is a missing sample. Zero is a present sample with no height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SparklineBar {
    value: Option<u64>,
    style: Option<Style>,
}

impl SparklineBar {
    /// A present sample.
    #[must_use]
    pub const fn value(value: u64) -> Self {
        Self {
            value: Some(value),
            style: None,
        }
    }

    /// A missing sample.
    #[must_use]
    pub const fn absent() -> Self {
        Self {
            value: None,
            style: None,
        }
    }

    /// Style layered on the sparkline style. [`Color::Default`] leaves that channel alone.
    #[must_use]
    pub fn style(mut self, style: impl Into<Option<Style>>) -> Self {
        self.style = style.into();
        self
    }

    /// Sample value, when this column is present.
    #[must_use]
    pub const fn sample(self) -> Option<u64> {
        self.value
    }
}

impl From<u64> for SparklineBar {
    fn from(value: u64) -> Self {
        Self::value(value)
    }
}

impl From<Option<u64>> for SparklineBar {
    fn from(value: Option<u64>) -> Self {
        Self { value, style: None }
    }
}

/// A value that can be drawn as one sparkline column.
pub trait SparklineValue {
    /// Column value and optional style.
    fn sparkline_bar(&self) -> SparklineBar;
}

impl SparklineValue for u64 {
    fn sparkline_bar(&self) -> SparklineBar {
        SparklineBar::value(*self)
    }
}

impl SparklineValue for Option<u64> {
    fn sparkline_bar(&self) -> SparklineBar {
        SparklineBar::from(*self)
    }
}

impl SparklineValue for SparklineBar {
    fn sparkline_bar(&self) -> SparklineBar {
        *self
    }
}

/// Which end of the area receives the first sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RenderDirection {
    /// First sample on the left.
    #[default]
    LeftToRight,
    /// First sample on the right.
    RightToLeft,
}

/// How sample values map onto bar height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Baseline {
    /// Zero is the bottom of the bar, matching ratatui.
    #[default]
    Zero,
    /// The smallest sample is the bottom of the bar.
    Minimum,
}

/// Glyphs for the nine bar heights, from empty through full.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BarSet {
    /// No fill.
    pub empty: &'static str,
    /// One eighth.
    pub one_eighth: &'static str,
    /// One quarter.
    pub one_quarter: &'static str,
    /// Three eighths.
    pub three_eighths: &'static str,
    /// Half.
    pub half: &'static str,
    /// Five eighths.
    pub five_eighths: &'static str,
    /// Three quarters.
    pub three_quarters: &'static str,
    /// Seven eighths.
    pub seven_eighths: &'static str,
    /// Full cell.
    pub full: &'static str,
}

impl Default for BarSet {
    fn default() -> Self {
        NINE_LEVELS
    }
}

impl BarSet {
    fn glyph(self, height: u64) -> &'static str {
        match height {
            0 => self.empty,
            1 => self.one_eighth,
            2 => self.one_quarter,
            3 => self.three_eighths,
            4 => self.half,
            5 => self.five_eighths,
            6 => self.three_quarters,
            7 => self.seven_eighths,
            _ => self.full,
        }
    }
}

const FULL: &str = "█";
const SEVEN_EIGHTHS: &str = "▇";
const THREE_QUARTERS: &str = "▆";
const FIVE_EIGHTHS: &str = "▅";
const HALF: &str = "▄";
const THREE_EIGHTHS: &str = "▃";
const ONE_QUARTER: &str = "▂";
const ONE_EIGHTH: &str = "▁";

/// Three visible heights: empty, half, and full.
pub const THREE_LEVELS: BarSet = BarSet {
    full: FULL,
    seven_eighths: FULL,
    three_quarters: HALF,
    five_eighths: HALF,
    half: HALF,
    three_eighths: HALF,
    one_quarter: HALF,
    one_eighth: " ",
    empty: " ",
};

/// Eight block steps plus an empty cell. This is the default set.
pub const NINE_LEVELS: BarSet = BarSet {
    full: FULL,
    seven_eighths: SEVEN_EIGHTHS,
    three_quarters: THREE_QUARTERS,
    five_eighths: FIVE_EIGHTHS,
    half: HALF,
    three_eighths: THREE_EIGHTHS,
    one_quarter: ONE_QUARTER,
    one_eighth: ONE_EIGHTH,
    empty: " ",
};

/// A column chart of sampled values, one bar per cell of width.
///
/// Heights use the eight block steps in [`NINE_LEVELS`]. An explicit [`style`](Sparkline::style)
/// is painted as given, which is how the ratatui parity case locks colors. Without that style,
/// bars take semantic tokens from [`WidgetState`]: the series uses primary text, the latest
/// sample uses the accent, focus uses the focus ring, and disabled bars are dim.
#[derive(Clone, Debug)]
pub struct Sparkline<'a, T> {
    data: &'a [T],
    block: Option<Block>,
    style: Style,
    styled: bool,
    absent_style: Style,
    absent_symbol: &'a str,
    max: Option<u64>,
    bars: BarSet,
    direction: RenderDirection,
    baseline: Baseline,
    state: WidgetState,
}

impl<'a, T> Sparkline<'a, T> {
    /// Sparkline over `data`, scaled to the tallest sample.
    ///
    /// # Examples
    ///
    /// ```
    /// use lunatui_core::{ClusterTable, Grid, Rect, Style};
    /// use lunatui_theme::{Theme, Variant};
    /// use lunatui_widgets::{RenderCx, Sparkline, Widget};
    ///
    /// let data = [0_u64, 2, 4, 8];
    /// let theme = Theme::builtin(Variant::Dark);
    /// let mut grid = Grid::new(4, 1);
    /// let mut clusters = ClusterTable::new();
    /// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    /// Sparkline::new(&data)
    ///     .style(Style::new())
    ///     .render(Rect::new(0, 0, 4, 1), &mut cx);
    /// ```
    #[must_use]
    pub fn new(data: &'a [T]) -> Self {
        Self {
            data,
            block: None,
            style: Style::new(),
            styled: false,
            absent_style: Style::new(),
            absent_symbol: " ",
            max: None,
            bars: NINE_LEVELS,
            direction: RenderDirection::LeftToRight,
            baseline: Baseline::Zero,
            state: WidgetState::Normal,
        }
    }

    /// Optional surrounding block. The bars occupy its inner area.
    #[must_use]
    pub fn block(mut self, block: Block) -> Self {
        self.block = Some(block);
        self
    }

    /// Style for every present bar. Set this to match an explicit ratatui style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.styled = true;
        self
    }

    /// Style layered onto the sparkline style for missing samples.
    #[must_use]
    pub fn absent_value_style(mut self, style: Style) -> Self {
        self.absent_style = style;
        self
    }

    /// Glyph used for every row of a missing sample. Defaults to a space.
    #[must_use]
    pub const fn absent_value_symbol(mut self, symbol: &'a str) -> Self {
        self.absent_symbol = symbol;
        self
    }

    /// Upper end of the scale. Without it, the tallest present sample is used.
    #[must_use]
    pub const fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Bar glyphs. [`NINE_LEVELS`] is the default and [`THREE_LEVELS`] is the coarser set.
    #[must_use]
    pub const fn bar_set(mut self, bars: BarSet) -> Self {
        self.bars = bars;
        self
    }

    /// Which side the first sample is drawn on.
    #[must_use]
    pub const fn direction(mut self, direction: RenderDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Zero baseline matches ratatui. [`Baseline::Minimum`] lifts the floor to the smallest sample.
    #[must_use]
    pub const fn baseline(mut self, baseline: Baseline) -> Self {
        self.baseline = baseline;
        self
    }

    /// Interaction state used when no explicit [`style`](Self::style) is set.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }
}

impl<T: SparklineValue> Widget for Sparkline<'_, T> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let inner = if let Some(block) = &self.block {
            block.render(area, cx);
            block.inner(area)
        } else {
            area
        };
        self.render_bars(inner, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let columns = u16::try_from(self.data.len()).unwrap_or(u16::MAX);
        Size::new(columns, 1).clamp(avail)
    }
}

impl<T: SparklineValue> Sparkline<'_, T> {
    fn render_bars(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let limit = usize::from(area.width).min(self.data.len());
        let (floor, scale) = self.scale_ends();
        for index in 0..limit {
            let bar = self.data[index].sparkline_bar();
            let column = u16::try_from(index).unwrap_or(0);
            let x = match self.direction {
                RenderDirection::LeftToRight => area.x.saturating_add(column),
                RenderDirection::RightToLeft => area
                    .x
                    .saturating_add(area.width.saturating_sub(1))
                    .saturating_sub(column),
            };
            let (height, symbol, style) = match bar.sample() {
                Some(value) => (
                    scale_height(value.saturating_sub(floor), scale, area.height),
                    None,
                    bar.style,
                ),
                None => (
                    u64::from(area.height).saturating_mul(8),
                    Some(self.absent_symbol),
                    Some(self.absent_style),
                ),
            };
            let column_style = patch_style(
                self.column_style(cx, index + 1 == limit),
                style.unwrap_or_default(),
            );
            self.paint_column(cx, x, area, height, symbol, column_style);
        }
    }

    fn paint_column(
        &self,
        cx: &mut RenderCx<'_>,
        x: u16,
        area: Rect,
        height: u64,
        symbol: Option<&str>,
        style: Style,
    ) {
        let mut remaining = height;
        for row in (0..area.height).rev() {
            let glyph = symbol.unwrap_or_else(|| self.bars.glyph(remaining));
            if remaining > 8 {
                remaining -= 8;
            } else {
                remaining = 0;
            }
            cx.put_char_patched(x, area.y.saturating_add(row), first_char(glyph), style);
        }
    }

    fn scale_ends(&self) -> (u64, u64) {
        let mut peak = 0_u64;
        let mut floor = u64::MAX;
        let mut any = false;
        for item in self.data {
            if let Some(value) = item.sparkline_bar().sample() {
                any = true;
                peak = peak.max(value);
                floor = floor.min(value);
            }
        }
        let max = self.max.unwrap_or(if any { peak } else { 1 });
        match self.baseline {
            Baseline::Zero => (0, max),
            Baseline::Minimum => {
                let span = max.saturating_sub(floor);
                if !any || span == 0 {
                    (0, max)
                } else {
                    (floor, span)
                }
            }
        }
    }

    fn column_style(&self, cx: &RenderCx<'_>, last: bool) -> Style {
        if self.styled {
            return self.style;
        }
        let (token, bold, dim) = match self.state {
            WidgetState::Disabled => (Token::TextPrimary, false, true),
            WidgetState::Focused if last => (Token::FocusRing, true, false),
            WidgetState::Focused => (Token::Accent, false, false),
            WidgetState::Normal if last => (Token::Accent, false, false),
            WidgetState::Normal => (Token::TextPrimary, false, false),
        };
        Style::new()
            .fg(cx.token(token))
            .with_attrs(Attrs::new().set_bold(bold).set_dim(dim))
    }
}

fn first_char(symbol: &str) -> char {
    symbol.chars().next().unwrap_or(' ')
}

fn scale_height(value: u64, scale: u64, rows: u16) -> u64 {
    if scale == 0 {
        return 0;
    }
    let max_ticks = u128::from(rows).saturating_mul(8);
    let ticks = u128::from(value).saturating_mul(max_ticks) / u128::from(scale);
    u64::try_from(ticks.min(max_ticks)).unwrap_or(u64::MAX)
}
