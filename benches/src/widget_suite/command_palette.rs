// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{CommandPalette, PaletteEntry, RenderCx, Widget};
use ratatui::{
    buffer::Buffer,
    layout::Rect as RatRect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Widget as RatWidget},
};

const ENTRIES: [PaletteEntry<'static>; 5] = [
    PaletteEntry::new("Open file"),
    PaletteEntry::new("Save file"),
    PaletteEntry::new("Close tab"),
    PaletteEntry::new("Split pane right"),
    PaletteEntry::new("Toggle sidebar"),
];

fn fixture(tick: u32) -> (&'static str, &'static [(usize, &'static [usize])]) {
    match tick % 3 {
        0 => ("", &[(0, &[]), (1, &[]), (2, &[]), (3, &[]), (4, &[])]),
        1 => ("of", &[(0, &[0, 5])]),
        _ => ("sv", &[(1, &[0, 2])]),
    }
}

pub(super) fn native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let (query, _) = fixture(tick);
    CommandPalette::new(query, query.len(), &ENTRIES)
        .selected(Some(0))
        .width(50)
        .max_rows(3)
        .backdrop(0.0)
        .render(area, &mut RenderCx::new(grid, clusters, theme));
}

pub(super) fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    let (query, hits) = fixture(tick);
    let theme = Theme::builtin(Variant::Dark);
    let color = |token| super::rat_color(theme.resolve(token));
    let surface = color(Token::Surface1);
    let accent = color(Token::Accent);
    let base = Style::new().fg(color(Token::TextPrimary)).bg(surface);
    let dim = base.add_modifier(Modifier::DIM);
    let active = Style::new()
        .fg(surface)
        .bg(accent)
        .add_modifier(Modifier::BOLD);
    let mark = Style::new()
        .fg(accent)
        .bg(surface)
        .remove_modifier(Modifier::BOLD);
    let rows = u16::try_from(hits.len().min(3)).unwrap_or(0);
    let panel = RatRect::new(
        area.x + (area.width - 50) / 2,
        area.y + (area.height - rows - 3) / 2,
        50,
        rows + 3,
    );
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(base)
        .border_style(Style::new().fg(accent).bg(surface))
        .render(panel, buffer);
    let inner = RatRect::new(panel.x + 1, panel.y + 1, 48, rows + 1);
    let shown = if query.is_empty() {
        "Type a command"
    } else {
        query
    };
    let count = format!(
        "{} command{}",
        hits.len(),
        if hits.len() == 1 { "" } else { "s" }
    );
    let prompt = Style::new()
        .fg(accent)
        .bg(surface)
        .add_modifier(Modifier::BOLD);
    Paragraph::new(Line::from(vec![
        Span::styled("› ", prompt),
        Span::styled(shown, if query.is_empty() { dim } else { base }),
    ]))
    .style(base)
    .render(RatRect::new(inner.x, inner.y, 48, 1), buffer);
    let count_x = inner.x + 48 - u16::try_from(count.len() + 1).unwrap_or(0);
    Paragraph::new(count).style(dim).render(
        RatRect::new(count_x, inner.y, 48 - (count_x - inner.x), 1),
        buffer,
    );
    buffer[(inner.x + 47, inner.y)].set_style(base.remove_modifier(Modifier::DIM));
    let caret_x = inner.x + 2 + u16::try_from(query.len()).unwrap_or(0);
    buffer[(caret_x, inner.y)].set_style(
        Style::new()
            .fg(surface)
            .bg(color(Token::FocusRing))
            .remove_modifier(Modifier::DIM),
    );
    for (row, (index, marks)) in hits.iter().take(3).enumerate() {
        let y = inner.y + 1 + u16::try_from(row).unwrap_or(0);
        let style = if row == 0 { active } else { base };
        Paragraph::new(" ".repeat(48))
            .style(style)
            .render(RatRect::new(inner.x, y, 48, 1), buffer);
        let title = ENTRIES[*index].title();
        for (at, ch) in title.chars().enumerate() {
            let style = if marks.contains(&at) {
                if row == 0 { mark } else { prompt }
            } else {
                style
            };
            buffer[(inner.x + 1 + u16::try_from(at).unwrap_or(0), y)]
                .set_char(ch)
                .set_style(style);
        }
    }
}
