// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_runtime::{
    App, ControlFlow, Ease, FocusDirection, FocusManager, HitId, HitMap, RunOptions, SchedulePoll,
    Scheduler, Spring, Tween, is_quit, render_once, run,
};
use lunatui_term::{ColorMode, Encoder, Event, KeyCode, KeyEvent, MouseButton, MouseKind};
use lunatui_theme::{Theme, Token, Variant, load_bundled};
use lunatui_widgets::{Block, RenderCx, StatusDot, StatusState, Widget};

const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 24;
const BENCH_FRAMES: u32 = 4;

fn bundled_or_builtin(id: &'static str) -> (&'static str, Theme) {
    load_bundled(id).map_or_else(
        |_| ("builtin", Theme::builtin(Variant::Dark)),
        |theme| (id, theme),
    )
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let counter = args.iter().any(|arg| arg == "--counter");
    let motion = args.iter().any(|arg| arg == "--motion");
    let snapshot = args.iter().any(|arg| arg == "--snapshot");
    let ratatui_bridge = args.iter().any(|arg| arg == "--ratatui-bridge");
    let ratatui_widget = args.iter().any(|arg| arg == "--ratatui-widget");
    let ratatui_style = args.iter().any(|arg| arg == "--ratatui-style");
    let porting = args.iter().any(|arg| arg == "--porting");
    let simd = args.iter().any(|arg| arg == "--simd");
    let flag_args: Vec<String> = args
        .iter()
        .filter(|arg| {
            *arg != "--counter"
                && *arg != "--motion"
                && *arg != "--snapshot"
                && *arg != "--ratatui-bridge"
                && *arg != "--ratatui-widget"
                && *arg != "--ratatui-style"
                && *arg != "--porting"
                && *arg != "--simd"
        })
        .cloned()
        .collect();
    let geometry = parse_geometry(&flag_args)?;
    if snapshot {
        if simd {
            print_simd_snapshot(geometry);
        } else if porting {
            print_porting_snapshot(geometry);
        } else if ratatui_style {
            print_ratatui_style_snapshot(geometry);
        } else if ratatui_widget {
            print_ratatui_widget_snapshot(geometry);
        } else if ratatui_bridge {
            print_ratatui_bridge_snapshot(geometry)?;
        } else {
            print_snapshot(counter, motion, geometry);
        }
        return Ok(());
    }
    if io::stdout().is_terminal() {
        let options = RunOptions::new(geometry.width, geometry.height);
        if counter {
            run(Counter::new(motion), options)?;
        } else {
            run(Hello, options)?;
        }
        return Ok(());
    }
    let times = run_bench(counter, geometry)?;
    let label = if counter {
        "lunatui demo counter (App + run)"
    } else {
        "lunatui demo hello (App + run, bench uses counter render path)"
    };
    println!(
        "{}",
        bench_summary(
            geometry.width,
            geometry.height,
            times,
            build_profile(),
            label
        )
    );
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct Hello;

impl App for Hello {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let (theme_id, theme) = bundled_or_builtin("prismattyc-dark");
        let _ = theme_id;
        let block = Block::bordered().title("lunatui");
        let mut cx = RenderCx::new(grid, clusters, &theme);
        block.render(area, &mut cx);
        let inner = block.inner(area);
        let accent = Style::new().fg(Color::Rgb(0x7a, 0xa2, 0xf7));
        cx.put_str(inner.x, inner.y.saturating_add(1), "Hello, lunatui", accent);
        cx.put_str(
            inner.x,
            inner.y.saturating_add(3),
            "PR 23: App + run() — press q to quit",
            Style::new(),
        );
    }
}

const HIT_DEC: HitId = HitId(1);
const HIT_INC: HitId = HitId(2);

#[derive(Clone, Debug)]
struct Counter {
    value: i32,
    motion: bool,
    display: Spring,
    accent: Tween<Color>,
    accent_start: Duration,
    motion_clock: Duration,
    last_tick: Option<Instant>,
    hits: HitMap,
    focus: FocusManager,
    hover: Option<HitId>,
    drag: Option<HitId>,
    theme: Theme,
    theme_id: &'static str,
    palette: Vec<Color>,
}

