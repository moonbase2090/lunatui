// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, text_display_width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

#[cfg(feature = "pixel-barchart")]
mod pixel;

fn clip_area_to_coordinates(area: Rect) -> Rect {
    Rect::new(
        area.x,
        area.y,
        area.width.min(u16::MAX - area.x),
        area.height.min(u16::MAX - area.y),
    )
}

const EIGHTHS_UP: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const EIGHTHS_ACROSS: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

/// One bar: a value with an optional label, an optional text shown on the bar, and an optional
/// style.
#[derive(Clone, Copy, Debug)]
pub struct Bar<'a> {
    value: u64,
    label: Option<&'a str>,
    text_value: Option<&'a str>,
    style: Option<Style>,
}

impl<'a> Bar<'a> {
    /// A bar of `value`, without a label.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            label: None,
            text_value: None,
            style: None,
        }
    }

    /// Label drawn under the bar (vertical) or left of it (horizontal).
    #[must_use]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Text drawn on the bar in place of its number. Setting it shows the bar's value text even
    /// when [`BarChart::show_values`] is off.
    #[must_use]
    pub const fn text_value(mut self, text: &'a str) -> Self {
        self.text_value = Some(text);
        self
    }

    /// Style of this bar, replacing the chart's bar style.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

/// A run of bars that share a label and sit together, with the chart's group gap between groups.
#[derive(Clone, Copy, Debug)]
pub struct BarGroup<'a> {
    label: Option<&'a str>,
    bars: &'a [Bar<'a>],
}

impl<'a> BarGroup<'a> {
    /// A group of `bars`, without a label.
    #[must_use]
    pub const fn new(bars: &'a [Bar<'a>]) -> Self {
        Self { label: None, bars }
    }

    /// Label centered under the group (vertical) or on a line above it (horizontal).
    #[must_use]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Direction bars grow in.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarDirection {
    /// Bars rise from the bottom of the area.
    #[default]
    Vertical,
    /// Bars grow right from the left of the area.
    Horizontal,
}

#[derive(Clone, Copy, Debug)]
enum Data<'a> {
    Bars(&'a [Bar<'a>]),
    Groups(&'a [BarGroup<'a>]),
}

/// Bar chart over a series of bars or groups of bars, vertical or horizontal.
///
/// Every bar is scaled to the largest value in the chart, or to [`max`](Self::max). Partial
/// lengths use native antialiased fills with a measured image presenter, or eighth blocks
/// in cell mode. [`pixels(false)`](Self::pixels) selects cells explicitly.
/// `bar_width` is the thickness of a bar
/// (columns when vertical, rows when horizontal); `bar_gap` separates bars in a group and
/// `group_gap` separates groups. Bar and label colors come from the theme: bars use the accent
/// token, the focus ring when focused, and dimmed text when disabled, unless a
/// [`style`](Self::style) is set.
///
/// ```
/// use lunatui_core::{ClusterTable, Grid, Rect};
/// use lunatui_theme::{Theme, Variant};
/// use lunatui_widgets::{Bar, BarChart, RenderCx, Widget};
///
/// let theme = Theme::builtin(Variant::Dark);
/// let mut grid = Grid::new(5, 2);
/// let mut clusters = ClusterTable::new();
/// let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
/// let bars = [Bar::new(0), Bar::new(8)];
/// BarChart::from_bars(&bars).render(Rect::new(0, 0, 5, 2), &mut cx);
/// assert_eq!(grid.row_display(1, &clusters), "  █  ");
/// ```
#[derive(Clone, Debug)]
pub struct BarChart<'a> {
    data: Data<'a>,
    direction: BarDirection,
    bar_width: u16,
    bar_gap: u16,
    group_gap: u16,
    max: Option<u64>,
    show_values: bool,
    state: WidgetState,
    background: Option<Style>,
    bar_style: Option<Style>,
    label_style: Option<Style>,
    value_style: Option<Style>,
    pixels: bool,
}

