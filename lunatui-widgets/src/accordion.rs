// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const INDENT: u16 = 2;
const OPEN: &str = "▾";
const CLOSED: &str = "▸";

/// One collapsible section of an [`Accordion`].
#[derive(Clone, Debug, PartialEq)]
pub struct AccordionSection {
    title: String,
    badge: Option<String>,
    rows: u16,
    open: bool,
    reveal: f64,
    disabled: bool,
}

impl AccordionSection {
    /// A collapsed section with this title whose content needs `rows` rows when fully open.
    #[must_use]
    pub fn new(title: impl Into<String>, rows: u16) -> Self {
        Self {
            title: title.into(),
            badge: None,
            rows,
            open: false,
            reveal: 0.0,
            disabled: false,
        }
    }

    /// Opens or closes the section: its caret turns and its body is fully shown or hidden.
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self.reveal = if open { 1.0 } else { 0.0 };
        self
    }

    /// How much of the body is shown, from `0.0` (none) to `1.0` (all of its rows), for animating
    /// an open or close. The caret follows [`AccordionSection::open`], not this fraction, and a
    /// NaN fraction counts as zero.
    #[must_use]
    pub fn reveal(mut self, reveal: f64) -> Self {
        self.reveal = if reveal.is_nan() {
            0.0
        } else {
            reveal.clamp(0.0, 1.0)
        };
        self
    }

    /// Dim text on the right of the header, such as a count.
    #[must_use]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Dims the section and takes it out of the tab order.
    #[must_use]
    pub const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    fn shown_rows(&self) -> u16 {
        let distance =
            |rows: u16| (f64::from(rows) / f64::from(self.rows.max(1)) - self.reveal).abs();
        (0..=self.rows)
            .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
            .unwrap_or(0)
    }
}

/// Where one section's header and body sit in an accordion layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionArea {
    /// Index of the section.
    pub index: usize,
    /// The one-row header.
    pub header: Rect,
    /// The cells for the caller's content, empty while the section is closed.
    pub body: Rect,
}

/// A stack of collapsible sections: a header per section with a caret, a title, and a badge, and a
/// body area under each open one for the caller's own content.
///
/// [`Accordion::layout`] gives the header and body rectangles of every section that fits, for
/// drawing content and for hit testing. A section's [`reveal`](AccordionSection::reveal) fraction
/// sets how many of its body rows show, so an app animating an open or close just raises or lowers
/// it each tick. [`Accordion::step`] moves focus through the headers in tab order.
#[derive(Clone, Debug, PartialEq)]
pub struct Accordion {
    sections: Vec<AccordionSection>,
    focus: Option<usize>,
    gap: u16,
    state: WidgetState,
    style: Option<Style>,
}

impl Accordion {
    /// An accordion of these sections.
    #[must_use]
    pub fn new(sections: impl IntoIterator<Item = AccordionSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            focus: None,
            gap: 0,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// The section whose header has input focus.
    #[must_use]
    pub const fn focus(mut self, index: Option<usize>) -> Self {
        self.focus = index;
        self
    }

    /// Blank rows between sections.
    #[must_use]
    pub const fn gap(mut self, rows: u16) -> Self {
        self.gap = rows;
        self
    }

    /// Interaction state of the whole accordion. A disabled one dims every header.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws every header in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Number of sections.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    /// Whether the accordion has no sections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// The header after (or before) `from`, wrapping around and skipping disabled sections. From
    /// `None`, forward gives the first enabled section and backward the last. `None` when no
    /// section can take focus.
    #[must_use]
    pub fn step(&self, from: Option<usize>, forward: bool) -> Option<usize> {
        let count = self.sections.len();
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
            .find(|index| !self.sections[*index].disabled)
    }

    /// Where each section's header and body sit when the accordion fills `area`. Sections whose
    /// header does not fit are left out, and a body is cut to the rows that remain.
    #[must_use]
    pub fn layout(&self, area: Rect) -> Vec<SectionArea> {
        let mut areas = Vec::new();
        let end = area.y.saturating_add(area.height);
        let mut y = area.y;
        for (index, section) in self.sections.iter().enumerate() {
            if area.width == 0 || y >= end {
                break;
            }
            let rows = section.shown_rows().min(end - y - 1);
            let indent = INDENT.min(area.width);
            areas.push(SectionArea {
                index,
                header: Rect::new(area.x, y, area.width, 1),
                body: Rect::new(area.x + indent, y + 1, area.width - indent, rows),
            });
            y = y
                .saturating_add(1)
                .saturating_add(rows)
                .saturating_add(self.gap);
        }
        areas
    }

    fn header_styles(&self, cx: &RenderCx<'_>, index: usize) -> (Style, Style) {
        let section = &self.sections[index];
        if let Some(style) = self.style {
            return (style, style);
        }
        let disabled = matches!(self.state, WidgetState::Disabled) || section.disabled;
        let focused = self.focus == Some(index) && !disabled;
        let text = Style::new()
            .fg(if focused {
                cx.token(Token::Accent)
            } else {
                cx.token(Token::TextPrimary)
            })
            .with_attrs(Attrs::new().set_dim(disabled).set_bold(focused));
        let badge = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .with_attrs(Attrs::new().set_dim(true));
        (text, badge)
    }
}

impl Widget for Accordion {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        for section_area in self.layout(area) {
            let section = &self.sections[section_area.index];
            let (text, badge) = self.header_styles(cx, section_area.index);
            let header = section_area.header;
            let caret = if section.open { OPEN } else { CLOSED };
            cx.put_str(header.x, header.y, caret, text);
            let title_at = header.x.saturating_add(2);
            let end = header.x.saturating_add(header.width);
            let badge_width = section.badge.as_ref().map_or(0, |badge| {
                u16::try_from(width(badge) + 1).unwrap_or(u16::MAX)
            });
            let room = end.saturating_sub(title_at);
            let (title_room, badge_width) = if room > badge_width {
                (room - badge_width, badge_width)
            } else {
                (room, 0)
            };
            cx.put_str(
                title_at,
                header.y,
                clip_display_prefix(&section.title, title_room),
                text,
            );
            if let (Some(label), true) = (&section.badge, badge_width > 0) {
                let label_width = badge_width - 1;
                cx.put_str(end - label_width, header.y, label, badge);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let areas = self.layout(Rect::new(0, 0, avail.width, avail.height));
        let bottom = areas
            .last()
            .map_or(0, |section| section.body.y + section.body.height);
        let width = if areas.is_empty() { 0 } else { avail.width };
        Size::new(width, bottom)
    }
}
