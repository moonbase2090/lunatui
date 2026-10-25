// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_bench::ceiling::{FRAME_TARGET, FrameTimes, Verdict};
use lunatui_bench::scenes::ratatui::dashboard::Model;
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
#[cfg(unix)]
use lunatui_term::{Session, SessionOptions, SyncOutput};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    Block, BorderType, Constraint, Gauge, Paragraph, RenderCx, Spinner, StatusBar, Table, Tabs,
    TextWrap, VecSource, Widget,
};

const SHORT_FRAMES: u32 = 240;
const DEMO_FRAMES: u32 = 1_500;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const DEFAULT_WIDTH: u16 = 80;
const DEFAULT_HEIGHT: u16 = 24;
const TAB_TITLES: [&str; 4] = ["● Overview", "○ Lints", "● Tests", "○ Deps"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Options {
    width: u16,
    height: u16,
    frames: u32,
    snapshot: bool,
    bench: bool,
    demo: bool,
}

fn main() -> io::Result<()> {
    let options = parse_args(&env::args().skip(1).collect::<Vec<_>>())?;
    if options.snapshot {
        let mut model = Model::new();
        advance_model(&mut model, 42);
        let (grid, clusters) = render_grid(&model, options.width, options.height);
        for y in 0..options.height {
            println!("{}", grid.row_display(y, &clusters));
        }
        return Ok(());
    }
    if options.bench {
        let (times, bytes) = run_bench(options)?;
        println!("{}", bench_summary(options, times, bytes));
        return Ok(());
    }
    let times = run_live(options)?;
    println!("{}", live_summary(options, times));
    Ok(())
}

fn parse_args(args: &[String]) -> io::Result<Options> {
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut frames = SHORT_FRAMES;
    let mut snapshot = false;
    let mut bench = false;
    let mut demo = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--snapshot" => snapshot = true,
            "--bench" => bench = true,
            "--demo" => {
                demo = true;
                frames = DEMO_FRAMES;
            }
            "--width" => {
                i += 1;
                width = parse_u16(args.get(i), "--width")?;
            }
            "--height" => {
                i += 1;
                height = parse_u16(args.get(i), "--height")?;
            }
            "--frames" => {
                i += 1;
                frames = u32::from(parse_u16(args.get(i), "--frames")?);
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
        width: width.max(40),
        height: height.max(12),
        frames,
        snapshot,
        bench,
        demo,
    })
}

fn parse_u16(value: Option<&String>, flag: &str) -> io::Result<u16> {
    let text = value.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{flag} needs a value"))
    })?;
    text.parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {flag}")))
}

fn advance_model(model: &mut Model, tick: u32) {
    model.tick = tick;
    model.scroll = usize::try_from(tick / 4 % 300).unwrap_or(0);
}

fn demo_size(base_w: u16, base_h: u16, tick: u32) -> (u16, u16) {
    if (500..1000).contains(&tick) {
        (
            base_w.saturating_add(20).max(60),
            base_h.saturating_add(6).max(18),
        )
    } else {
        (base_w, base_h)
    }
}

fn run_bench(options: Options) -> io::Result<(FrameTimes, usize)> {
    let mut model = Model::new();
    let mut prev = Grid::new(options.width, options.height);
    let mut encoder = Encoder::new();
    let mut samples = Vec::with_capacity(usize::try_from(options.frames).unwrap_or(0));
    let mut output = Vec::new();
    let mut bytes = 0_usize;
    for tick in 0..options.frames {
        advance_model(&mut model, tick);
        let (w, h) = if options.demo {
            demo_size(options.width, options.height, tick)
        } else {
            (options.width, options.height)
        };
        let start = Instant::now();
        let (next, clusters) = render_grid(&model, w, h);
        if next.width() != prev.width() || next.height() != prev.height() {
            prev = Grid::new(w, h);
            encoder = Encoder::new();
        }
        let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
        output.write_all(payload)?;
        bytes = bytes.saturating_add(payload.len());
        samples.push(start.elapsed());
        prev = next;
    }
    let times = FrameTimes::from_samples(&samples)
        .ok_or_else(|| io::Error::other("no frames were drawn"))?;
    Ok((times, bytes))
}

