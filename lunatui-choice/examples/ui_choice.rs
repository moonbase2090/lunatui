// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_badge::Badge;
use lunatui_button::Button;
use lunatui_card::Card;
use lunatui_choice::{CheckState, Checkbox, Radio, Switch, WidgetState};
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant as ThemeVariant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{Block, Paragraph, RenderCx, Widget};

const SLOT: u32 = 38;
const FREQUENCIES: [&str; 3] = ["Immediately", "Hourly digest", "Daily digest"];
const EVENTS: [&str; 3] = ["Deploys", "Failed checks", "Review requests"];

fn step(tick: u32) -> u32 {
    tick / SLOT
}

fn state_for(focus: u32, slot: u32) -> WidgetState {
    if focus == slot {
        WidgetState::Focused
    } else {
        WidgetState::Normal
    }
}

fn checked(on: bool) -> CheckState {
    if on {
        CheckState::Checked
    } else {
        CheckState::Unchecked
    }
}

struct Events {
    tick: u32,
}

impl Widget for Events {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let focus = step(self.tick);
        let on = [true, focus <= 1, focus > 2];
        for (index, label) in EVENTS.iter().enumerate() {
            let slot = u32::try_from(index).unwrap_or(0);
            let row = Rect::new(area.x, area.y + u16::try_from(index).unwrap_or(0), 30, 1);
            Checkbox::new(label)
                .checked(checked(on[index]))
                .state(state_for(focus, slot))
                .on(Role::Card)
                .render(row, cx);
        }
        Checkbox::new("Select all")
            .checked(CheckState::Mixed)
            .on(Role::Card)
            .render(Rect::new(area.x + 32, area.y, 30, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Delivery {
    tick: u32,
}

impl Widget for Delivery {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let focus = step(self.tick);
        let picked = usize::from(focus >= 5);
        let on_radio = (3..=5).contains(&focus);
        Radio::new(FREQUENCIES)
            .selected(Some(picked))
            .focused(on_radio.then(|| (focus - 3) as usize))
            .on(Role::Card)
            .render(Rect::new(area.x, area.y, 30, 3), cx);
        Switch::new("Post to Slack", focus >= 6)
            .state(state_for(focus, 6))
            .on_surface(Role::Card)
            .render(Rect::new(area.x + 32, area.y, 30, 1), cx);
        Switch::new("Email me", true)
            .on_surface(Role::Card)
            .render(Rect::new(area.x + 32, area.y + 1, 30, 1), cx);
        Switch::new("Text message", false)
            .state(WidgetState::Disabled)
            .on_surface(Role::Card)
            .render(Rect::new(area.x + 32, area.y + 2, 30, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Summary {
    tick: u32,
}

impl Widget for Summary {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let focus = step(self.tick);
        let events = 1 + u32::from(focus <= 1) + u32::from(focus > 2);
        let frequency = FREQUENCIES[usize::from(focus >= 5)];
        let slack = if focus >= 6 { "on" } else { "off" };
        let lines = [
            format!("Events selected: {events} of 3"),
            format!("Frequency: {frequency}"),
            format!("Slack: {slack}, email: on, text message: unavailable"),
        ];
        for (row, line) in lines.iter().enumerate() {
            let y = area.y + u16::try_from(row).unwrap_or(0);
            Paragraph::new(line.as_str())
                .style(surface(cx, Role::Card))
                .render(Rect::new(area.x, y, area.width, 1), cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = surface(&cx, Role::Background);
    Block::new().style(backdrop).render(area, &mut cx);
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Notification preferences")
        .style(title)
        .render(Rect::new(2, 0, 50, 1), &mut cx);
    Badge::new("saved")
        .tone(Tone::Success)
        .render(Rect::new(106, 0, 12, 1), &mut cx);
    let events = Events { tick };
    let delivery = Delivery { tick };
    let summary = Summary { tick };
    Card::new("Events")
        .description("A scripted run: focus steps through the choices and flips them")
        .body(&events)
        .render(Rect::new(1, 2, 118, 9), &mut cx);
    Card::new("Delivery")
        .description("One frequency, plus where the messages go")
        .body(&delivery)
        .render(Rect::new(1, 11, 118, 9), &mut cx);
    Card::new("Summary")
        .description("What will be saved")
        .body(&summary)
        .render(Rect::new(1, 20, 118, 10), &mut cx);
    Button::new("Save preferences")
        .on(Role::Background)
        .render(Rect::new(3, 31, 20, 1), &mut cx);
    Paragraph::new("[-] means some choices in the group are on. Dimmed rows are unavailable.")
        .style(backdrop)
        .render(Rect::new(3, 35, 114, 1), &mut cx);
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
        draw(&mut next, &mut clusters, &theme, area, flag("--tick", 0));
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
