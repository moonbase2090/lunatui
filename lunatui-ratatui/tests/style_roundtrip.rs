// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_ratatui::{
    StyledLine, StyledSpan, StyledText, color_from_ratatui, color_to_ratatui, style_from_ratatui,
    style_to_ratatui,
};
use ratatui::layout::Alignment;
use ratatui::style::{Color as RColor, Modifier, Style as RStyle};
use ratatui::text::{Line, Span, Text};

#[test]
fn color_roundtrip_common_subset() {
    let samples = [
        RColor::Reset,
        RColor::Red,
        RColor::LightBlue,
        RColor::Indexed(196),
        RColor::Rgb(1, 2, 3),
    ];
    for sample in samples {
        let back = color_to_ratatui(color_from_ratatui(sample));
        assert_eq!(sample, back, "color roundtrip {sample:?}");
    }
}

#[test]
fn style_roundtrip_common_subset() {
    let sample = RStyle::default()
        .fg(RColor::Green)
        .bg(RColor::Rgb(10, 20, 30))
        .add_modifier(Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED);
    let lun = style_from_ratatui(sample);
    let back = style_to_ratatui(lun);
    assert_eq!(sample, back);
}

#[test]
fn text_roundtrip_spans() {
    let line = Line::from(vec![
        Span::styled("a", RStyle::default().fg(RColor::Yellow)),
        Span::raw("b"),
    ]);
    let converted = StyledLine::from(&line);
    let back = Line::from(converted);
    assert_eq!(line, back);

    let text = Text::from(vec![
        Line::from("one"),
        Line::from(vec![Span::styled(
            "two",
            RStyle::default().add_modifier(Modifier::DIM),
        )]),
    ]);
    let styled = StyledText::from(&text);
    let restored = Text::from(styled);
    assert_eq!(text, restored);
}

#[test]
fn line_and_text_metadata_roundtrip() {
    let line = Line::from(vec![Span::raw("x")])
        .style(RStyle::default().fg(RColor::Magenta))
        .alignment(Alignment::Center);
    assert_eq!(Line::from(StyledLine::from(&line)), line);

    let text = Text::from(vec![Line::from("a"), Line::from("b")])
        .style(RStyle::default().add_modifier(Modifier::DIM))
        .alignment(Alignment::Right);
    assert_eq!(Text::from(StyledText::from(&text)), text);
}

#[test]
fn styled_span_preserves_content() {
    let span = Span::styled("xy", RStyle::default().fg(RColor::Cyan));
    let got = StyledSpan::from(&span);
    assert_eq!(got.text, "xy");
    assert_eq!(got.style, style_from_ratatui(span.style));
}
