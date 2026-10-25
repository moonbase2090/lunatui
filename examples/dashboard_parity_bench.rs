// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use lunatui_bench::scenes::ratatui::dashboard::{self as rat_dash, Model};
use lunatui_bench::{AllocStats, CountingAllocator};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Block, BorderType, Constraint, Gauge, RenderCx, Spinner, StatusBar, Table, Tabs, VecSource,
    Widget,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect as RatRect;
use ratatui::{Terminal, TerminalOptions, Viewport};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const TAB_TITLES: [&str; 4] = ["● Overview", "○ Lints", "● Tests", "○ Deps"];

#[derive(Clone, Copy)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
    runs: u32,
}

fn main() -> io::Result<()> {
    let options = parse_args(&env::args().skip(1).collect::<Vec<_>>())?;
    println!(
        "dashboard_parity_bench {}x{} frames={} runs={} ({})",
        options.width,
        options.height,
        options.frames,
        options.runs,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    println!(
        "scope: render + diff/encode + write-to-memory sink; identical Model tick/scroll schedule"
    );
    let mut luna_runs = Vec::new();
    let mut rat_runs = Vec::new();
    for run in 1..=options.runs {
        let luna = bench_lunatui(options);
        let rat = bench_ratatui(options)?;
        println!(
            "run {run}: lunatui median={} p95={} wall_ms={:.1} rss_kib={} bytes/frame≈{} allocs/frame≈{}",
            micros(luna.median),
            micros(luna.p95),
            luna.wall.as_secs_f64() * 1000.0,
            luna.rss_kib,
            luna.bytes_per_frame,
            tenths(luna.allocs_tenths),
        );
        println!(
            "run {run}: ratatui median={} p95={} wall_ms={:.1} rss_kib={} bytes/frame≈{} allocs/frame≈{}",
            micros(rat.median),
            micros(rat.p95),
            rat.wall.as_secs_f64() * 1000.0,
            rat.rss_kib,
            rat.bytes_per_frame,
            tenths(rat.allocs_tenths),
        );
        luna_runs.push(luna);
        rat_runs.push(rat);
    }
    println!("--- aggregate ---");
    print_agg("lunatui", &luna_runs);
    print_agg("ratatui", &rat_runs);
    println!(
        "note: if lunatui loses, hotspots in this path are full-grid fill + widget rebuild + Paragraph/TextView wrap work + per-row table cell writes; ratatui reuses Terminal buffer."
    );
    Ok(())
}

fn parse_args(args: &[String]) -> io::Result<Options> {
    let mut width = 80_u16;
    let mut height = 24_u16;
    let mut frames = 240_u32;
    let mut runs = 5_u32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--width" => {
                i += 1;
                width = parse_num(args.get(i), "--width")?;
            }
            "--height" => {
                i += 1;
                height = parse_num(args.get(i), "--height")?;
            }
            "--frames" => {
                i += 1;
                frames = parse_num(args.get(i), "--frames")?;
            }
            "--runs" => {
                i += 1;
                runs = parse_num(args.get(i), "--runs")?;
            }
            flag => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown flag: {flag}"),
                ));
            }
        }
        i += 1;
    }
    Ok(Options {
        width,
        height,
        frames,
        runs,
    })
}

fn parse_num<T: std::str::FromStr>(value: Option<&String>, flag: &str) -> io::Result<T> {
    value
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
        })?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

#[derive(Clone, Debug)]
struct RunStats {
    median: Duration,
    p95: Duration,
    wall: Duration,
    bytes_per_frame: usize,
    allocs_tenths: u64,
    rss_kib: usize,
}

fn bench_lunatui(options: Options) -> RunStats {
    let mut model = Model::new();
    let theme = Theme::builtin(Variant::Dark);
    let mut prev = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut samples = Vec::new();
    let mut output = Vec::new();
    let mut bytes = 0_usize;
    let wall_start = Instant::now();
    let ((), alloc) = ALLOC.measure(|| {
        for tick in 0..options.frames {
            model.tick = tick;
            model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
            let start = Instant::now();
            paint_lunatui(
                &model,
                &theme,
                &mut next,
                &mut clusters,
                options.width,
                options.height,
            );
            let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
            let _ = output.write_all(payload);
            bytes = bytes.saturating_add(payload.len());
            samples.push(start.elapsed());
            std::mem::swap(&mut prev, &mut next);
        }
    });
    summarize(
        &samples,
        wall_start.elapsed(),
        bytes,
        alloc,
        options.frames,
        rss_kib(),
    )
}

