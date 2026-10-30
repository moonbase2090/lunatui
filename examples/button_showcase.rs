// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Button, ButtonState, ButtonVariant, LineGauge, Paragraph, RenderCx,
    StatusBar, Widget, WidgetState,
};

const RUN: usize = 0;
const DEPLOY: usize = 1;
const ROLL_BACK: usize = 2;
const CANCEL: usize = 3;
const LABELS: [&str; 4] = ["Run checks", "Deploy to prod", "Roll back", "Cancel"];

const CHANGES: [&str; 6] = [
    "Passkey sign-in on web and mobile",
    "Search ranks recent visits first",
    "Cache layer moved to v3",
    "Vary header on compressed responses",
    "First-day checklist rewritten",
    "Image pipeline crates updated",
];

const CHECKS: [(&str, u32, u32); 4] = [
    ("build", 21, 50),
    ("unit tests", 30, 70),
    ("e2e tests", 40, 80),
    ("security audit", 50, 60),
];

const CHECKS_DONE: u32 = 120;
const DOWN: u32 = 150;
const DEPLOY_START: u32 = 162;
const DEPLOY_SPAN: u32 = 54;

const HISTORY: [(&str, &str); 6] = [
    ("Mon 16:02", "2.3.2 deployed to prod by Ana Ruiz"),
    ("Mon 15:48", "2.3.2 checks passed in 3m 51s"),
    ("Fri 11:20", "2.3.1 rolled back after a cache miss storm"),
    ("Fri 11:04", "2.3.1 deployed to prod by Kofi Mensah"),
    ("Thu 17:31", "2.3.0 deployed to prod by Lee Park"),
    ("Thu 17:12", "2.3.0 checks passed in 4m 08s"),
];

const LIVE: [(u32, &str); 4] = [
    (21, "2.4.0 checks started by Mia Chen"),
    (CHECKS_DONE, "2.4.0 checks passed in 3m 12s"),
    (DEPLOY_START, "2.4.0 rollout to prod started"),
    (DEPLOY_START + DEPLOY_SPAN, "2.4.0 is live on prod"),
];

