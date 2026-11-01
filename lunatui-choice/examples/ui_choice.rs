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

const SLOT: u32 = 15;
const FREQUENCIES: [&str; 3] = ["Immediately", "Hourly digest", "Daily digest"];
const EVENTS: [&str; 3] = ["Deploys", "Failed checks", "Review requests"];
const STOPS: u32 = 9;

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

fn flips(step: u32, slot: u32) -> u32 {
    if step > slot {
        (step - slot - 1) / STOPS + 1
    } else {
        0
    }
}

struct Model {
    focus: u32,
    events: [bool; 3],
    frequency: usize,
    switches: [bool; 2],
    saved: bool,
    pressing: bool,
}

impl Model {
    fn at(tick: u32) -> Self {
        let step = tick / SLOT;
        let focus = step % STOPS;
        let on = |base: bool, slot: u32| base != (flips(step, slot) % 2 == 1);
        let frequency = if focus >= 3 {
            (focus - 3).min(2) as usize
        } else if step < STOPS {
            0
        } else {
            2
        };
        Self {
            focus,
            events: [on(true, 0), on(true, 1), on(false, 2)],
            frequency,
            switches: [on(false, 6), on(true, 7)],
            saved: step == 0 || (step > 8 && step < 10),
            pressing: step == 8 && tick % SLOT >= SLOT / 2,
        }
    }

    fn all(&self) -> CheckState {
        let count = self.events.iter().filter(|on| **on).count();
        match count {
            0 => CheckState::Unchecked,
            3 => CheckState::Checked,
            _ => CheckState::Mixed,
        }
    }
}

struct Events {
    model: Model,
}

impl Widget for Events {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        for (index, label) in EVENTS.iter().enumerate() {
            let slot = u32::try_from(index).unwrap_or(0);
            let row = Rect::new(area.x, area.y + u16::try_from(index).unwrap_or(0), 30, 1);
            Checkbox::new(label)
                .checked(checked(self.model.events[index]))
                .state(state_for(self.model.focus, slot))
                .on(Role::Card)
                .render(row, cx);
        }
        Checkbox::new("Select all")
            .checked(self.model.all())
            .on(Role::Card)
            .render(Rect::new(area.x + 32, area.y, 30, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

struct Delivery {
    model: Model,
}

impl Widget for Delivery {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let focus = self.model.focus;
        Radio::new(FREQUENCIES)
            .selected(Some(self.model.frequency))
            .focused((3..=5).contains(&focus).then(|| (focus - 3) as usize))
            .on(Role::Card)
            .render(Rect::new(area.x, area.y, 30, 3), cx);
        Switch::new("Post to Slack", self.model.switches[0])
            .state(state_for(focus, 6))
            .on_surface(Role::Card)
            .render(Rect::new(area.x + 32, area.y, 30, 1), cx);
        Switch::new("Email me", self.model.switches[1])
            .state(state_for(focus, 7))
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
    model: Model,
}

impl Widget for Summary {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let model = &self.model;
        let events = model.events.iter().filter(|on| **on).count();
        let onoff = |on: bool| if on { "on" } else { "off" };
        let lines = [
            format!("Events selected: {events} of 3"),
            format!("Frequency: {}", FREQUENCIES[model.frequency]),
            format!(
                "Slack: {}, email: {}, text message: unavailable",
                onoff(model.switches[0]),
                onoff(model.switches[1])
            ),
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
    *grid = Grid::new(area.width, area.height);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = surface(&cx, Role::Background);
    Block::new().style(backdrop).render(area, &mut cx);
    let title = backdrop
        .fg(cx.token(Role::Primary.token()))
        .with_attrs(Attrs::new().set_bold(true));
    Paragraph::new("LUNATUI  /  Notification preferences")
        .style(title)
        .render(Rect::new(2, 0, 50, 1), &mut cx);
    let model = Model::at(tick);
    let (badge, tone) = if model.saved {
        ("saved", Tone::Success)
    } else {
        ("unsaved", Tone::Warning)
    };
    Badge::new(badge)
        .tone(tone)
        .render(Rect::new(106, 0, 12, 1), &mut cx);
    let events = Events {
        model: Model::at(tick),
    };
    let delivery = Delivery {
        model: Model::at(tick),
    };
    let summary = Summary {
        model: Model::at(tick),
    };
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
        .state(state_for(model.focus, 8))
        .pressed(model.pressing)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(grid: &mut Grid, clusters: &mut ClusterTable, tick: u32) -> String {
        let theme = Theme::builtin(ThemeVariant::Dark);
        draw(grid, clusters, &theme, Rect::new(0, 0, 120, 36), tick);
        grid.display_text(clusters)
    }

    fn summary_rows(text: &str) -> Vec<&str> {
        text.lines().skip(25).take(3).collect()
    }

    fn row(text: &str) -> String {
        format!(" │  {text:<114}│ ")
    }

    #[test]
    fn a_shrinking_line_leaves_no_trailing_cells() {
        let mut clusters = ClusterTable::new();
        let mut grid = Grid::new(120, 36);
        let long = frame(&mut grid, &mut clusters, 135);
        assert_eq!(
            summary_rows(&long),
            [
                row("Events selected: 1 of 3"),
                row("Frequency: Daily digest"),
                row("Slack: on, email: off, text message: unavailable"),
            ]
        );
        let short = frame(&mut grid, &mut clusters, 0);
        assert_eq!(
            summary_rows(&short),
            [
                row("Events selected: 2 of 3"),
                row("Frequency: Immediately"),
                row("Slack: off, email: on, text message: unavailable"),
            ]
        );
        let again = frame(&mut grid, &mut clusters, 105);
        assert_eq!(
            summary_rows(&again),
            [
                row("Events selected: 1 of 3"),
                row("Frequency: Daily digest"),
                row("Slack: on, email: on, text message: unavailable"),
            ]
        );
    }

    #[test]
    fn the_summary_line_reads_cleanly_after_it_shrinks() {
        let mut clusters = ClusterTable::new();
        let mut grid = Grid::new(120, 36);
        let slack_off = frame(&mut grid, &mut clusters, 0);
        assert!(slack_off.contains("Slack: off, email: on, text message: unavailable "));
        let slack_on = frame(&mut grid, &mut clusters, 7 * SLOT);
        assert!(slack_on.contains("Slack: on, email: on, text message: unavailable "));
        assert!(!slack_on.contains("unavailablee"));
    }

    #[test]
    fn the_script_changes_something_every_stop() {
        let mut clusters = ClusterTable::new();
        let mut frames = Vec::new();
        for step in 0..18 {
            let mut grid = Grid::new(120, 36);
            let text = frame(&mut grid, &mut clusters, step * SLOT);
            let styles: Vec<_> = (0..36)
                .flat_map(|y| (0..120).map(move |x| (x, y)))
                .map(|(x, y)| grid.get(x, y).map(lunatui_core::Cell::style))
                .collect();
            frames.push((text, styles));
        }
        for pair in frames.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
    }
}