impl Counter {
    fn new(motion: bool) -> Self {
        let (theme_id, theme) = bundled_or_builtin("prismattyc-dark");
        let palette = theme.palette();
        Self {
            value: 0,
            motion,
            display: Spring::new(0.0),
            accent: Tween::new(
                Color::Rgb(0x1e, 0x1e, 0x2e),
                Color::Rgb(0x9e, 0xce, 0x6a),
                Duration::from_millis(400),
                Ease::InOutCubic,
            ),
            accent_start: Duration::ZERO,
            motion_clock: Duration::ZERO,
            last_tick: None,
            hits: HitMap::new(),
            focus: FocusManager::new(),
            hover: None,
            drag: None,
            theme,
            theme_id,
            palette,
        }
    }

    fn toggle_theme(&mut self) {
        let next_id = match self.theme_id {
            "prismattyc-dark" | "prismattyc" | "prismattyc-default" => "prismattyc-light",
            "prismattyc-light" => "prismattyc-dark",
            _ => match self.theme.variant() {
                Variant::Light => "prismattyc-dark",
                Variant::Dark => "prismattyc-light",
            },
        };
        let (theme_id, theme) = bundled_or_builtin(next_id);
        self.theme_id = theme_id;
        self.theme = theme;
        self.palette = self.theme.palette();
    }

    fn activate(&mut self, id: HitId) {
        if id == HIT_DEC {
            self.value -= 1;
        } else if id == HIT_INC {
            self.value += 1;
        }
        if self.motion {
            self.display.set_target(spring_target_for(self.value));
            self.accent_start = self.motion_clock;
        }
    }

    fn tick_motion(&mut self, delta: Duration) {
        if !self.motion {
            return;
        }
        self.motion_clock += delta;
        self.display.advance(delta);
    }

    fn sync_motion_clock(&mut self) {
        if !self.motion {
            return;
        }
        let now = Instant::now();
        if let Some(prev) = self.last_tick {
            self.tick_motion(now.saturating_duration_since(prev));
        }
        self.last_tick = Some(now);
    }

    fn shown_value(&self) -> i32 {
        if self.motion {
            i32::from(nearest_i16(self.display.value()))
        } else {
            self.value
        }
    }

    fn accent_running(&self) -> bool {
        self.motion_clock.saturating_sub(self.accent_start) < self.accent.duration()
    }
}

impl App for Counter {
    fn needs_animation_frames(&self) -> bool {
        self.motion && (!self.display.is_settled() || self.accent_running())
    }

    fn token_palette(&self) -> Option<&[Color]> {
        Some(&self.palette)
    }

    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            return ControlFlow::Break;
        }
        match event {
            Event::Key(key) => self.on_key(key),
            Event::Mouse(mouse) => self.on_mouse(mouse),
            _ => {}
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        self.sync_motion_clock();
        let surface = Style::new().bg(Color::Token(Token::Surface0.id()));
        let border = Style::new().fg(Color::Token(Token::Accent.id()));
        let block = Block::bordered()
            .title("Counter")
            .style(surface)
            .border_style(border);
        let mut cx = RenderCx::new(grid, clusters, &self.theme);
        block.render(area, &mut cx);
        let inner = block.inner(area);
        let dot_state = if self.value >= 0 {
            StatusState::Ok
        } else {
            StatusState::Fail
        };
        let dot_area = Rect::new(
            inner.x.saturating_add(inner.width.saturating_sub(1)),
            inner.y,
            1,
            1,
        );
        let dot = StatusDot::new(dot_state);
        dot.render(dot_area, &mut cx);
        let line = format!("value: {value}", value = self.shown_value());
        let text = Style::new().fg(Color::Token(Token::TextPrimary.id()));
        let value_y = inner.y.saturating_add(1);
        cx.put_str(inner.x, value_y, &line, text);
        if self.motion {
            let elapsed = self.motion_clock.saturating_sub(self.accent_start);
            let bar = self.accent.sample(elapsed);
            let bar_rect = Rect::new(inner.x, inner.y.saturating_add(2), 20, 1);
            cx.set_style(bar_rect, Style::new().bg(bar));
        }
        cx.put_str(
            inner.x,
            inner.y.saturating_add(4),
            "Tab/click; t prismattyc light/dark; q quit",
            Style::new(),
        );

        let btn_y = inner.y.saturating_add(6);
        let dec = Rect::new(inner.x, btn_y, 5, 1);
        let inc = Rect::new(inner.x.saturating_add(7), btn_y, 5, 1);
        self.hits.clear();
        self.hits.push(HIT_DEC, dec, 0);
        self.hits.push(HIT_INC, inc, 1);
        self.focus.sync(vec![(HIT_DEC, dec), (HIT_INC, inc)]);

        let focused = self.focus.focused();
        draw_button(
            grid,
            dec,
            "[ - ]",
            focused == Some(HIT_DEC),
            self.hover == Some(HIT_DEC),
            clusters,
        );
        draw_button(
            grid,
            inc,
            "[ + ]",
            focused == Some(HIT_INC),
            self.hover == Some(HIT_INC),
            clusters,
        );
    }
}