impl<'a> BarChart<'a> {
    /// A chart of `bars` in one unlabeled group.
    #[must_use]
    pub const fn from_bars(bars: &'a [Bar<'a>]) -> Self {
        Self::with_data(Data::Bars(bars))
    }

    /// A chart of `groups`.
    #[must_use]
    pub const fn from_groups(groups: &'a [BarGroup<'a>]) -> Self {
        Self::with_data(Data::Groups(groups))
    }

    const fn with_data(data: Data<'a>) -> Self {
        Self {
            data,
            direction: BarDirection::Vertical,
            bar_width: 1,
            bar_gap: 1,
            group_gap: 2,
            max: None,
            show_values: false,
            state: WidgetState::Normal,
            background: None,
            bar_style: None,
            label_style: None,
            value_style: None,
            pixels: true,
        }
    }

    /// Direction bars grow in; vertical by default.
    #[must_use]
    pub const fn direction(mut self, direction: BarDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Thickness of each bar in cells, at least 1; 1 by default.
    #[must_use]
    pub const fn bar_width(mut self, width: u16) -> Self {
        self.bar_width = if width == 0 { 1 } else { width };
        self
    }

    /// Cells between bars in the same group; 1 by default.
    #[must_use]
    pub const fn bar_gap(mut self, gap: u16) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Cells between groups; 2 by default.
    #[must_use]
    pub const fn group_gap(mut self, gap: u16) -> Self {
        self.group_gap = gap;
        self
    }

    /// Value that fills the whole length; defaults to the largest value.
    #[must_use]
    pub const fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Draw each bar's number on it when the bar has no [`Bar::text_value`]; off by default.
    #[must_use]
    pub const fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Interaction state used for colors that are not set explicitly.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Style the area is reset to before anything is drawn, such as a panel background. Without
    /// one, cells are reset to the default style.
    #[must_use]
    pub const fn background(mut self, style: Style) -> Self {
        self.background = Some(style);
        self
    }

    /// Style of every bar that has no style of its own.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.bar_style = Some(style);
        self
    }

    /// Style of bar and group labels.
    #[must_use]
    pub const fn label_style(mut self, style: Style) -> Self {
        self.label_style = Some(style);
        self
    }

    /// Style of value text drawn outside a bar.
    #[must_use]
    pub const fn value_style(mut self, style: Style) -> Self {
        self.value_style = Some(style);
        self
    }

    /// Enables native antialiased fills when the active image presenter supports them.
    /// Missing support or a declined raster retains the original cell renderer.
    /// Enabled by default; `false` selects cells explicitly.
    #[must_use]
    pub const fn pixels(mut self, enabled: bool) -> Self {
        self.pixels = enabled;
        self
    }

    fn groups(&self) -> impl Iterator<Item = (Option<&'a str>, &'a [Bar<'a>])> + '_ {
        let (bars, groups) = match self.data {
            Data::Bars(bars) => (Some(bars), &[][..]),
            Data::Groups(groups) => (None, groups),
        };
        bars.map(|bars| (None, bars))
            .into_iter()
            .chain(groups.iter().map(|group| (group.label, group.bars)))
    }

    fn largest(&self) -> u64 {
        self.groups()
            .flat_map(|(_, bars)| bars.iter().map(|bar| bar.value))
            .max()
            .unwrap_or(0)
    }

    fn colors(&self, cx: &RenderCx<'_>) -> Colors {
        let dim = Attrs::new().set_dim(self.state == WidgetState::Disabled);
        let bar_token = match self.state {
            WidgetState::Focused => Token::FocusRing,
            WidgetState::Disabled => Token::TextPrimary,
            _ => Token::Accent,
        };
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .with_attrs(dim);
        Colors {
            bar: self
                .bar_style
                .unwrap_or_else(|| Style::new().fg(cx.token(bar_token)).with_attrs(dim)),
            label: self.label_style.unwrap_or(text),
            value: self.value_style.unwrap_or(text),
            on_bar: cx.token(Token::Surface0),
        }
    }

    fn value_text(&self, bar: &Bar<'a>) -> Option<String> {
        match bar.text_value {
            Some(text) => Some(text.to_owned()),
            None if self.show_values => Some(bar.value.to_string()),
            None => None,
        }
    }
}

struct Colors {
    bar: Style,
    label: Style,
    value: Style,
    on_bar: lunatui_core::Color,
}

fn ticks(value: u64, max: u64, cells: u16) -> u16 {
    if max == 0 {
        return 0;
    }
    let total = u128::from(cells) * 8;
    let scaled = (u128::from(value) * total / u128::from(max)).min(total);
    u16::try_from(scaled).unwrap_or(u16::MAX)
}

fn put_styled(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    let mut column = x;
    for glyph in text.chars() {
        cx.put_char_patched(column, y, glyph, style);
        let mut buffer = [0_u8; 4];
        let used = text_display_width(glyph.encode_utf8(&mut buffer));
        column = column.saturating_add(u16::try_from(used).unwrap_or(1));
    }
}

fn fit(text: &str, width: u16) -> String {
    if text_display_width(text) <= usize::from(width) {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut cut = clip_display_prefix(text, width - 1).to_owned();
    cut.push('…');
    cut
}

#[derive(Clone, Copy)]
struct BarPlacement {
    track: Rect,
    direction: BarDirection,
    value: u64,
    max: u64,
    style: Style,
}

#[derive(Clone, Copy)]
enum TextAlign {
    Start,
    Center,
}

#[derive(Clone, Copy)]
struct TextPlacement<'a> {
    x: u16,
    y: u16,
    width: u16,
    text: &'a str,
    style: Style,
    align: TextAlign,
}

trait Visitor {
    type Error;

    fn bar(&mut self, bar: BarPlacement) -> Result<(), Self::Error>;
    fn text(&mut self, text: TextPlacement<'_>) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy)]
enum Plot {
    Vertical { area: Rect, bar_labels: bool },
    Horizontal { area: Rect, label_width: u16 },
}

impl Plot {
    #[cfg(feature = "pixel-barchart")]
    fn area(self) -> Rect {
        match self {
            Self::Vertical { area, .. } | Self::Horizontal { area, .. } => area,
        }
    }
}

struct Layout<'chart, 'data> {
    chart: &'chart BarChart<'data>,
    area: Rect,
    max: u64,
    colors: Colors,
    plot: Plot,
}

impl<'chart, 'data> Layout<'chart, 'data> {
    fn new(chart: &'chart BarChart<'data>, area: Rect, cx: &RenderCx<'_>) -> Self {
        let plot = match chart.direction {
            BarDirection::Vertical => {
                let bar_labels = chart
                    .groups()
                    .any(|(_, bars)| bars.iter().any(|bar| bar.label.is_some()));
                let group_labels = chart.groups().any(|(label, _)| label.is_some());
                Plot::Vertical {
                    area: Rect::new(
                        area.x,
                        area.y,
                        area.width,
                        area.height
                            .saturating_sub(u16::from(bar_labels) + u16::from(group_labels)),
                    ),
                    bar_labels,
                }
            }
            BarDirection::Horizontal => {
                let width = chart
                    .groups()
                    .flat_map(|(_, bars)| bars.iter())
                    .filter_map(|bar| bar.label)
                    .map(text_display_width)
                    .max()
                    .unwrap_or(0);
                let label_width = u16::try_from(width).unwrap_or(u16::MAX).min(area.width / 2);
                let x = area
                    .x
                    .saturating_add(label_width)
                    .saturating_add(u16::from(label_width > 0));
                Plot::Horizontal {
                    area: Rect::new(
                        x,
                        area.y,
                        area.x.saturating_add(area.width).saturating_sub(x),
                        area.height,
                    ),
                    label_width,
                }
            }
        };
        Self {
            chart,
            area,
            max: chart.max.unwrap_or_else(|| chart.largest()),
            colors: chart.colors(cx),
            plot,
        }
    }

    fn visit<V: Visitor>(&self, visitor: &mut V) -> Result<(), V::Error> {
        match self.plot {
            Plot::Vertical { area, bar_labels } => self.vertical(area, bar_labels, visitor),
            Plot::Horizontal { area, label_width } => self.horizontal(area, label_width, visitor),
        }
    }

    fn vertical<V: Visitor>(
        &self,
        plot: Rect,
        bar_labels: bool,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        let bottom = plot.y.saturating_add(plot.height);
        let group_y = bottom.saturating_add(u16::from(bar_labels));
        let right = self.area.x.saturating_add(self.area.width);
        let mut x = self.area.x;
        for (group_index, (group_label, bars)) in self.chart.groups().enumerate() {
            if group_index > 0 {
                x = x.saturating_add(self.chart.group_gap);
            }
            let start = x;
            for (bar_index, bar) in bars.iter().enumerate() {
                if bar_index > 0 {
                    x = x.saturating_add(self.chart.bar_gap);
                }
                if x >= right {
                    return Ok(());
                }
                let width = self.chart.bar_width.min(right - x);
                let style = bar.style.unwrap_or(self.colors.bar);
                let filled = ticks(bar.value, self.max, plot.height);
                visitor.bar(BarPlacement {
                    track: Rect::new(x, plot.y, width, plot.height),
                    direction: BarDirection::Vertical,
                    value: bar.value,
                    max: self.max,
                    style,
                })?;
                if let Some(text) = self.chart.value_text(bar)
                    && plot.height > 0
                {
                    let used = filled / 8 + u16::from(!filled.is_multiple_of(8));
                    let (style, y) = if used == 0 {
                        (self.colors.value, bottom - 1)
                    } else if used < plot.height {
                        (self.colors.value, bottom - 1 - used)
                    } else {
                        (
                            Style::new().fg(self.colors.on_bar).bg(style.fg),
                            bottom - plot.height,
                        )
                    };
                    visitor.text(TextPlacement {
                        x,
                        y,
                        width,
                        text: &text,
                        style,
                        align: TextAlign::Center,
                    })?;
                }
                if let Some(text) = bar.label {
                    visitor.text(TextPlacement {
                        x,
                        y: bottom,
                        width,
                        text,
                        style: self.colors.label,
                        align: TextAlign::Center,
                    })?;
                }
                x = x.saturating_add(self.chart.bar_width);
            }
            if let Some(text) = group_label {
                visitor.text(TextPlacement {
                    x: start,
                    y: group_y,
                    width: x.min(right).saturating_sub(start),
                    text,
                    style: self.colors.label,
                    align: TextAlign::Center,
                })?;
            }
        }
        Ok(())
    }

    fn horizontal<V: Visitor>(
        &self,
        plot: Rect,
        label_width: u16,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        let right = self.area.x.saturating_add(self.area.width);
        let bottom = self.area.y.saturating_add(self.area.height);
        let mut y = self.area.y;
        for (group_index, (group_label, bars)) in self.chart.groups().enumerate() {
            if group_index > 0 {
                y = y.saturating_add(self.chart.group_gap);
            }
            if let Some(text) = group_label {
                if y >= bottom {
                    return Ok(());
                }
                visitor.text(TextPlacement {
                    x: self.area.x,
                    y,
                    width: self.area.width,
                    text,
                    style: self.colors.label,
                    align: TextAlign::Start,
                })?;
                y = y.saturating_add(1);
            }
            for (bar_index, bar) in bars.iter().enumerate() {
                if bar_index > 0 {
                    y = y.saturating_add(self.chart.bar_gap);
                }
                if y >= bottom {
                    return Ok(());
                }
                let height = self.chart.bar_width.min(bottom - y);
                let style = bar.style.unwrap_or(self.colors.bar);
                let filled = ticks(bar.value, self.max, plot.width);
                let whole = filled / 8;
                let partial = !filled.is_multiple_of(8);
                visitor.bar(BarPlacement {
                    track: Rect::new(plot.x, y, plot.width, height),
                    direction: BarDirection::Horizontal,
                    value: bar.value,
                    max: self.max,
                    style,
                })?;
                let row = y.saturating_add((height - 1) / 2);
                if let Some(text) = bar.label {
                    visitor.text(TextPlacement {
                        x: self.area.x,
                        y: row,
                        width: label_width,
                        text,
                        style: self.colors.label,
                        align: TextAlign::Start,
                    })?;
                }
                if let Some(text) = self.chart.value_text(bar) {
                    let used = u16::try_from(text_display_width(&text)).unwrap_or(u16::MAX);
                    if whole >= used.saturating_add(1) {
                        visitor.text(TextPlacement {
                            x: plot.x,
                            y: row,
                            width: whole,
                            text: &text,
                            style: Style::new().fg(self.colors.on_bar).bg(style.fg),
                            align: TextAlign::Start,
                        })?;
                    } else {
                        let after = plot
                            .x
                            .saturating_add(whole)
                            .saturating_add(u16::from(partial))
                            .saturating_add(1);
                        visitor.text(TextPlacement {
                            x: after,
                            y: row,
                            width: right.saturating_sub(after),
                            text: &text,
                            style: self.colors.value,
                            align: TextAlign::Start,
                        })?;
                    }
                }
                y = y.saturating_add(self.chart.bar_width);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Ink {
    Cells,
    #[cfg(feature = "pixel-barchart")]
    Spaces,
}

impl Ink {
    fn glyph(self, glyph: &str) -> &str {
        match self {
            Self::Cells => glyph,
            #[cfg(feature = "pixel-barchart")]
            Self::Spaces => " ",
        }
    }
}

fn paint_bar_cells(bar: BarPlacement, cx: &mut RenderCx<'_>, ink: Ink) {
    let Rect {
        x,
        y,
        width,
        height,
    } = bar.track;
    let filled = ticks(
        bar.value,
        bar.max,
        match bar.direction {
            BarDirection::Vertical => height,
            BarDirection::Horizontal => width,
        },
    );
    let whole = filled / 8;
    let partial = usize::from(filled % 8);
    match bar.direction {
        BarDirection::Vertical => {
            let bottom = y.saturating_add(height);
            for row in 0..whole {
                for column in 0..width {
                    put_styled(
                        cx,
                        x.saturating_add(column),
                        bottom - 1 - row,
                        ink.glyph("█"),
                        bar.style,
                    );
                }
            }
            if partial != 0 {
                for column in 0..width {
                    put_styled(
                        cx,
                        x.saturating_add(column),
                        bottom - 1 - whole,
                        ink.glyph(EIGHTHS_UP[partial]),
                        bar.style,
                    );
                }
            }
            for row in whole + u16::from(partial != 0)..height {
                for column in 0..width {
                    put_styled(
                        cx,
                        x.saturating_add(column),
                        bottom - 1 - row,
                        " ",
                        bar.style,
                    );
                }
            }
        }
        BarDirection::Horizontal => {
            for row in 0..height {
                for column in 0..whole {
                    put_styled(
                        cx,
                        x.saturating_add(column),
                        y.saturating_add(row),
                        ink.glyph("█"),
                        bar.style,
                    );
                }
                if partial != 0 {
                    put_styled(
                        cx,
                        x.saturating_add(whole),
                        y.saturating_add(row),
                        ink.glyph(EIGHTHS_ACROSS[partial]),
                        bar.style,
                    );
                }
            }
        }
    }
}

fn visit_text<E>(
    text: TextPlacement<'_>,
    mut write: impl FnMut(u16, u16, char, Style, usize) -> Result<(), E>,
) -> Result<(), E> {
    let shown = fit(text.text, text.width);
    let used = u16::try_from(text_display_width(&shown)).unwrap_or(text.width);
    let mut column = text.x.saturating_add(match text.align {
        TextAlign::Start => 0,
        TextAlign::Center => (text.width - used) / 2,
    });
    for glyph in shown.chars() {
        let mut buffer = [0_u8; 4];
        let width = text_display_width(glyph.encode_utf8(&mut buffer));
        write(column, text.y, glyph, text.style, width)?;
        column = column.saturating_add(u16::try_from(width).unwrap_or(1));
    }
    Ok(())
}

struct CellBackend<'cx, 'frame> {
    cx: &'cx mut RenderCx<'frame>,
}

impl Visitor for CellBackend<'_, '_> {
    type Error = core::convert::Infallible;

    fn bar(&mut self, bar: BarPlacement) -> Result<(), Self::Error> {
        paint_bar_cells(bar, self.cx, Ink::Cells);
        Ok(())
    }

    fn text(&mut self, text: TextPlacement<'_>) -> Result<(), Self::Error> {
        visit_text(text, |x, y, glyph, style, _| {
            self.cx.put_char_patched(x, y, glyph, style);
            Ok(())
        })
    }
}

impl Widget for BarChart<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let area = clip_area_to_coordinates(area);
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.clear_area(area, self.background.unwrap_or_default());
        let layout = Layout::new(self, area, cx);
        #[cfg(feature = "pixel-barchart")]
        if self.pixels && pixel::render(&layout, cx) {
            return;
        }
        let result = layout.visit(&mut CellBackend { cx });
        match result {
            Ok(()) => (),
            Err(never) => match never {},
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, avail.height).clamp(avail)
    }
}
