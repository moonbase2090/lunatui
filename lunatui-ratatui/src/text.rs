// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::Style;
use ratatui::text::{Line, Span, Text};

/// One styled text run in lunatui form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledSpan {
    /// UTF-8 text.
    pub text: String,
    /// lunatui style for the run.
    pub style: Style,
}

/// One logical line built from styled spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledLine {
    /// Spans in left-to-right order.
    pub spans: Vec<StyledSpan>,
}

/// Multi-line styled text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledText {
    /// Lines in order.
    pub lines: Vec<StyledLine>,
}

impl From<&Span<'_>> for StyledSpan {
    fn from(span: &Span<'_>) -> Self {
        Self {
            text: span.content.to_string(),
            style: crate::style::style_from_ratatui(span.style),
        }
    }
}

impl From<Span<'_>> for StyledSpan {
    fn from(span: Span<'_>) -> Self {
        Self::from(&span)
    }
}

impl From<&Line<'_>> for StyledLine {
    fn from(line: &Line<'_>) -> Self {
        Self {
            spans: line.spans.iter().map(StyledSpan::from).collect(),
        }
    }
}

impl From<Line<'_>> for StyledLine {
    fn from(line: Line<'_>) -> Self {
        Self::from(&line)
    }
}

impl From<&Text<'_>> for StyledText {
    fn from(text: &Text<'_>) -> Self {
        Self {
            lines: text.lines.iter().map(StyledLine::from).collect(),
        }
    }
}

impl From<Text<'_>> for StyledText {
    fn from(text: Text<'_>) -> Self {
        Self::from(&text)
    }
}

impl From<StyledSpan> for Span<'static> {
    fn from(span: StyledSpan) -> Self {
        Span::styled(span.text, crate::style::style_to_ratatui(span.style))
    }
}

impl From<StyledLine> for Line<'static> {
    fn from(line: StyledLine) -> Self {
        Line::from(line.spans.into_iter().map(Span::from).collect::<Vec<_>>())
    }
}

impl From<StyledText> for Text<'static> {
    fn from(text: StyledText) -> Self {
        Text::from(text.lines.into_iter().map(Line::from).collect::<Vec<_>>())
    }
}