impl Counter {
    fn on_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Tab => self.focus.tab(key.modifiers.is_shift()),
            KeyCode::Up => self.focus.arrow(FocusDirection::Up),
            KeyCode::Down => self.focus.arrow(FocusDirection::Down),
            KeyCode::Left => self.focus.arrow(FocusDirection::Left),
            KeyCode::Right => self.focus.arrow(FocusDirection::Right),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(id) = self.focus.focused() {
                    self.activate(id);
                }
            }
            KeyCode::Char('t') => self.toggle_theme(),
            KeyCode::Char('+' | '=') => {
                self.value += 1;
                if self.motion {
                    self.display.set_target(spring_target_for(self.value));
                    self.accent_start = self.motion_clock;
                }
            }
            KeyCode::Char('-') => {
                self.value -= 1;
                if self.motion {
                    self.display.set_target(spring_target_for(self.value));
                    self.accent_start = self.motion_clock;
                }
            }
            _ => {}
        }
    }

    fn on_mouse(&mut self, mouse: &lunatui_term::MouseEvent) {
        if mouse.unit != lunatui_term::MouseUnit::Cells {
            return;
        }
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                if let Some(id) = self.hits.pick(mouse.column, mouse.row) {
                    self.drag = Some(id);
                    self.focus.focus(id);
                    self.activate(id);
                }
            }
            MouseKind::Up(MouseButton::Left) => {
                self.drag = None;
            }
            MouseKind::Drag(MouseButton::Left) | MouseKind::Move => {
                self.hover = self.hits.pick(mouse.column, mouse.row);
            }
            _ => {}
        }
    }
}

fn draw_button(
    grid: &mut Grid,
    area: Rect,
    label: &str,
    focused: bool,
    hovered: bool,
    clusters: &mut ClusterTable,
) {
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let style = if focused {
        Style::new().fg(Color::Rgb(0x1a, 0x1b, 0x26)).bg(accent)
    } else if hovered {
        Style::new().fg(accent)
    } else {
        Style::new()
    };
    grid.put_str(area.x, area.y, style, label, clusters);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Geometry {
    width: u16,
    height: u16,
}

fn parse_geometry(args: &[String]) -> io::Result<Geometry> {
    let mut width = None;
    let mut height = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--width" => {
                index += 1;
                width = Some(parse_u16(args.get(index), "--width")?);
            }
            "--height" => {
                index += 1;
                height = Some(parse_u16(args.get(index), "--height")?);
            }
            flag => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag: {flag}"),
                ));
            }
        }
        index += 1;
    }
    let (width, height) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (None, None) => terminal_size().unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT)),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pass both --width and --height, or neither",
            ));
        }
    };
    Ok(Geometry { width, height })
}

fn parse_u16(value: Option<&String>, flag: &str) -> io::Result<u16> {
    let text = value.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
    })?;
    text.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

fn terminal_size() -> Option<(u16, u16)> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let stdout = io::stdout();
        if !stdout.is_terminal() {
            return None;
        }
        let mut winsize = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ok =
            // SAFETY: TIOCGWINSZ reads the tty geometry for stdout when it is a terminal.
            unsafe { libc::ioctl(stdout.as_raw_fd(), libc::TIOCGWINSZ, &raw mut winsize) == 0 };
        if !ok || winsize.ws_col == 0 || winsize.ws_row == 0 {
            return None;
        }
        Some((winsize.ws_col, winsize.ws_row))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn print_snapshot(counter: bool, motion: bool, geometry: Geometry) {
    let (grid, _) = if counter {
        let mut app = Counter::new(motion);
        app.value = 3;
        if motion {
            app.display.set_target(spring_target_for(app.value));
            for _ in 0..20 {
                app.tick_motion(Duration::from_millis(16));
            }
            app.accent_start = Duration::from_millis(200);
            app.tick_motion(Duration::from_millis(200));
        }
        render_once(&mut app, geometry.width, geometry.height)
    } else {
        let mut hello = Hello;
        render_once(&mut hello, geometry.width, geometry.height)
    };
    for y in 0..geometry.height {
        let mut line = String::new();
        for x in 0..geometry.width {
            let ch = match grid.get(x, y) {
                Some(cell) => char::from_u32(cell.glyph().bits()).unwrap_or(' '),
                None => ' ',
            };
            line.push(ch);
        }
        println!("{line}");
    }
}