fn run_live(options: Options) -> io::Result<FrameTimes> {
    #[cfg(unix)]
    {
        let mut model = Model::new();
        let mut prev = Grid::new(options.width, options.height);
        let mut encoder = Encoder::new();
        let mut samples = Vec::with_capacity(usize::try_from(options.frames).unwrap_or(0));
        let mut session = Session::enter_with_options(
            io::stdout().lock(),
            SessionOptions {
                sync_output: SyncOutput::Enabled,
                mouse_tracking: lunatui_term::MouseTracking::Disabled,
            },
        )?;
        for tick in 0..options.frames {
            advance_model(&mut model, tick);
            let (w, h) = if options.demo {
                demo_size(options.width, options.height, tick)
            } else {
                (options.width, options.height)
            };
            let start = Instant::now();
            let (next, clusters) = render_grid(&model, w, h);
            if next.width() != prev.width() || next.height() != prev.height() {
                prev = Grid::new(w, h);
                encoder = Encoder::new();
            }
            let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
            session.present_frame(payload)?;
            samples.push(start.elapsed());
            prev = next;
            thread::sleep(FRAME_INTERVAL.saturating_sub(start.elapsed()));
        }
        FrameTimes::from_samples(&samples).ok_or_else(|| io::Error::other("no frames were drawn"))
    }
    #[cfg(not(unix))]
    {
        let _ = options;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lunatui_dashboard live mode requires Unix",
        ))
    }
}

