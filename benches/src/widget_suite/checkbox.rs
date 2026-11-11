// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use lunatui_choice::{CheckState, Checkbox};
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::Role;
use lunatui_widgets::{Block, BorderType, Paragraph, RenderCx, Widget, WidgetState};
use ratatui::widgets::{Block as RatBlock, BorderType as RatBorder, Paragraph as RatParagraph};
use ratatui::{
    buffer::Buffer, layout::Rect as RatRect, style::Modifier, widgets::Widget as RatWidget,
};

const LABELS: [&str; 6] = [
    "Deploys",
    "Failed checks",
    "Review requests",
    "Security alerts",
    "Weekly summary",
    "All events",
];
const SUMMARY: [&str; 5] = [
    "No live alerts selected",
    "1 live alert selected",
    "2 live alerts selected",
    "3 live alerts selected",
    "All live alerts selected",
];

fn frame(tick: u32) -> ([CheckState; 6], usize, &'static str) {
    let step = tick / 12;
    let mask = match (step / 5) % 4 {
        0 => 0,
        1 => 1,
        2 => 3,
        _ => 15,
    };
    let mut states = [CheckState::Unchecked; 6];
    let mut count = 0;
    for (index, state) in states.iter_mut().take(4).enumerate() {
        let bit = u32::try_from(index).unwrap_or(0);
        if mask & (1 << bit) != 0 {
            *state = CheckState::Checked;
            count += 1;
        }
    }
    states[4] = CheckState::Checked;
    states[5] = match count {
        0 => CheckState::Unchecked,
        4 => CheckState::Checked,
        _ => CheckState::Mixed,
    };
    (
        states,
        usize::try_from(step % 5).unwrap_or(0),
        SUMMARY[count],
    )
}

fn state(index: usize, focus: usize) -> WidgetState {
    if index == 4 {
        WidgetState::Disabled
    } else if index == if focus == 4 { 5 } else { focus } {
        WidgetState::Focused
    } else {
        WidgetState::Normal
    }
}

fn panel(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4).min(60);
    let height = area.height.saturating_sub(2).min(20);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn rat_rect(area: Rect) -> RatRect {
    RatRect::new(area.x, area.y, area.width, area.height)
}

fn rat_style(style: Style) -> ratatui::style::Style {
    let mut result = ratatui::style::Style::new()
        .fg(super::rat_color(style.fg))
        .bg(super::rat_color(style.bg));
    if style.attrs.bold() {
        result = result.add_modifier(Modifier::BOLD);
    }
    if style.attrs.dim() {
        result = result.add_modifier(Modifier::DIM);
    }
    result
}

fn ink(theme: &Theme, fg: Role, bg: Role) -> Style {
    Style::new()
        .fg(theme.resolve(fg.token()))
        .bg(theme.resolve(bg.token()))
}

/// Renders the native checkbox scene from the fixture's data and supplied theme.
pub fn native(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let (states, focus, summary) = frame(tick);
    let page = ink(theme, Role::Foreground, Role::Background);
    let card = ink(theme, Role::Foreground, Role::Card);
    let dim = ink(theme, Role::Muted, Role::Card);
    let accent = ink(theme, Role::Primary, Role::Card);
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(page).render(area, &mut cx);
    let panel = panel(area);
    if panel.width < 8 || panel.height < 20 {
        return;
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .style(card)
        .border_style(dim)
        .render(panel, &mut cx);
    let text = Rect::new(panel.x + 2, panel.y + 1, panel.width - 4, 1);
    Block::new().style(accent).render(text, &mut cx);
    Paragraph::new("Notification preferences")
        .style(accent)
        .render(text, &mut cx);
    Block::new()
        .style(dim)
        .render(Rect::new(text.x, text.y + 1, text.width, 1), &mut cx);
    Paragraph::new("Space toggles a choice; Tab moves focus")
        .style(dim)
        .render(Rect::new(text.x, text.y + 1, text.width, 1), &mut cx);
    for (index, label) in LABELS.iter().enumerate() {
        let y = panel.y + 4 + 2 * u16::try_from(index).unwrap_or(0);
        Checkbox::new(label)
            .checked(states[index])
            .state(state(index, focus))
            .on(Role::Card)
            .render(Rect::new(text.x, y, text.width, 1), &mut cx);
    }
    Block::new()
        .style(dim)
        .render(Rect::new(text.x, panel.y + 17, text.width, 1), &mut cx);
    Paragraph::new(summary)
        .style(dim)
        .render(Rect::new(text.x, panel.y + 17, text.width, 1), &mut cx);
}

/// Renders the same scene with tui-checkbox, including native-equivalent glyphs and styles.
pub fn baseline_themed(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let (states, focus, summary) = frame(tick);
    let page = ink(theme, Role::Foreground, Role::Background);
    let card = ink(theme, Role::Foreground, Role::Card);
    let dim = ink(theme, Role::Muted, Role::Card);
    let accent = ink(theme, Role::Primary, Role::Card);
    RatBlock::new().style(rat_style(page)).render(area, buffer);
    let panel = panel(Rect::new(area.x, area.y, area.width, area.height));
    if panel.width < 8 || panel.height < 20 {
        return;
    }
    RatBlock::bordered()
        .border_type(RatBorder::Rounded)
        .style(rat_style(card))
        .border_style(rat_style(dim))
        .render(rat_rect(panel), buffer);
    let text = Rect::new(panel.x + 2, panel.y + 1, panel.width - 4, 1);
    RatParagraph::new("Notification preferences")
        .style(rat_style(accent))
        .render(rat_rect(text), buffer);
    RatParagraph::new("Space toggles a choice; Tab moves focus")
        .style(rat_style(dim))
        .render(RatRect::new(text.x, text.y + 1, text.width, 1), buffer);
    for (index, label) in LABELS.iter().enumerate() {
        let active = states[index] != CheckState::Unchecked;
        let mut label_style = card;
        let mut mark = ink(
            theme,
            if active { Role::Primary } else { Role::Border },
            Role::Card,
        );
        match state(index, focus) {
            WidgetState::Focused => {
                mark = ink(theme, Role::Ring, Role::Card).with_attrs(Attrs::new().set_bold(true));
                label_style = card.with_attrs(Attrs::new().set_bold(true));
            }
            WidgetState::Disabled => {
                mark = mark.with_attrs(Attrs::new().set_dim(true));
                label_style = dim.with_attrs(Attrs::new().set_dim(true));
            }
            _ if active => mark = mark.with_attrs(Attrs::new().set_bold(true)),
            _ => {}
        }
        let row = RatRect::new(
            text.x,
            panel.y + 4 + 2 * u16::try_from(index).unwrap_or(0),
            text.width,
            1,
        );
        RatBlock::new()
            .style(rat_style(label_style))
            .render(row, buffer);
        tui_checkbox::Checkbox::new(*label, active)
            .checked_symbol(if states[index] == CheckState::Mixed {
                "[-]"
            } else {
                "[x]"
            })
            .unchecked_symbol("[ ]")
            .style(rat_style(label_style))
            .checkbox_style(rat_style(mark))
            .label_style(rat_style(label_style))
            .render(row, buffer);
    }
    RatParagraph::new(summary)
        .style(rat_style(dim))
        .render(RatRect::new(text.x, panel.y + 17, text.width, 1), buffer);
}

/// Draws the fair-suite baseline with a reused dark theme, matching the native runner.
pub fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    static THEME: OnceLock<Theme> = OnceLock::new();
    baseline_themed(
        buffer,
        area,
        THEME.get_or_init(|| Theme::builtin(Variant::Dark)),
        tick,
    );
}
