// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_badge::{Badge, Kbd, Look};
use lunatui_card::{Card, Separator};
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{RenderCx, Widget};

const SERVICES: [(&str, &str, &str); 7] = [
    ("api", "v2.41.0", "platform"),
    ("web", "v8.12.3", "frontend"),
    ("worker", "v1.9.0", "platform"),
    ("billing", "v3.2.1", "payments"),
    ("search", "v0.14.0", "discovery"),
    ("auth", "v5.0.4", "identity"),
    ("mail", "v2.8.8", "growth"),
];

fn state(index: usize, tick: u32) -> (&'static str, Tone) {
    let step = tick / 40;
    let slot = u32::try_from(index).unwrap_or(0);
    match (slot + 7 - step % 7) % 7 {
        0 => ("rolling", Tone::Warning),
        3 => ("failed", Tone::Danger),
        5 => ("queued", Tone::Neutral),
        _ => ("live", Tone::Success),
    }
}

struct Board {
    tick: u32,
}

impl Widget for Board {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let bold = Attrs::new().set_bold(true);
        let mut y = area.y;
        for (index, (name, version, team)) in SERVICES.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }
            if index > 0 {
                Separator::horizontal()
                    .on(Role::Card)
                    .render(Rect::new(area.x, y, area.width, 1), cx);
                y += 1;
            }
            cx.put_str(area.x, y, name, surface(cx, Role::Card).with_attrs(bold));
            cx.put_str(area.x + 12, y, version, surface(cx, Role::Card));
            Badge::new(team)
                .look(Look::Subtle)
                .render(Rect::new(area.x + 24, y, 14, 1), cx);
            let (label, tone) = state(index, self.tick);
            let badge = Badge::new(label).tone(tone);
            let w = badge.measure(Size::new(20, 1)).width;
            badge.render(Rect::new(area.x + area.width - w, y, w, 1), cx);
            y += 1;
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Legend;

impl Widget for Legend {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let tones = [
            ("neutral", Tone::Neutral),
            ("primary", Tone::Primary),
            ("success", Tone::Success),
            ("warning", Tone::Warning),
            ("danger", Tone::Danger),
        ];
        for (row, look) in [(0, Look::Filled), (1, Look::Subtle)] {
            let mut x = area.x;
            for (name, tone) in tones {
                let badge = Badge::new(name).tone(tone).look(look);
                let w = badge.measure(Size::new(20, 1)).width;
                badge.render(Rect::new(x, area.y + row, w, 1), cx);
                x += w + 2;
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, 2)
    }
}

struct Hints;

impl Widget for Hints {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let hints: [(&[&str], &str); 4] = [
            (&["j", "k"], "move"),
            (&["Enter"], "open"),
            (&["Ctrl", "K"], "commands"),
            (&["q"], "quit"),
        ];
        let mut x = area.x;
        for (keys, label) in hints {
            let chip = Kbd::new(keys);
            let w = chip.measure(Size::new(30, 1)).width;
            chip.render(Rect::new(x, area.y, w, 1), cx);
            let text = format!(" {label}");
            cx.put_str(x + w, area.y, &text, surface(cx, Role::Background));
            x += w + u16::try_from(text.chars().count()).unwrap_or(0) + 3;
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, 1)
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = surface(&cx, Role::Background);
    cx.fill(area, backdrop, ' ');
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(2, 0, "LUNATUI  /  Release board", title);
    let board = Board { tick };
    let hints = Hints;
    Card::new("Services")
        .description("Production and staging rollout state")
        .footer("Press Ctrl K for the command palette")
        .body(&board)
        .render(Rect::new(1, 2, 118, 22), &mut cx);
    Card::new("Legend")
        .description("Tones and looks")
        .body(&Legend)
        .render(Rect::new(1, 25, 118, 9), &mut cx);
    hints.render(Rect::new(3, 35, 114, 1), &mut cx);
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let flag = |name: &str, default: u32| -> u32 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let width = u16::try_from(flag("--width", 120)).unwrap_or(120);
    let height = u16::try_from(flag("--height", 36)).unwrap_or(36);
    let theme = Theme::builtin(Variant::Dark);
    let area = Rect::new(0, 0, width, height);
    let mut clusters = ClusterTable::new();
    let mut next = Grid::new(width, height);
    if args.iter().any(|a| a == "--snapshot") {
        draw(&mut next, &mut clusters, &theme, area, 0);
        print!("{}", next.display_text(&clusters));
        return Ok(());
    }
    let frames = flag("--frames", 270);
    let fps = flag("--fps", 30).max(1);
    let period = Duration::from_secs(1) / fps;
    let mut previous = Grid::new(width, height);
    let mut encoder = Encoder::new();
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let started = Instant::now();
    for tick in 0..frames {
        draw(&mut next, &mut clusters, &theme, area, tick);
        let mut frame = b"\x1b[?2026h".to_vec();
        frame.extend_from_slice(encoder.encode_diff(
            &previous,
            &next,
            &clusters,
            ColorMode::TrueColor,
        ));
        frame.extend_from_slice(b"\x1b[?2026l");
        out.write_all(&frame)?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        let due = started + period * (tick + 1);
        thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    out.flush()
}