fn run_bench(_counter: bool, geometry: Geometry) -> io::Result<FrameTimes> {
    let mut app = Counter::new(false);
    let mut prev = Grid::new(geometry.width, geometry.height);
    let mut next = Grid::new(geometry.width, geometry.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let area = Rect::new(0, 0, geometry.width, geometry.height);
    let mut samples = Vec::new();
    let epoch = Instant::now();
    let mut sched = Scheduler::idle();
    sched.on_input();
    let mut frame = 0_u32;
    while frame < BENCH_FRAMES {
        let now = epoch.elapsed();
        match sched.poll(now, false) {
            SchedulePoll::WaitInput => break,
            SchedulePoll::SleepUntil(deadline) => thread::sleep(deadline.saturating_sub(now)),
            SchedulePoll::Render => {
                let start = Instant::now();
                app.value = i32::try_from(frame).unwrap_or(0);
                next.fill(area, Style::new().bg(Color::Default), ' ', &mut clusters);
                app.render(&mut next, area, &mut clusters);
                let runs = diff_runs(&prev, &next);
                let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
                if payload.is_empty() && frame > 0 {
                    return Err(io::Error::other("encode_diff returned an empty payload"));
                }
                if runs.is_empty() && frame > 0 {
                    return Err(io::Error::other("expected diff runs after the first frame"));
                }
                samples.push(start.elapsed());
                prev = next;
                next = Grid::new(geometry.width, geometry.height);
                sched.after_present(epoch.elapsed(), Duration::ZERO);
                frame += 1;
                sched.on_input();
            }
        }
    }
    FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no samples"))
}

fn spring_target_for(value: i32) -> f32 {
    let capped = value.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    f32::from(i16::try_from(capped).unwrap_or(0))
}

fn nearest_i16(value: f32) -> i16 {
    let lower = floor_i16(value);
    if lower == i16::MAX {
        return lower;
    }
    let upper = lower + 1;
    let clamped = value.clamp(f32::from(i16::MIN), f32::from(i16::MAX));
    let toward_lower = clamped - f32::from(lower);
    let toward_upper = f32::from(upper) - clamped;
    if toward_upper < toward_lower {
        upper
    } else {
        lower
    }
}

fn floor_i16(value: f32) -> i16 {
    let clamped = value.clamp(f32::from(i16::MIN), f32::from(i16::MAX));
    let mut low = i32::from(i16::MIN);
    let mut high = i32::from(i16::MAX);
    let mut answer = i16::MIN;
    while low <= high {
        let mid = low + (high - low) / 2;
        let sample = f32::from(i16::try_from(mid).unwrap_or(0));
        if sample <= clamped {
            answer = i16::try_from(mid).unwrap_or(i16::MIN);
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    answer
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    }
}

fn bench_summary(width: u16, height: u16, times: FrameTimes, profile: &str, label: &str) -> String {
    let verdict = match times.verdict() {
        Verdict::WithinTarget => "within",
        Verdict::OverTarget | Verdict::OverCeiling => "over",
    };
    format!(
        "{label}, {width}x{height}, {} frames, {profile}: p50 {}, p99 {}, max {} ({verdict} the {} target)",
        times.samples,
        millis(times.p50),
        millis(times.p99),
        millis(times.max),
        millis(FRAME_TARGET),
    )
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

fn render_porting_snapshot(geometry: Geometry) -> (lunatui_core::Grid, lunatui_core::ClusterTable) {
    use lunatui_core::{ClusterTable, Grid, Rect};
    use lunatui_ratatui::{RatatuiStatefulWidget, RatatuiWidget};
    use lunatui_theme::{Theme, Variant};
    use lunatui_widgets::{Block, RenderCx, Widget};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block as RatBlock, List, ListItem, ListState, Paragraph};

    let width = geometry.width.min(80);
    let height = geometry.height.min(24);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .title("porting guide L2")
        .render(Rect::new(0, 0, width, height), &mut cx);
    let half = width / 2;
    let panel_h = 8.min(height.saturating_sub(2));
    let left_w = half.saturating_sub(1).max(1);
    let right_w = width.saturating_sub(half).saturating_sub(1).max(1);
    let inner_w = width.saturating_sub(2).max(1);
    Block::bordered()
        .title("native")
        .render(Rect::new(1, 1, left_w, panel_h), &mut cx);
    RatatuiWidget::new(RatBlock::bordered().title("ratatui"))
        .render(Rect::new(half, 1, right_w, panel_h), &mut cx);
    let para = Line::from(vec![
        Span::styled("port ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("text", Style::default().fg(Color::Green)),
    ]);
    RatatuiWidget::new(Paragraph::new(para)).render(
        Rect::new(
            1,
            10.min(height.saturating_sub(1)),
            inner_w,
            4.min(height.saturating_sub(10)),
        ),
        &mut cx,
    );
    let list = RatatuiStatefulWidget::new(
        List::new(vec![ListItem::new("one"), ListItem::new("two")]).highlight_symbol("> "),
        ListState::default().with_selected(Some(0)),
    );
    list.render(
        Rect::new(
            1,
            15.min(height.saturating_sub(1)),
            inner_w,
            height.saturating_sub(17).max(1),
        ),
        &mut cx,
    );
    (grid, clusters)
}

fn print_simd_snapshot(_geometry: Geometry) {
    use lunatui_core::simd::{
        ascii_display_width, ascii_display_width_levels_agree, ascii_display_width_scalar,
        differential_levels, first_mismatch, first_mismatch_levels_agree, first_mismatch_scalar,
        runtime_dispatch_level,
    };

    let width = 200_u16;
    let height = 60_u16;
    let len = usize::from(width) * usize::from(height);
    let previous = vec![0_u128; len];
    let mut next = previous.clone();
    for index in (0..len).step_by(97) {
        next[index] = index as u128 + 1;
    }
    let probe = first_mismatch_scalar(&previous, &next);
    let agree = first_mismatch_levels_agree(&previous, &next);
    let dispatched = first_mismatch(&previous, &next);

    let start = Instant::now();
    for _ in 0..256 {
        let _ = first_mismatch(&previous, &next);
    }
    let elapsed = start.elapsed();

    let log_line = "2026-10-16T19:00:00Z INFO  worker completed batch duration=12ms\n";
    let ascii_log = log_line.repeat(5000);
    let ascii_probe = ascii_display_width_scalar(&ascii_log);
    let ascii_agree = ascii_display_width_levels_agree(&ascii_log);
    let ascii_dispatched = ascii_display_width(&ascii_log);
    let ascii_start = Instant::now();
    for _ in 0..256 {
        let _ = ascii_display_width(&ascii_log);
    }
    let ascii_elapsed = ascii_start.elapsed();

    println!("simd runtime={}", runtime_dispatch_level().as_str());
    println!(
        "simd differential_levels={}",
        differential_levels()
            .iter()
            .map(|level| level.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("simd first_mismatch_probe={probe:?} dispatched={dispatched:?}");
    println!("simd levels_agree={agree}");
    println!(
        "simd bench_first_mismatch_{width}x{height}x256={}",
        millis(elapsed)
    );
    println!("simd ascii_width_probe={ascii_probe} dispatched={ascii_dispatched}");
    println!("simd ascii_width_levels_agree={ascii_agree}");
    println!("simd bench_ascii_log_5000x256={}", millis(ascii_elapsed));
}

fn print_porting_snapshot(geometry: Geometry) {
    let (grid, clusters) = render_porting_snapshot(geometry);
    for row in 0..grid.height() {
        println!("{}", grid.row_display(row, &clusters));
    }
}

fn print_ratatui_style_snapshot(geometry: Geometry) {
    use lunatui_core::{ClusterTable, Grid, Rect};
    use lunatui_ratatui::{RatatuiWidget, StyledLine};
    use lunatui_theme::{Theme, Variant};
    use lunatui_widgets::{Block, RenderCx, Widget};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let width = geometry.width.min(80);
    let height = geometry.height.min(24);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .title("ratatui style L2")
        .render(Rect::new(0, 0, width, height), &mut cx);
    let rat_line = Line::from(vec![
        Span::styled(
            "styled ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("plain"),
    ]);
    let _styled: StyledLine = StyledLine::from(&rat_line);
    let inner = Rect::new(2, 2, width.saturating_sub(4), 3);
    RatatuiWidget::new(Paragraph::new(rat_line)).render(inner, &mut cx);
    for row in 0..height {
        println!("{}", grid.row_display(row, &clusters));
    }
}

fn print_ratatui_widget_snapshot(geometry: Geometry) {
    use lunatui_core::{ClusterTable, Grid, Rect};
    use lunatui_ratatui::RatatuiWidget;
    use lunatui_theme::{Theme, Variant};
    use lunatui_widgets::{Block, RenderCx, Widget};
    use ratatui::style::Color;
    use ratatui::widgets::{Gauge, Paragraph};

    let width = geometry.width.min(80);
    let height = geometry.height.min(24);
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    Block::bordered()
        .title("ratatui L2")
        .render(Rect::new(0, 0, width, height), &mut cx);
    let inner = Rect::new(2, 1, width.saturating_sub(4), height.saturating_sub(4));
    RatatuiWidget::new(
        Gauge::default()
            .percent(72)
            .label("72%")
            .gauge_style(ratatui::style::Style::default().fg(Color::Cyan)),
    )
    .render(inner, &mut cx);
    RatatuiWidget::new(Paragraph::new("L2 widget bridge (RatatuiWidget)"))
        .render(Rect::new(2, inner.y + 2, inner.width, 1), &mut cx);
    for row in 0..height {
        println!("{}", grid.row_display(row, &clusters));
    }
}

fn print_ratatui_bridge_snapshot(geometry: Geometry) -> io::Result<()> {
    use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
    use lunatui_ratatui::LunatuiBackend;
    use ratatui::backend::Backend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    let width = geometry.width.min(80);
    let height = geometry.height.min(24);
    let area = Rect::new(0, 0, width, height);
    let mut frame = Buffer::empty(area);
    dashboard::render(&Model::new(), area, &mut frame);
    let mut bytes = Vec::new();
    let mut backend = LunatuiBackend::new(width, height, &mut bytes);
    for y in 0..height {
        for x in 0..width {
            let cell = &frame[(x, y)];
            backend.draw(std::iter::once((x, y, cell)))?;
        }
    }
    backend.flush()?;
    for row in 0..height {
        println!("{}", backend.grid().row_display(row, backend.clusters()));
    }
    println!("encoded_bytes={}", backend.bytes_flushed());
    Ok(())
}

#[cfg(test)]
mod tests {
    use lunatui_core::{Cell, GlyphSlot, Rect};
    use lunatui_runtime::App;
    use lunatui_term::{Event, KeyCode, KeyEvent};
    use lunatui_widgets::Block;

    use super::{Counter, Geometry, Hello, nearest_i16, parse_geometry, render_porting_snapshot};

    #[test]
    fn hello_renders_title_glyph() {
        let mut hello = Hello;
        let (grid, _) = lunatui_runtime::render_once(&mut hello, 40, 10);
        let block = Block::bordered().title("lunatui");
        let inner = block.inner(Rect::new(0, 0, 40, 10));
        assert_eq!(
            grid.get(inner.x, inner.y.saturating_add(1))
                .map(Cell::glyph),
            GlyphSlot::inline_scalar('H')
        );
    }

    #[test]
    fn counter_increments_on_plus() {
        let mut counter = Counter::new(false);
        let plus = Event::Key(KeyEvent::press(KeyCode::Char('+')));
        assert_eq!(
            counter.update(&plus),
            lunatui_runtime::ControlFlow::Continue
        );
        assert_eq!(counter.value, 1);
    }

    #[test]
    fn nearest_i16_rounds_negative_spring_values() {
        assert_eq!(nearest_i16(-0.8), -1);
        assert_eq!(nearest_i16(-0.2), 0);
    }

    #[test]
    fn porting_snapshot_handles_minimal_geometry() {
        let (grid, clusters) = render_porting_snapshot(Geometry {
            width: 1,
            height: 1,
        });
        assert_eq!(grid.width(), 1);
        assert_eq!(grid.height(), 1);
        let row = grid.row_display(0, &clusters);
        assert!(
            row.contains('┘') || row.contains('┌') || row.contains('│') || row.contains('─'),
            "1x1 porting frame should paint block border glyphs, got: {row:?}"
        );
    }

    #[test]
    fn parse_geometry_accepts_explicit_size() -> Result<(), Box<dyn std::error::Error>> {
        let args = [
            "--width".to_string(),
            "80".to_string(),
            "--height".to_string(),
            "24".to_string(),
        ];
        let geometry = parse_geometry(&args)?;
        assert_eq!(geometry.width, 80);
        assert_eq!(geometry.height, 24);
        Ok(())
    }
}
