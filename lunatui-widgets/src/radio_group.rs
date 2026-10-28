// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const GLYPH_WIDTH: u16 = 1;

/// Which way a [`RadioGroup`] lays its options out.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioFlow {
    /// One option per row, with `gap` blank rows between them.
    #[default]
    Vertical,
    /// Options side by side on one row, with `gap` blank cells between them.
    Horizontal,
}

/// A group of options of which at most one is selected: `◉ Selected`, `○ Other`.
///
/// The selected glyph is the accent color and the focused option is drawn with the focus ring
/// glyph and a bold label. A disabled group, or a disabled option, is dimmed.
/// [`RadioGroup::option_areas`] gives the cells each option covers for hit testing, and
/// [`RadioGroup::step`] moves the selection with the keyboard, wrapping around and skipping
/// disabled options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioGroup {
    options: Vec<String>,
    selected: Option<usize>,
    focused: Option<usize>,
    disabled: Vec<usize>,
    flow: RadioFlow,
    gap: u16,
    state: WidgetState,
    style: Option<Style>,
}

impl RadioGroup {
    /// A group with the given option labels and nothing selected.
    #[must_use]
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            focused: None,
            disabled: Vec::new(),
            flow: RadioFlow::Vertical,
            gap: 0,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// The selected option, if any. An index past the last option selects nothing.
    #[must_use]
    pub fn select(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// The option that has input focus, if any.
    #[must_use]
    pub fn focus(mut self, index: Option<usize>) -> Self {
        self.focused = index;
        self
    }

    /// Options that cannot be chosen and are drawn dimmed.
    #[must_use]
    pub fn disabled_options(mut self, indexes: impl IntoIterator<Item = usize>) -> Self {
        self.disabled = indexes.into_iter().collect();
        self
    }

    /// Lays the options out one per row or side by side.
    #[must_use]
    pub const fn flow(mut self, flow: RadioFlow) -> Self {
        self.flow = flow;
        self
    }

    /// Blank rows or cells between options.
    #[must_use]
    pub const fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }

    /// Interaction state of the whole group. A disabled group dims every option.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws every option in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Number of options.
    #[must_use]
    pub fn len(&self) -> usize {
        self.options.len()
    }

    /// Whether the group has no options.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    /// The option after (or before) `from`, wrapping around and skipping disabled options.
    /// Starting from `None`, forward gives the first enabled option and backward the last.
    /// `None` when no option can be chosen.
    #[must_use]
    pub fn step(&self, from: Option<usize>, forward: bool) -> Option<usize> {
        let count = self.options.len();
        if count == 0 {
            return None;
        }
        let start = match (from.filter(|index| *index < count), forward) {
            (Some(index), true) => index + 1,
            (Some(index), false) => index + count - 1,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        (0..count)
            .map(|offset| {
                if forward {
                    (start + offset) % count
                } else {
                    (start + count - offset) % count
                }
            })
            .find(|index| !self.disabled.contains(index))
    }

    /// Where each option sits when rendered into `area`, as `(index, rect)` pairs in order, for
    /// hit testing. A rect covers the glyph, one blank cell, and the label. Options that do not
    /// fit are left out and the last one that is cut off by the edge covers only the cells that
    /// show.
    #[must_use]
    pub fn option_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        let mut areas = Vec::new();
        let end_x = area.x.saturating_add(area.width);
        let end_y = area.y.saturating_add(area.height);
        let (mut x, mut y) = (area.x, area.y);
        for (index, label) in self.options.iter().enumerate() {
            if x >= end_x || y >= end_y {
                break;
            }
            let label_width = u16::try_from(width(label)).unwrap_or(u16::MAX);
            let wanted = GLYPH_WIDTH.saturating_add(1).saturating_add(label_width);
            let cells = wanted.min(end_x - x);
            areas.push((index, Rect::new(x, y, cells, 1)));
            match self.flow {
                RadioFlow::Vertical => y = y.saturating_add(1).saturating_add(self.gap),
                RadioFlow::Horizontal => {
                    x = x.saturating_add(wanted).saturating_add(self.gap);
                }
            }
        }
        areas
    }

    fn option_style(&self, cx: &RenderCx<'_>, index: usize) -> (Style, Style) {
        if let Some(style) = self.style {
            return (style, style);
        }
        let dim = matches!(self.state, WidgetState::Disabled) || self.disabled.contains(&index);
        let focused = self.focused == Some(index) && !dim;
        let selected = self.selected == Some(index);
        let glyph = if focused {
            cx.token(Token::FocusRing)
        } else if selected {
            cx.token(Token::Accent)
        } else {
            cx.token(Token::TextPrimary)
        };
        let attrs = Attrs::new().set_dim(dim).set_bold(focused);
        (
            Style::new().fg(glyph).with_attrs(attrs),
            Style::new()
                .fg(cx.token(Token::TextPrimary))
                .with_attrs(attrs),
        )
    }
}

impl Widget for RadioGroup {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        for (index, rect) in self.option_areas(area) {
            let (glyph_style, label_style) = self.option_style(cx, index);
            let glyph = if self.selected == Some(index) {
                "◉"
            } else {
                "○"
            };
            cx.put_str(rect.x, rect.y, glyph, glyph_style);
            let room = rect.width.saturating_sub(GLYPH_WIDTH + 1);
            cx.put_str(
                rect.x + GLYPH_WIDTH + 1,
                rect.y,
                clip_display_prefix(&self.options[index], room),
                label_style,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let areas = self.option_areas(Rect::new(0, 0, avail.width, avail.height));
        let right = areas
            .iter()
            .map(|(_, rect)| rect.x + rect.width)
            .max()
            .unwrap_or(0);
        let bottom = areas.last().map_or(0, |(_, rect)| rect.y + 1);
        Size::new(right, bottom)
    }
}