fn bench_ratatui(options: Options) -> io::Result<RunStats> {
    let mut model = Model::new();
    let total = Arc::new(AtomicUsize::new(0));
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(CountSink {
            total: Arc::clone(&total),
        }),
        TerminalOptions {
            viewport: Viewport::Fixed(RatRect::new(0, 0, options.width, options.height)),
        },
    )?;
    let mut samples = Vec::new();
    let wall_start = Instant::now();
    let ((), alloc) = ALLOC.measure(|| {
        for tick in 0..options.frames {
            model.tick = tick;
            model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
            let start = Instant::now();
            let _ = terminal.draw(|frame| {
                rat_dash::render(&model, frame.area(), frame.buffer_mut());
            });
            samples.push(start.elapsed());
        }
    });
    let bytes = total.load(Ordering::Relaxed);
    Ok(summarize(
        &samples,
        wall_start.elapsed(),
        bytes,
        alloc,
        options.frames,
        rss_kib(),
    ))
}

fn summarize(
    samples: &[Duration],
    wall: Duration,
    bytes: usize,
    alloc: AllocStats,
    frames: u32,
    rss_kib: usize,
) -> RunStats {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let median = percentile(&sorted, 50);
    let p95 = percentile(&sorted, 95);
    let frames_usize = usize::try_from(frames).unwrap_or(1).max(1);
    let allocs_tenths = (alloc.allocations.saturating_mul(10)) / frames_usize;
    RunStats {
        median,
        p95,
        wall,
        bytes_per_frame: bytes / frames_usize,
        allocs_tenths: u64::try_from(allocs_tenths).unwrap_or(u64::MAX),
        rss_kib,
    }
}

fn percentile(sorted: &[Duration], pct: usize) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() - 1) * pct) / 100;
    sorted[idx]
}

fn print_agg(label: &str, runs: &[RunStats]) {
    let mut medians: Vec<_> = runs.iter().map(|r| r.median).collect();
    let mut p95s: Vec<_> = runs.iter().map(|r| r.p95).collect();
    medians.sort_unstable();
    p95s.sort_unstable();
    let mid = runs.len() / 2;
    let alloc_sum: u64 = runs.iter().map(|r| r.allocs_tenths).sum();
    let alloc_avg = alloc_sum / u64::try_from(runs.len().max(1)).unwrap_or(1);
    let bytes_avg = runs.iter().map(|r| r.bytes_per_frame).sum::<usize>() / runs.len().max(1);
    let rss_max = runs.iter().map(|r| r.rss_kib).max().unwrap_or(0);
    println!(
        "{label}: across {} runs median-of-medians={} median-of-p95={} avg_bytes/frame≈{} avg_allocs/frame≈{} max_rss_kib={}",
        runs.len(),
        micros(medians[mid]),
        micros(p95s[mid]),
        bytes_avg,
        tenths(alloc_avg),
        rss_max,
    );
}

fn micros(d: Duration) -> String {
    format!("{:.1} µs", d.as_secs_f64() * 1_000_000.0)
}

fn tenths(value: u64) -> String {
    format!("{}.{}", value / 10, value % 10)
}

fn rss_kib() -> usize {
    let pid = std::process::id();
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<usize>()
                .ok()
        })
        .unwrap_or(0)
}

struct CountSink {
    total: Arc<AtomicUsize>,
}

