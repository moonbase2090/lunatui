// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Deploy console built from `lunatui-card`. `ui_card` plays an animated run (`--width`,
//! `--height`, `--frames`, `--fps`); `ui_card --snapshot` prints one frame as text.

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_card::{Card, Empty, Separator};
use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Size, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::{Role, Tone, surface};
use lunatui_widgets::{Gauge, Paragraph, RenderCx, Widget};

/// Big number over a gauge.
struct Stat {
    value: String,
    ratio: f32,
    tone: Tone,
}

impl Widget for Stat {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.height < 3 {
            return;
        }
        let bold = surface(cx, Role::Card).with_attrs(Attrs::new().set_bold(true));
        cx.put_str(area.x, area.y, &self.value, bold);
        let fill = cx.token(self.tone.fill());
        let track = cx.token(Role::Border.token());
        Gauge::new(self.ratio)
            .gauge_style(Style::new().fg(fill))
            .track_style(Style::new().fg(track))
            .render(Rect::new(area.x, area.y + 2, area.width, 1), cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, 3)
    }
}

/// Deploy rows separated by rules, newest first.
struct Deploys {
    rows: Vec<(String, &'static str, Tone)>,
}

impl Widget for Deploys {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let mut y = area.y;
        for (index, (name, status, tone)) in self.rows.iter().enumerate() {
            if y + 1 >= area.y + area.height {
                break;
            }
            if index > 0 {
                Separator::horizontal()
                    .on(Role::Card)
                    .render(Rect::new(area.x, y, area.width, 1), cx);
                y += 1;
            }
            Paragraph::new(name.as_str())
                .style(surface(cx, Role::Card))
                .render(Rect::new(area.x, y, area.width.saturating_sub(14), 1), cx);
            let chip = format!(" {status} ");
            let w = u16::try_from(chip.chars().count()).unwrap_or(0);
            cx.put_str(
                area.x + area.width.saturating_sub(w),
                y,
                &chip,
                tone.style(cx),
            );
            y += 1;
        }
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(avail.width, avail.height)
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let backdrop = Style::new().bg(cx.token(Role::Background.token()));
    cx.fill(area, backdrop, ' ');
    let bold = Style::new()
        .fg(cx.token(Role::Primary.token()))
        .bg(cx.token(Role::Background.token()))
        .with_attrs(Attrs::new().set_bold(true));
    cx.put_str(2, 0, "LUNATUI  /  Deploy console", bold);

    let seconds = tick / 30;
    // Triangle wave in 0..=100 so the demo needs no float casts.
    let wave = |period: u32, offset: u32| -> u16 {
        let phase = (tick + offset) % period;
        let half = period / 2;
        let rise = if phase < half { phase } else { period - phase };
        u16::try_from(rise * 100 / half).unwrap_or(100)
    };
    let ratio = |base: u16, swing: u16, w: u16| f32::from(base + swing * w / 100) / 100.0;
    let requests = Stat {
        value: format!("{} req/s", 4_120 + tick * 3),
        ratio: ratio(55, 30, wave(240, 0)),
        tone: Tone::Primary,
    };
    let latency = Stat {
        value: format!("{} ms p95", 112 + u32::from(wave(120, 30)) * 3 / 10),
        ratio: ratio(30, 30, wave(120, 30)),
        tone: Tone::Warning,
    };
    let budget = Stat {
        value: format!("{}.{} % left", 98 - seconds / 8, 9 - (tick / 3) % 10),
        ratio: ratio(90, 0, 0) - f32::from(u16::try_from(tick).unwrap_or(0)) / 2400.0,
        tone: Tone::Success,
    };
    let stats: [(&str, &str, &Stat); 3] = [
        ("Requests", "Last minute", &requests),
        ("Latency", "Rolling window", &latency),
        ("Error budget", "30 day target", &budget),
    ];
    for (x, (title, desc, stat)) in (1_u16..).step_by(40).zip(stats) {
        Card::new(title)
            .description(desc)
            .body(stat)
            .render(Rect::new(x, 2, 38, 10), &mut cx);
    }

    let names = [
        ("api  v2.41.0  to production", "live", Tone::Success),
        ("web  v8.12.3  to production", "live", Tone::Success),
        ("worker  v1.9.0  to staging", "rolling", Tone::Warning),
        ("api  v2.40.2  to production", "live", Tone::Success),
        ("billing  v3.2.1  to production", "failed", Tone::Danger),
        ("web  v8.12.2  to production", "live", Tone::Success),
        ("search  v0.14.0  to staging", "live", Tone::Success),
        ("auth  v5.0.4  to production", "live", Tone::Success),
        ("docs  v1.3.0  to production", "live", Tone::Success),
        ("mail  v2.8.8  to staging", "live", Tone::Success),
    ];
    // A new deploy lands every 1.5 seconds and pushes the rest down.
    let newest = (tick / 45) as usize;
    let deploys = Deploys {
        rows: (0..7)
            .map(|i| {
                let (n, s, t) = names[(newest + names.len() - i) % names.len()];
                (n.to_owned(), s, t)
            })
            .collect(),
    };
    let footer = format!("Updated {seconds}s ago");
    Card::new("Recent deploys")
        .description("Newest first")
        .footer(&footer)
        .body(&deploys)
        .render(Rect::new(1, 13, 74, 22), &mut cx);
    let calm = Empty::new("All clear", "No alerts in the last hour").on(Role::Card);
    Card::new("Alerts")
        .description("Paging rules")
        .body(&calm)
        .render(Rect::new(77, 13, 42, 22), &mut cx);
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
