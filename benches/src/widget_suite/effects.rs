// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::sync::OnceLock;
use std::time::Duration;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, oklab};
use lunatui_runtime::Ease;
use lunatui_runtime::effect::{Effect, Palette};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, BorderType, Paragraph, RenderCx, Widget};
use ratatui::widgets::{Block as RatBlock, BorderType as RatBorder, Paragraph as RatParagraph};
use ratatui::{
    buffer::Buffer, layout::Rect as RatRect, style::Style as RatStyle, widgets::Widget as RatWidget,
};

use super::rat_color;

/// One tick is 10 ms; the scripted run is 240 ticks.
const TICK: Duration = Duration::from_millis(10);
const FADE: Duration = Duration::from_millis(600);
const HOLD: Duration = Duration::from_millis(1200);

const ROWS: [(&str, Token); 6] = [
    ("build        passed in 41 s", Token::Ok),
    ("unit tests   1284 passed", Token::Ok),
    ("lint         2 warnings", Token::Warn),
    ("integration  3 failed", Token::Fail),
    ("deploy       waiting for approval", Token::Accent),
    ("report       written to target/report", Token::TextMuted),
];

/// Fade in for 600 ms, hold for 1.2 s, fade out for 600 ms. Each fade is a
/// native `Effect` that blends toward the page color.
fn timeline() -> &'static Effect {
    static EFFECT: OnceLock<Effect> = OnceLock::new();
    EFFECT.get_or_init(|| {
        let page = Color::Token(Token::Surface0.id());
        Effect::sequence(vec![
            Effect::fade(page, FADE, Ease::Linear).reversed(),
            Effect::fade(page, FADE, Ease::Linear).delayed(HOLD),
        ])
    })
}

fn panel(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4).min(48);
    let height = area.height.saturating_sub(2).min(12);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn style(theme: &Theme, fg: Token, bg: Token) -> Style {
    Style::new().fg(theme.resolve(fg)).bg(theme.resolve(bg))
}

fn rat_style(style: Style) -> RatStyle {
    RatStyle::new()
        .fg(rat_color(style.fg))
        .bg(rat_color(style.bg))
}

fn rgb(theme: &Theme, token: Token) -> [u8; 3] {
    match theme.resolve(token) {
        Color::Rgb(red, green, blue) => [red, green, blue],
        _ => [0, 0, 0],
    }
}

fn elapsed(tick: u32) -> Duration {
    TICK * tick
}

/// Draws the scene with the native widgets, then applies the native effect.
pub(super) fn native(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let page = style(theme, Token::TextPrimary, Token::Surface0);
    let card = style(theme, Token::TextPrimary, Token::Surface1);
    let border = style(theme, Token::Border, Token::Surface1);
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(page).render(area, &mut cx);
    let panel = panel(area);
    if panel.width >= 8 && panel.height >= 10 {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .style(card)
            .border_style(border)
            .render(panel, &mut cx);
        let text = Rect::new(panel.x + 2, panel.y + 1, panel.width - 4, 1);
        let accent = style(theme, Token::Accent, Token::Surface1);
        Block::new().style(accent).render(text, &mut cx);
        Paragraph::new("Release pipeline")
            .style(accent)
            .render(text, &mut cx);
        for (index, (line, token)) in ROWS.iter().enumerate() {
            let row = Rect::new(
                text.x,
                panel.y + 3 + u16::try_from(index).unwrap_or(0),
                text.width,
                1,
            );
            let ink = style(theme, *token, Token::Surface1);
            Block::new().style(ink).render(row, &mut cx);
            Paragraph::new(*line).style(ink).render(row, &mut cx);
        }
    }
    let resolve = |color: Color| match color {
        Color::Token(id) => match theme.resolve_id(id) {
            Color::Rgb(red, green, blue) => Some([red, green, blue]),
            _ => None,
        },
        _ => None,
    };
    let palette = Palette::new(rgb(theme, Token::TextPrimary), rgb(theme, Token::Surface0))
        .with_resolver(&resolve);
    timeline().render(elapsed(tick), grid, area, &palette);
}

/// How far the page is faded at `tick`, computed with the same arithmetic as
/// the native effect so both sides blend by an identical amount.
fn strength(tick: u32) -> f32 {
    let now = elapsed(tick);
    let ratio = |since: Duration| now.saturating_sub(since).as_secs_f32() / FADE.as_secs_f32();
    if now < FADE {
        1.0 - ratio(Duration::ZERO).clamp(0.0, 1.0)
    } else {
        ratio(FADE + HOLD).clamp(0.0, 1.0)
    }
}

fn faded(
    color: ratatui::style::Color,
    fallback: [u8; 3],
    target: [u8; 3],
    amount: f32,
) -> ratatui::style::Color {
    let from = match color {
        ratatui::style::Color::Rgb(red, green, blue) => [red, green, blue],
        _ => fallback,
    };
    let mixed = oklab::lerp_rgb(from, target, amount);
    ratatui::style::Color::Rgb(mixed[0], mixed[1], mixed[2])
}

/// Draws the same scene with ratatui's own widgets, then fades it with a small
/// hand-written loop that blends each cell's foreground and background toward
/// the page color. ratatui has no effect system and tachyonfx is not used.
pub(super) fn baseline(buffer: &mut Buffer, area: RatRect, tick: u32) {
    static THEME: OnceLock<Theme> = OnceLock::new();
    let theme = THEME.get_or_init(|| Theme::builtin(Variant::Dark));
    let page = style(theme, Token::TextPrimary, Token::Surface0);
    let card = style(theme, Token::TextPrimary, Token::Surface1);
    let border = style(theme, Token::Border, Token::Surface1);
    RatBlock::new().style(rat_style(page)).render(area, buffer);
    let bounds = panel(Rect::new(area.x, area.y, area.width, area.height));
    if bounds.width >= 8 && bounds.height >= 10 {
        RatBlock::bordered()
            .border_type(RatBorder::Rounded)
            .style(rat_style(card))
            .border_style(rat_style(border))
            .render(
                RatRect::new(bounds.x, bounds.y, bounds.width, bounds.height),
                buffer,
            );
        let text = RatRect::new(bounds.x + 2, bounds.y + 1, bounds.width - 4, 1);
        let accent = style(theme, Token::Accent, Token::Surface1);
        RatParagraph::new("Release pipeline")
            .style(rat_style(accent))
            .render(text, buffer);
        for (index, (line, token)) in ROWS.iter().enumerate() {
            let row = RatRect::new(
                text.x,
                bounds.y + 3 + u16::try_from(index).unwrap_or(0),
                text.width,
                1,
            );
            RatParagraph::new(*line)
                .style(rat_style(style(theme, *token, Token::Surface1)))
                .render(row, buffer);
        }
    }
    let amount = strength(tick);
    if amount <= 0.0 {
        return;
    }
    let target = rgb(theme, Token::Surface0);
    let fg = rgb(theme, Token::TextPrimary);
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let cell = &mut buffer[(x, y)];
            cell.fg = faded(cell.fg, fg, target, amount);
            cell.bg = faded(cell.bg, target, target, amount);
        }
    }
}