impl Write for CountSink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.total.fetch_add(data.len(), Ordering::Relaxed);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn paint_lunatui(
    model: &Model,
    theme: &Theme,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    width: u16,
    height: u16,
) {
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let text = Style::new().fg(Color::Rgb(0xc0, 0xca, 0xf5)).bg(bg);
    let bold = Attrs::default().set_bold(true);
    grid.fill(
        Rect::new(0, 0, width, height),
        Style::new().bg(bg),
        ' ',
        clusters,
    );
    for y in 0..height {
        let color = if (u32::from(y) + model.tick).is_multiple_of(12) {
            accent
        } else {
            dim
        };
        grid.put_str(1, y, Style::new().fg(color).bg(bg), "│", clusters);
    }
    let mut cx = RenderCx::new(grid, clusters, theme);
    let main = Rect::new(3, 0, width.saturating_sub(3), height);
    Tabs::new(TAB_TITLES)
        .select(usize::try_from(model.tick / 30 % 4).unwrap_or(0))
        .style(text)
        .highlight_style(Style::new().fg(accent).bg(bg).with_attrs(bold))
        .render(Rect::new(main.x, main.y, main.width, 1), &mut cx);
    let body = Rect::new(
        main.x,
        main.y + 1,
        main.width,
        main.height.saturating_sub(2),
    );
    let left_w = (body.width * 60) / 100;
    let left = Rect::new(body.x, body.y, left_w, body.height);
    let right = Rect::new(
        body.x + left_w + 1,
        body.y,
        body.width.saturating_sub(left_w + 1),
        body.height,
    );
    paint_checks(model, left, text, bold, dim, bg, &mut cx);
    paint_side(model, right, text, accent, dim, bg, &mut cx);
    let status = Rect::new(main.x, main.y + main.height - 1, main.width, 1);
    StatusBar::new(format!(" scorecard • 3 failing • frame {}", model.tick))
        .style(text)
        .render(status, &mut cx);
    Spinner::new(model.tick)
        .style(Style::new().fg(Color::Indexed(14)).bg(bg))
        .render(Rect::new(status.x, status.y, 1, 1), &mut cx);
}

fn paint_checks(
    model: &Model,
    left: Rect,
    text: Style,
    bold: Attrs,
    dim: Color,
    bg: Color,
    cx: &mut RenderCx<'_>,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Checks ")
        .style(text)
        .border_style(Style::new().fg(dim).bg(bg));
    let inner = block.inner(left);
    block.render(left, cx);
    let header = vec![vec![
        "check".into(),
        "state".into(),
        "time".into(),
        "file".into(),
    ]];
    Table::new(
        VecSource::from_rows(header),
        vec![
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Fill(1),
        ],
    )
    .style(text.with_attrs(bold))
    .render(Rect::new(inner.x, inner.y, inner.width, 1), cx);
    let pass = Color::Rgb(0x9e, 0xce, 0x6a);
    let fail = Color::Rgb(0xf7, 0x76, 0x8e);
    let mut y = inner.y.saturating_add(1);
    for row in model
        .rows
        .iter()
        .skip(model.scroll)
        .take(usize::from(inner.height.saturating_sub(1)))
    {
        let state_style = if row[1] == "FAIL" {
            Style::new().fg(fail).bg(bg).with_attrs(bold)
        } else {
            Style::new().fg(pass).bg(bg)
        };
        cx.put_str(inner.x, y, &row[0], text);
        cx.put_str(inner.x + 12, y, &row[1], state_style);
        cx.put_str(inner.x + 18, y, &row[2], text);
        cx.put_str(inner.x + 25, y, &row[3], text);
        y = y.saturating_add(1);
    }
}

fn paint_side(
    model: &Model,
    right: Rect,
    text: Style,
    accent: Color,
    dim: Color,
    bg: Color,
    cx: &mut RenderCx<'_>,
) {
    if right.width == 0 || right.height <= 3 {
        return;
    }
    let log_h = right.height.saturating_sub(3);
    let log_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Log ")
        .style(text)
        .border_style(Style::new().fg(dim).bg(bg));
    let log_area = Rect::new(right.x, right.y, right.width, log_h);
    let log_inner = log_block.inner(log_area);
    log_block.render(log_area, cx);
    paint_log_lines(model, log_inner, text, cx);
    let ratio = f32::from(u8::try_from(model.tick % 100).unwrap_or(0)) / 100.0;
    Gauge::new(ratio)
        .block(Block::bordered().title(" Progress ").style(text))
        .gauge_style(Style::new().fg(accent).bg(bg))
        .render(Rect::new(right.x, right.y + log_h, right.width, 3), cx);
}

fn paint_log_lines(model: &Model, area: Rect, style: Style, cx: &mut RenderCx<'_>) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mut y = area.y;
    for line in model
        .log
        .lines()
        .skip(model.scroll)
        .take(usize::from(area.height))
    {
        let clipped: String = line.chars().take(usize::from(area.width)).collect();
        cx.put_str(area.x, y, &clipped, style);
        y = y.saturating_add(1);
        if y >= area.y + area.height {
            break;
        }
    }
}
