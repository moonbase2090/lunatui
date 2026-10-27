// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, text_display_width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

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
/// lengths use eighth blocks, so bar ends move smoothly. `bar_width` is the thickness of a bar
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
    bar_style: Option<Style>,
    label_style: Option<Style>,
    value_style: Option<Style>,
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
            bar_style: None,
            label_style: None,
            value_style: None,
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

fn put_clipped(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    put_styled(cx, x, y, &fit(text, width), style);
}

fn put_centered(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let shown = fit(text, width);
    let used = u16::try_from(text_display_width(&shown)).unwrap_or(width);
    put_styled(cx, x + (width - used) / 2, y, &shown, style);
}

impl BarChart<'_> {
    fn render_vertical(&self, area: Rect, cx: &mut RenderCx<'_>, max: u64) {
        let colors = self.colors(cx);
        let has_bar_labels = self
            .groups()
            .any(|(_, bars)| bars.iter().any(|bar| bar.label.is_some()));
        let has_group_labels = self.groups().any(|(label, _)| label.is_some());
        let plot = area
            .height
            .saturating_sub(u16::from(has_bar_labels) + u16::from(has_group_labels));
        let bottom = area.y + plot;
        let bar_label_y = bottom;
        let group_label_y = bottom + u16::from(has_bar_labels);
        let right = area.x + area.width;
        let mut x = area.x;
        for (group_index, (group_label, bars)) in self.groups().enumerate() {
            if group_index > 0 {
                x = x.saturating_add(self.group_gap);
            }
            let start = x;
            for (bar_index, bar) in bars.iter().enumerate() {
                if bar_index > 0 {
                    x = x.saturating_add(self.bar_gap);
                }
                if x >= right {
                    return;
                }
                let width = self.bar_width.min(right - x);
                let style = bar.style.unwrap_or(colors.bar);
                let filled = ticks(bar.value, max, plot);
                let whole = filled / 8;
                for row in 0..whole {
                    for column in 0..width {
                        put_styled(cx, x + column, bottom - 1 - row, "█", style);
                    }
                }
                let partial = usize::from(filled % 8);
                if partial != 0 {
                    let glyph = EIGHTHS_UP[partial];
                    for column in 0..width {
                        put_styled(cx, x + column, bottom - 1 - whole, glyph, style);
                    }
                }
                for row in whole + u16::from(partial != 0)..plot {
                    for column in 0..width {
                        put_styled(cx, x + column, bottom - 1 - row, " ", style);
                    }
                }
                if let Some(text) = self.value_text(bar)
                    && plot > 0
                {
                    let used = whole + u16::from(partial != 0);
                    let (text_style, y) = if used == 0 {
                        (colors.value, bottom - 1)
                    } else if used < plot {
                        (colors.value, bottom - 1 - used)
                    } else {
                        (Style::new().fg(colors.on_bar).bg(style.fg), bottom - plot)
                    };
                    put_centered(cx, x, y, width, &text, text_style);
                }
                if let Some(label) = bar.label {
                    put_centered(cx, x, bar_label_y, width, label, colors.label);
                }
                x = x.saturating_add(self.bar_width);
            }
            if let Some(label) = group_label {
                let span = x.min(right).saturating_sub(start);
                put_centered(cx, start, group_label_y, span, label, colors.label);
            }
        }
    }

    fn render_horizontal(&self, area: Rect, cx: &mut RenderCx<'_>, max: u64) {
        let colors = self.colors(cx);
        let label_width = self
            .groups()
            .flat_map(|(_, bars)| bars.iter())
            .filter_map(|bar| bar.label)
            .map(text_display_width)
            .max()
            .unwrap_or(0);
        let label_width = u16::try_from(label_width)
            .unwrap_or(u16::MAX)
            .min(area.width / 2);
        let track_x = area.x + label_width + u16::from(label_width > 0);
        let track = (area.x + area.width).saturating_sub(track_x);
        let bottom = area.y + area.height;
        let mut y = area.y;
        for (group_index, (group_label, bars)) in self.groups().enumerate() {
            if group_index > 0 {
                y = y.saturating_add(self.group_gap);
            }
            if let Some(label) = group_label {
                if y >= bottom {
                    return;
                }
                put_clipped(cx, area.x, y, area.width, label, colors.label);
                y += 1;
            }
            for (bar_index, bar) in bars.iter().enumerate() {
                if bar_index > 0 {
                    y = y.saturating_add(self.bar_gap);
                }
                if y >= bottom {
                    return;
                }
                let height = self.bar_width.min(bottom - y);
                let style = bar.style.unwrap_or(colors.bar);
                let filled = ticks(bar.value, max, track);
                let whole = filled / 8;
                let partial = usize::from(filled % 8);
                for row in 0..height {
                    for column in 0..whole {
                        put_styled(cx, track_x + column, y + row, "█", style);
                    }
                    if partial != 0 {
                        let glyph = EIGHTHS_ACROSS[partial];
                        put_styled(cx, track_x + whole, y + row, glyph, style);
                    }
                }
                if let Some(label) = bar.label {
                    let row = y + (height - 1) / 2;
                    put_clipped(cx, area.x, row, label_width, label, colors.label);
                }
                if let Some(text) = self.value_text(bar) {
                    let row = y + (height - 1) / 2;
                    let used = u16::try_from(text_display_width(&text)).unwrap_or(u16::MAX);
                    if whole >= used.saturating_add(1) {
                        let inside = Style::new().fg(colors.on_bar).bg(style.fg);
                        put_clipped(cx, track_x, row, whole, &text, inside);
                    } else {
                        let after = track_x + whole + u16::from(partial != 0) + 1;
                        let room = (area.x + area.width).saturating_sub(after);
                        put_clipped(cx, after, row, room, &text, colors.value);
                    }
                }
                y = y.saturating_add(self.bar_width);
            }
        }
    }
}

impl Widget for BarChart<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.clear_area(area);
        let max = self.max.unwrap_or_else(|| self.largest());
        match self.direction {
            BarDirection::Vertical => self.render_vertical(area, cx, max),
            BarDirection::Horizontal => self.render_horizontal(area, cx, max),
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, avail.height).clamp(avail)
    }
}