fn render_grid(model: &Model, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    grid.fill(
        Rect::new(0, 0, width, height),
        Style::new().bg(bg),
        ' ',
        &mut clusters,
    );
    paint_rail(
        model,
        Rect::new(0, 0, width, height),
        accent,
        dim,
        bg,
        &mut grid,
        &mut clusters,
    );
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    paint_main(model, Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn paint_rail(
    model: &Model,
    area: Rect,
    accent: Color,
    dim: Color,
    bg: Color,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
) {
    if area.width <= 3 {
        return;
    }
    for y in area.y..area.y + area.height {
        let color = if (u32::from(y) + model.tick).is_multiple_of(12) {
            accent
        } else {
            dim
        };
        grid.put_str(area.x + 1, y, Style::new().fg(color).bg(bg), "│", clusters);
    }
}

fn paint_main(model: &Model, area: Rect, cx: &mut RenderCx<'_>) {
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let text = Style::new().fg(Color::Rgb(0xc0, 0xca, 0xf5)).bg(bg);
    let accent_style = Style::new().fg(accent).bg(bg);
    let dim_style = Style::new().fg(dim).bg(bg);
    let bold = Attrs::default().set_bold(true);
    let cyan = Color::Indexed(14);

    let rail_w = 3_u16;
    if area.width <= rail_w + 10 || area.height < 5 {
        return;
    }
    let main = Rect::new(area.x + rail_w, area.y, area.width - rail_w, area.height);
    let tabs_area = Rect::new(main.x, main.y, main.width, 1);
    let status_area = Rect::new(main.x, main.y + main.height - 1, main.width, 1);
    let body = Rect::new(
        main.x,
        main.y + 1,
        main.width,
        main.height.saturating_sub(2),
    );

    Tabs::new(TAB_TITLES)
        .select(selected_tab(model.tick))
        .style(text)
        .highlight_style(Style::new().fg(accent).bg(bg).with_attrs(bold))
        .render(tabs_area, cx);

    let left_w = (body.width * 60) / 100;
    let gap = 1_u16;
    let left = Rect::new(body.x, body.y, left_w, body.height);
    let right = Rect::new(
        body.x + left_w + gap,
        body.y,
        body.width.saturating_sub(left_w + gap),
        body.height,
    );

    render_checks(model, left, cx);

    if right.width > 0 && right.height > 3 {
        let log_h = right.height.saturating_sub(3);
        let log_area = Rect::new(right.x, right.y, right.width, log_h);
        let gauge_area = Rect::new(right.x, right.y + log_h, right.width, 3);
        let log_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Log ")
            .style(text)
            .border_style(dim_style);
        let log_inner = log_block.inner(log_area);
        log_block.render(log_area, cx);
        Paragraph::new(model.log.as_str())
            .wrap(TextWrap { trim: true })
            .scroll((u16::try_from(model.scroll).unwrap_or(u16::MAX), 0))
            .style(text)
            .render(log_inner, cx);
        Gauge::new(f32::from(u8::try_from(model.tick % 100).unwrap_or(0)) / 100.0)
            .block(
                Block::bordered()
                    .title(" Progress ")
                    .style(text)
                    .border_style(dim_style),
            )
            .gauge_style(accent_style)
            .render(gauge_area, cx);
    }

    StatusBar::new(format!(" scorecard • 3 failing • frame {}", model.tick))
        .style(text)
        .render(status_area, cx);
    Spinner::new(model.tick)
        .style(Style::new().fg(cyan).bg(bg))
        .render(Rect::new(status_area.x, status_area.y, 1, 1), cx);
}

fn render_checks(model: &Model, area: Rect, cx: &mut RenderCx<'_>) {
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let pass = Color::Rgb(0x9e, 0xce, 0x6a);
    let fail = Color::Rgb(0xf7, 0x76, 0x8e);
    let text = Style::new().fg(Color::Rgb(0xc0, 0xca, 0xf5)).bg(bg);
    let bold = Attrs::default().set_bold(true);
    let dim_style = Style::new().fg(dim).bg(bg);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Checks ")
        .style(text)
        .border_style(dim_style);
    let inner = block.inner(area);
    block.render(area, cx);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let header_rows = vec![vec![
        "check".to_owned(),
        "state".to_owned(),
        "time".to_owned(),
        "file".to_owned(),
    ]];
    let cols = vec![
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Fill(1),
    ];
    Table::new(VecSource::from_rows(header_rows), cols)
        .style(text.with_attrs(bold))
        .render(Rect::new(inner.x, inner.y, inner.width, 1), cx);

    let body_y = inner.y.saturating_add(1);
    let body_h = inner.height.saturating_sub(1);
    if body_h == 0 {
        return;
    }
    let mut y = body_y;
    for row in model
        .rows
        .iter()
        .skip(model.scroll)
        .take(usize::from(body_h))
    {
        let state_style = if row[1] == "FAIL" {
            Style::new().fg(fail).bg(bg).with_attrs(bold)
        } else {
            Style::new().fg(pass).bg(bg)
        };
        let cells = [
            (row[0].as_str(), text),
            (row[1].as_str(), state_style),
            (row[2].as_str(), text),
            (row[3].as_str(), text),
        ];
        let widths = [12_u16, 6, 7, inner.width.saturating_sub(12 + 6 + 7)];
        let mut x = inner.x;
        for (i, (cell, style)) in cells.iter().enumerate() {
            let w = widths[i];
            if w == 0 {
                continue;
            }
            let clipped: String = cell.chars().take(usize::from(w)).collect();
            cx.put_str(x, y, &clipped, *style);
            x = x.saturating_add(w);
        }
        y = y.saturating_add(1);
        if y >= inner.y + inner.height {
            break;
        }
    }
}

fn selected_tab(tick: u32) -> usize {
    usize::try_from(tick / 30 % 4).unwrap_or(0)
}

fn live_summary(options: Options, times: FrameTimes) -> String {
    let mode = if options.demo { "demo" } else { "live" };
    format!(
        "lunatui_dashboard {mode}, {}x{}, {} frames, {}: p50 {}, p99 {}, max {} ({})",
        options.width,
        options.height,
        times.samples,
        build_profile(),
        millis(times.p50),
        millis(times.p99),
        millis(times.max),
        target_phrase(times),
    )
}

fn bench_summary(options: Options, times: FrameTimes, bytes: usize) -> String {
    let per_frame = bytes.checked_div(times.samples).unwrap_or(0);
    format!(
        "lunatui_dashboard bench, {}x{}, {} frames, {}: p50 {}, p99 {}, max {}, bytes_total={}, bytes_per_frame≈{} ({})",
        options.width,
        options.height,
        times.samples,
        build_profile(),
        millis(times.p50),
        millis(times.p99),
        millis(times.max),
        bytes,
        per_frame,
        target_phrase(times),
    )
}

fn target_phrase(times: FrameTimes) -> String {
    let verdict = match times.verdict() {
        Verdict::WithinTarget => "within",
        Verdict::OverTarget | Verdict::OverCeiling => "over",
    };
    format!("{verdict} the {} target", millis(FRAME_TARGET))
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    }
}

fn millis(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}