#[derive(Clone, Copy)]
enum Step {
    Key(u32, &'static str, Option<usize>),
    Press(u32),
    Release(u32),
    Point(u32, usize),
    Down(u32),
    Up(u32),
}

const STEPS: [Step; 9] = [
    Step::Press(9),
    Step::Release(21),
    Step::Key(45, "Tab", Some(ROLL_BACK)),
    Step::Key(65, "Tab", Some(CANCEL)),
    Step::Key(85, "Tab", Some(RUN)),
    Step::Point(DOWN, DEPLOY),
    Step::Down(DOWN),
    Step::Up(DEPLOY_START),
    Step::Key(225, "Tab", Some(ROLL_BACK)),
];

struct App {
    focus: usize,
    held: ButtonState,
    key: &'static str,
    note: &'static str,
    pointer: Option<(usize, u32)>,
}

fn replay(tick: u32) -> App {
    let mut app = App {
        focus: RUN,
        held: ButtonState::new(),
        key: "",
        note: "Release 2.4.0 is ready to check",
        pointer: None,
    };
    for step in STEPS {
        match step {
            Step::Key(at, key, focus) if tick >= at => {
                app.key = key;
                if let Some(focus) = focus {
                    app.focus = focus;
                }
                if tick < CHECKS_DONE {
                    app.note = "Tab skips Deploy until the checks pass";
                }
            }
            Step::Press(at) if tick >= at => {
                app.key = "Enter";
                app.held.press();
            }
            Step::Release(at) if tick >= at && app.held.release() => {
                app.note = "Running 4 checks";
            }
            Step::Point(at, button) if tick + 30 >= at => app.pointer = Some((button, at)),
            Step::Down(at) if tick >= at => {
                app.key = "click";
                app.held.press();
                app.focus = DEPLOY;
            }
            Step::Up(at) if tick >= at && app.held.release() => {
                app.note = "Rolling 2.4.0 out to prod";
            }
            _ => {}
        }
    }
    if (CHECKS_DONE..150).contains(&tick) {
        app.note = "All checks passed; Deploy is enabled";
    }
    if tick >= DEPLOY_START + DEPLOY_SPAN {
        app.note = "2.4.0 is live on prod";
    }
    app
}

fn slots(bar: Rect) -> [Rect; 4] {
    let widths = [16_u16, 20, 15, 12];
    let mut x = bar.x + 2;
    widths.map(|width| {
        let slot = Rect::new(x, bar.y, width, 3);
        x += width + 2;
        slot
    })
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let text = theme.resolve(Token::TextPrimary);
    let fill = Style::new().fg(text).bg(theme.resolve(Token::Surface0));
    let raised = Style::new().fg(text).bg(theme.resolve(Token::Surface1));
    let accent = theme.resolve(Token::Accent);
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(fill).render(area, &mut cx);
    if area.width < 100 || area.height < 30 {
        return;
    }
    let app = replay(tick);
    let seconds = 12 + tick / 30;
    Paragraph::new(format!(
        " acme/web · release 2.4.0{:>width$}",
        format!("09:41:{seconds:02} "),
        width = usize::from(area.width) - 25
    ))
    .style(raised.fg(accent).with_attrs(Attrs::new().set_bold(true)))
    .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let top = area.y + 2;
    let left = Rect::new(area.x + 1, top, 52, 13);
    let right = Rect::new(
        left.x + left.width + 1,
        top,
        area.width - left.width - 3,
        left.height,
    );
    changes(&mut cx, left, fill, accent);
    checks(&mut cx, right, fill, theme, tick);
    let bar = Rect::new(area.x + 1, area.y + area.height - 6, area.width - 2, 5);
    history(
        &mut cx,
        Rect::new(
            area.x + 1,
            top + left.height + 1,
            area.width - 2,
            bar.y - top - left.height - 2,
        ),
        fill,
        accent,
        tick,
    );
    actions(&mut cx, &app, bar, fill, theme, tick);
    let dim = raised.with_attrs(Attrs::new().set_dim(true));
    StatusBar::new("")
        .style(raised)
        .left([
            (
                if app.key.is_empty() {
                    String::new()
                } else {
                    format!(" key {} ", app.key)
                },
                raised.fg(theme.resolve(Token::FocusRing)),
            ),
            (format!(" {} ", app.note), raised.fg(accent)),
        ])
        .right([(String::from(" Tab next  Enter press  click press "), dim)])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
    if let Some((button, arrive)) = app.pointer.filter(|_| tick < DEPLOY_START + 24) {
        let slot = slots(Rect::new(bar.x + 1, bar.y + 1, bar.width - 2, 3))[button];
        let target = (slot.x + slot.width - 3, slot.y + 1);
        let start = (area.x + area.width - 24, area.y + area.height - 12);
        let spent = (tick + 30).saturating_sub(arrive).min(30);
        let along = |from: u16, to: u16| {
            let from = i32::from(from);
            let to = i32::from(to);
            let at = from + (to - from) * i32::try_from(spent).unwrap_or(30) / 30;
            u16::try_from(at).unwrap_or(0)
        };
        let at = (along(start.0, target.0), along(start.1, target.1));
        let pressed = app.held.is_pressed();
        let under = if spent < 30 {
            fill.bg
        } else if pressed {
            theme.resolve(Token::Surface0)
        } else if tick >= DOWN {
            theme.resolve(Token::FocusRing)
        } else {
            theme.resolve(Token::Accent)
        };
        let ink = if spent >= 30 && !pressed {
            theme.resolve(Token::Surface0)
        } else {
            text
        };
        Paragraph::new(if pressed { "●" } else { "↖" })
            .style(
                Style::new()
                    .fg(ink)
                    .bg(under)
                    .with_attrs(Attrs::new().set_bold(true)),
            )
            .render(Rect::new(at.0, at.1, 1, 1), &mut cx);
    }
}

fn changes(cx: &mut RenderCx<'_>, pane: Rect, fill: Style, accent: Color) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Release 2.4.0 ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    Paragraph::new("38 commits from 9 people since 2.3.2")
        .style(dim)
        .render(Rect::new(inner.x + 1, inner.y + 1, inner.width - 2, 1), cx);
    for (row, change) in (3_u16..).zip(CHANGES) {
        Paragraph::new(format!("• {change}")).style(fill).render(
            Rect::new(inner.x + 1, inner.y + row, inner.width - 2, 1),
            cx,
        );
    }
}

fn history(cx: &mut RenderCx<'_>, pane: Rect, fill: Style, accent: Color, tick: u32) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Deploy history ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let live: Vec<_> = LIVE.iter().filter(|(at, _)| tick >= *at).rev().collect();
    let newest = live.first().map(|(at, _)| *at);
    let rows = live
        .iter()
        .map(|(at, line)| (format!("09:41:{:02}", 12 + at / 30), *line, Some(*at)))
        .chain(
            HISTORY
                .iter()
                .map(|(when, line)| ((*when).to_string(), *line, None)),
        );
    for (row, (when, line, at)) in (0_u16..).zip(rows) {
        if row >= inner.height {
            break;
        }
        let style = if at.is_some() && at == newest {
            fill.fg(accent)
        } else {
            fill
        };
        Paragraph::new(when)
            .style(dim)
            .render(Rect::new(inner.x + 1, inner.y + row, 10, 1), cx);
        Paragraph::new(line).style(style).render(
            Rect::new(inner.x + 12, inner.y + row, inner.width - 13, 1),
            cx,
        );
    }
}

fn checks(cx: &mut RenderCx<'_>, pane: Rect, fill: Style, theme: &Theme, tick: u32) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Checks ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let rows = CHECKS
        .iter()
        .map(|(name, start, span)| (*name, *start, *span))
        .chain([("rollout to prod", DEPLOY_START, DEPLOY_SPAN)]);
    for (index, (name, start, span)) in (0_u16..).zip(rows) {
        let y = inner.y + 1 + index * 2;
        let done = tick.saturating_sub(start).min(span);
        let started = tick >= start;
        let percent = done * 100 / span;
        let tone = if done == span {
            theme.resolve(Token::Ok)
        } else if started {
            theme.resolve(Token::Warn)
        } else {
            theme.resolve(Token::TextPrimary)
        };
        Paragraph::new(name)
            .style(fill)
            .render(Rect::new(inner.x + 1, y, 18, 1), cx);
        LineGauge::new(f32::from(u8::try_from(percent).unwrap_or(100)) / 100.0)
            .filled_style(fill.fg(tone))
            .unfilled_style(dim)
            .render(Rect::new(inner.x + 20, y, inner.width - 33, 1), cx);
        let label = if done == span {
            String::from("passed")
        } else if started {
            format!("{percent:>3}%")
        } else {
            String::from("waiting")
        };
        Paragraph::new(label)
            .style(if started { fill.fg(tone) } else { dim })
            .render(Rect::new(inner.x + inner.width - 11, y, 10, 1), cx);
    }
}

fn actions(cx: &mut RenderCx<'_>, app: &App, bar: Rect, fill: Style, theme: &Theme, tick: u32) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Actions ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(bar, cx);
    let inner = block.inner(bar);
    for (index, slot) in slots(inner).into_iter().enumerate() {
        let variant = match index {
            DEPLOY => ButtonVariant::Primary,
            ROLL_BACK => ButtonVariant::Destructive,
            _ => ButtonVariant::Normal,
        };
        let disabled = index == DEPLOY && tick < CHECKS_DONE;
        let state = if disabled {
            WidgetState::Disabled
        } else if index == app.focus {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
        Button::new(LABELS[index])
            .variant(variant)
            .state(state)
            .pressed(index == app.focus && app.held.is_pressed())
            .render(slot, cx);
    }
}
