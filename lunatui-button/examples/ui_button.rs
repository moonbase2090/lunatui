// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_badge::{Badge, Kbd};
use lunatui_button::{Button, ButtonSize, Toggle, Variant, WidgetState};
use lunatui_card::Card;
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant as ThemeVariant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

const ACTIONS: [(&str, Variant); 5] = [
    ("Deploy", Variant::Default),
    ("Rollback", Variant::Destructive),
    ("Promote", Variant::Secondary),
    ("Diff", Variant::Outline),
    ("Logs", Variant::Ghost),
];

const FLAGS: [(&str, bool); 4] = [
    ("canary", true),
    ("dry run", false),
    ("notify", true),
    ("verbose", false),
];

fn focus_at(tick: u32) -> usize {
    ((tick / 45) % 9) as usize
}

fn pressed_at(tick: u32) -> bool {
    tick % 45 >= 30 && tick % 45 < 38
}

struct Actions {
    tick: u32,
}

impl Widget for Actions {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let focus = focus_at(self.tick);
        let down = pressed_at(self.tick);
        let mut x = area.x;
        for (index, (label, variant)) in ACTIONS.iter().enumerate() {
            let state = if focus == index {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            };
            let button = Button::new(label)
                .variant(*variant)
                .state(state)
                .pressed(focus == index && down)
                .on(Role::Card);
            let w = button.measure(Size::new(30, 1)).width;
            button.render(Rect::new(x, area.y, w, 1), cx);
            x += w + 2;
        }
        let mut x = area.x;
        for (label, size) in [
            ("Small", ButtonSize::Small),
            ("Medium", ButtonSize::Medium),
            ("Large", ButtonSize::Large),
        ] {
            let button = Button::new(label)
                .size(size)
                .variant(Variant::Secondary)
                .on(Role::Card);
            let w = button.measure(Size::new(30, 1)).width;
            button.render(Rect::new(x, area.y + 2, w, 1), cx);
            x += w + 2;
        }
        let disabled = Button::new("Disabled")
            .state(WidgetState::Disabled)
            .on(Role::Card);
        let w = disabled.measure(Size::new(30, 1)).width;
        disabled.render(Rect::new(x, area.y + 2, w, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Flags {
    tick: u32,
}

impl Widget for Flags {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let focus = focus_at(self.tick);
        let mut x = area.x;
        for (index, (label, on)) in FLAGS.iter().enumerate() {
            let slot = ACTIONS.len() + index;
            let flipped = focus > slot || (focus == slot && pressed_at(self.tick));
            let on = *on != (flipped && focus >= slot);
            let state = if focus == slot {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            };
            let toggle = Toggle::new(label, on).state(state).on(Role::Card);
            let w = toggle.measure(Size::new(30, 1)).width;
            toggle.render(Rect::new(x, area.y, w, 1), cx);
            x += w + 2;
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, 1)
    }
}

const STAGES: [(&str, &str, Tone); 6] = [
    ("build", "2m 14s", Tone::Success),
    ("unit tests", "4m 02s", Tone::Success),
    ("integration", "6m 31s", Tone::Success),
    ("canary 5%", "running", Tone::Warning),
    ("canary 50%", "queued", Tone::Neutral),
    ("promote", "queued", Tone::Neutral),
];

struct Pipeline;

impl Widget for Pipeline {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let text = surface(cx, Role::Card);
        for (row, (name, time, tone)) in STAGES.iter().enumerate() {
            let y = area.y + u16::try_from(row).unwrap_or(0);
            Paragraph::new(*name)
                .style(text)
                .render(Rect::new(area.x, y, 20, 1), cx);
            Paragraph::new(*time)
                .style(text)
                .render(Rect::new(area.x + 20, y, 20, 1), cx);
            Badge::new(if *tone == Tone::Success {
                "passed"
            } else {
                time
            })
            .tone(*tone)
            .render(Rect::new(area.x + 40, y, 12, 1), cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Hints;

impl Widget for Hints {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let hints: [(&[&str], &str); 2] = [(&["Tab"], "next"), (&["Enter"], "press")];
        let page = surface(cx, Role::Background);
        let lead = "Scripted keys:";
        Paragraph::new(lead)
            .style(page)
            .render(Rect::new(area.x, area.y, 14, 1), cx);
        let mut x = area.x + 17;
        for (keys, label) in hints {
            let chip = Kbd::new(keys);
            let w = chip.measure(Size::new(30, 1)).width;
            chip.render(Rect::new(x, area.y, w, 1), cx);
            let text = format!(" {label}");
            let text_width = u16::try_from(text.chars().count()).unwrap_or(0);
            Paragraph::new(text)
                .style(page)
                .render(Rect::new(x + w, area.y, text_width, 1), cx);
            x += w + text_width + 3;
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, 1)
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = surface(&cx, Role::Background);
    Block::new().style(backdrop).render(area, &mut cx);
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Deploy console")
        .style(title)
        .render(Rect::new(2, 0, 40, 1), &mut cx);
    let badge = Badge::new("production").tone(Tone::Success);
    badge.render(Rect::new(106, 0, 12, 1), &mut cx);
    let actions = Actions { tick };
    let flags = Flags { tick };
    Card::new("Release 2.41.0")
        .description("A scripted run: focus steps through each action and presses it")
        .body(&actions)
        .render(Rect::new(1, 2, 118, 10), &mut cx);
    Card::new("Options")
        .description("Toggles stay pressed until you flip them")
        .body(&flags)
        .render(Rect::new(1, 12, 118, 8), &mut cx);
    Card::new("Pipeline")
        .description("Stages for release 2.41.0")
        .body(&Pipeline)
        .render(Rect::new(1, 20, 118, 14), &mut cx);
    Hints.render(Rect::new(3, 35, 114, 1), &mut cx);
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
    let theme = Theme::builtin(ThemeVariant::Dark);
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
