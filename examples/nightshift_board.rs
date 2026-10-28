// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_layout::Constraint;
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, Date, Gauge, LineGauge, List, Monthly, Overlay, Paragraph, RenderCx, Spinner,
    SpinnerPreset, StatusBar, Table, VecSource, WeekStart, Widget,
};

struct Options {
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
}

#[derive(Clone, Copy)]
struct Palette {
    surface: Color,
    raised: Color,
    text: Color,
    muted: Color,
    accent: Color,
    focus: Color,
    warning: Color,
    success: Color,
    failure: Color,
}

fn main() -> io::Result<()> {
    let options = parse_options(&env::args().skip(1).collect::<Vec<_>>())?;
    let mut output = io::stdout().lock();
    output.write_all(b"\x1b[?1049h\x1b[?25l")?;

    let result = play(&mut output, &options);

    output.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    output.flush()?;
    result
}

fn play(output: &mut impl Write, options: &Options) -> io::Result<()> {
    let theme = Theme::builtin(Variant::Dark);
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;

    for tick in 0..options.frames {
        let started = Instant::now();
        draw(&mut next, &mut clusters, &theme, tick);
        output.write_all(encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor))?;
        output.flush()?;
        std::mem::swap(&mut previous, &mut next);
        thread::sleep(period.saturating_sub(started.elapsed()));
    }
    Ok(())
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, tick: u32) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let palette = Palette {
        surface: theme.resolve(Token::Surface0),
        raised: theme.resolve(Token::Surface1),
        text: theme.resolve(Token::TextPrimary),
        muted: theme.resolve(Token::TextMuted),
        accent: theme.resolve(Token::Accent),
        focus: theme.resolve(Token::FocusRing),
        warning: theme.resolve(Token::Warn),
        success: Color::Rgb(155, 227, 168),
        failure: theme.resolve(Token::Fail),
    };

    grid.fill(
        area,
        Style::new().fg(palette.text).bg(palette.surface),
        ' ',
        clusters,
    );
    let mut cx = RenderCx::new(grid, clusters, theme);

    let page_style = Style::new().fg(palette.text).bg(palette.surface);
    let page = Block::bordered()
        .title(" NIGHT SHIFT / RELEASE CONTROL ")
        .style(page_style)
        .border_style(Style::new().fg(palette.focus).bg(palette.surface))
        .title_style(Style::new().fg(palette.accent).bg(palette.surface));
    page.render(area, &mut cx);
    if area.width < 100 || area.height < 32 {
        return;
    }

    let width = area.width;
    let height = area.height;
    draw_header(&mut cx, width, tick, palette);
    draw_calendar(&mut cx, tick, palette);
    draw_regions(&mut cx, tick, palette);
    draw_deployments(&mut cx, width, tick, palette);
    draw_metrics(&mut cx, tick, palette);
    draw_events(&mut cx, width, palette);
    draw_footer(&mut cx, width, height, palette);
    draw_overlay(&mut cx, area, width, height, palette);
}

fn draw_header(cx: &mut RenderCx<'_>, width: u16, tick: u32, palette: Palette) {
    StatusBar::new("")
        .style(Style::new().bg(palette.raised))
        .left([
            (
                " PROD ",
                Style::new().fg(palette.surface).bg(palette.success),
            ),
            (
                "  release 2.4.1  ",
                Style::new().fg(palette.text).bg(palette.raised),
            ),
        ])
        .right([
            (
                " us-west-3 ",
                Style::new().fg(palette.text).bg(palette.raised),
            ),
            (
                "  3 alerts ",
                Style::new().fg(palette.surface).bg(palette.warning),
            ),
        ])
        .render(Rect::new(1, 1, width.saturating_sub(2), 1), cx);
    Spinner::new(tick / 4)
        .preset(SpinnerPreset::BARS)
        .style(Style::new().fg(palette.success).bg(palette.raised))
        .render(Rect::new(52, 1, 3, 1), cx);
    Paragraph::new("deploying")
        .style(Style::new().fg(palette.text).bg(palette.raised))
        .render(Rect::new(56, 1, 9, 1), cx);
}

fn draw_calendar(cx: &mut RenderCx<'_>, tick: u32, palette: Palette) {
    let calendar_rect = Rect::new(2, 3, 30, 12);
    let calendar_block = panel(
        cx,
        calendar_rect,
        " TEAM CALENDAR ",
        palette.surface,
        palette.focus,
        palette.accent,
    );
    let selected =
        Date::from_ymd(2026, 10, 26 + ((tick / 24) % 3) as u8).unwrap_or(Date::UNIX_EPOCH);
    let events = [
        (date(2026, 10, 20), Style::new().fg(palette.accent)),
        (date(2026, 10, 22), Style::new().fg(palette.success)),
        (date(2026, 10, 24), Style::new().fg(palette.warning)),
        (date(2026, 10, 29), Style::new().fg(palette.failure)),
    ];
    Monthly::new(selected, &events[..])
        .week_start(WeekStart::Monday)
        .show_weekdays_header(Style::new().fg(palette.muted))
        .weekend_style(Style::new().fg(palette.focus))
        .selected_style(Style::new().fg(palette.surface).bg(palette.accent))
        .render(calendar_block, cx);
}

fn draw_regions(cx: &mut RenderCx<'_>, tick: u32, palette: Palette) {
    let regions_rect = Rect::new(2, 16, 30, 16);
    let regions_block = panel(
        cx,
        regions_rect,
        " ON-CALL REGIONS ",
        palette.surface,
        palette.focus,
        palette.accent,
    );
    let list = List::new(region_source())
        .style(Style::new().fg(palette.text).bg(palette.surface))
        .selected(Some(usize::try_from((tick / 18) % 5).unwrap_or(0)))
        .highlight_style(Style::new().fg(palette.text).bg(palette.raised))
        .highlight_symbol("▌ ", Style::new().fg(palette.accent))
        .icons(vec![
            ("◆".to_owned(), Style::new().fg(palette.success)),
            ("◆".to_owned(), Style::new().fg(palette.accent)),
            ("◆".to_owned(), Style::new().fg(palette.warning)),
            ("◆".to_owned(), Style::new().fg(palette.success)),
            ("◆".to_owned(), Style::new().fg(palette.failure)),
        ])
        .marks(
            vec![
                Some(("OK".to_owned(), Style::new().fg(palette.success))),
                Some(("ACK".to_owned(), Style::new().fg(palette.accent))),
                Some(("WAIT".to_owned(), Style::new().fg(palette.warning))),
                Some(("OK".to_owned(), Style::new().fg(palette.success))),
                Some(("PAGE".to_owned(), Style::new().fg(palette.failure))),
            ],
            1,
        );
    list.render(regions_block, cx);
}

fn draw_deployments(cx: &mut RenderCx<'_>, width: u16, tick: u32, palette: Palette) {
    let table_rect = Rect::new(34, 3, width.saturating_sub(36), 13);
    let table_block = panel(
        cx,
        table_rect,
        " DEPLOYMENT QUEUE ",
        palette.surface,
        palette.focus,
        palette.accent,
    );
    Table::new(
        deployment_source(),
        vec![
            Constraint::Length(23),
            Constraint::Length(15),
            Constraint::Fill(1),
        ],
    )
    .header([
        ("SERVICE", Style::new().fg(palette.muted)),
        ("REGION", Style::new().fg(palette.muted)),
        ("STATUS", Style::new().fg(palette.muted)),
    ])
    .alignments(vec![
        lunatui_widgets::Alignment::Left,
        lunatui_widgets::Alignment::Left,
        lunatui_widgets::Alignment::Right,
    ])
    .selected(Some(usize::try_from((tick / 30) % 4).unwrap_or(0)))
    .highlight_style(Style::new().fg(palette.text).bg(palette.raised))
    .highlight_symbol("▌ ", Style::new().fg(palette.accent))
    .style(Style::new().fg(palette.text).bg(palette.surface))
    .render(table_block, cx);
}

fn draw_metrics(cx: &mut RenderCx<'_>, tick: u32, palette: Palette) {
    let metric_rect = Rect::new(34, 17, 48, 15);
    let metric_block = panel(
        cx,
        metric_rect,
        " LIVE CAPACITY ",
        palette.surface,
        palette.focus,
        palette.accent,
    );
    let metric_width = metric_block.width.saturating_sub(2);
    Paragraph::new("CPU  •  us-west-3")
        .style(Style::new().fg(palette.text).bg(palette.surface))
        .render(
            Rect::new(metric_block.x, metric_block.y, metric_width, 1),
            cx,
        );
    let phase = f32::from(u8::try_from(tick % 120).unwrap_or(0)) / 120.0;
    Gauge::new(0.71 + phase * 0.19)
        .sub_cell(true)
        .gauge_style(Style::new().fg(palette.accent).bg(palette.surface))
        .track_style(Style::new().fg(palette.muted).bg(palette.surface))
        .render(
            Rect::new(
                metric_block.x,
                metric_block.y.saturating_add(1),
                metric_width,
                3,
            ),
            cx,
        );
    Paragraph::new("EDGE  •  canary rollout")
        .style(Style::new().fg(palette.text).bg(palette.surface))
        .render(
            Rect::new(
                metric_block.x,
                metric_block.y.saturating_add(5),
                metric_width,
                1,
            ),
            cx,
        );
    let edge_ratio = 0.42 + f64::from(tick % 80) / 160.0;
    let edge_value = format!("{:.0}%", edge_ratio * 100.0);
    LineGauge::new()
        .label("EDGE")
        .ratio(edge_ratio)
        .value_label(edge_value)
        .style(Style::new().fg(palette.text).bg(palette.surface))
        .filled_style(Style::new().fg(palette.success))
        .unfilled_style(Style::new().fg(palette.muted))
        .render(
            Rect::new(
                metric_block.x,
                metric_block.y.saturating_add(6),
                metric_width,
                1,
            ),
            cx,
        );
    Paragraph::new("P95  42 ms     ERROR RATE  0.02%")
        .style(Style::new().fg(palette.muted).bg(palette.surface))
        .render(
            Rect::new(
                metric_block.x,
                metric_block.y.saturating_add(8),
                metric_width,
                1,
            ),
            cx,
        );
}

fn draw_events(cx: &mut RenderCx<'_>, width: u16, palette: Palette) {
    let incidents_rect = Rect::new(84, 17, width.saturating_sub(86), 15);
    let incidents_block = panel(
        cx,
        incidents_rect,
        " RECENT EVENTS ",
        palette.surface,
        palette.focus,
        palette.accent,
    );
    let event_lines = [
        ("14:01  build queued", palette.text),
        ("14:03  image compiling", palette.muted),
        ("14:05  checks passed", palette.success),
        ("14:07  canary at 10%", palette.warning),
        ("14:09  rollout paused", palette.failure),
    ];
    for (row, (line, color)) in event_lines.into_iter().enumerate() {
        let y = incidents_block
            .y
            .saturating_add(u16::try_from(row).unwrap_or(0));
        Paragraph::new(line)
            .style(Style::new().fg(color).bg(palette.surface))
            .render(
                Rect::new(incidents_block.x, y, incidents_block.width, 1),
                cx,
            );
    }
}

fn draw_footer(cx: &mut RenderCx<'_>, width: u16, height: u16, palette: Palette) {
    let footer_y = height.saturating_sub(2);
    StatusBar::new("")
        .style(Style::new().bg(palette.raised))
        .left([
            (" F1 ", Style::new().fg(palette.surface).bg(palette.focus)),
            (
                " acknowledge   ",
                Style::new().fg(palette.text).bg(palette.raised),
            ),
            ("F2 ", Style::new().fg(palette.surface).bg(palette.focus)),
            (
                " rollback",
                Style::new().fg(palette.text).bg(palette.raised),
            ),
        ])
        .right([(
            " operator: luna ",
            Style::new().fg(palette.text).bg(palette.raised),
        )])
        .render(Rect::new(1, footer_y, width.saturating_sub(2), 1), cx);
}

fn draw_overlay(cx: &mut RenderCx<'_>, area: Rect, width: u16, height: u16, palette: Palette) {
    let modal_x = width.saturating_sub(36);
    let modal_y = height.saturating_sub(13);
    let modal = Rect::new(modal_x, modal_y, 32, 9);
    Overlay::new(
        Block::bordered()
            .title(" APPROVAL REQUIRED ")
            .style(Style::new().fg(palette.text).bg(palette.raised))
            .border_style(Style::new().fg(palette.warning).bg(palette.raised)),
        modal,
    )
    .backdrop(0.24, Color::Rgb(0, 0, 0))
    .shadow(Style::new().bg(Color::Rgb(3, 5, 14)))
    .render(area, cx);
    Paragraph::new("Canary hold: 10% traffic")
        .style(Style::new().fg(palette.text).bg(palette.raised))
        .render(Rect::new(modal.x + 2, modal.y + 2, modal.width - 4, 1), cx);
    Paragraph::new("Approve  •  inspect")
        .style(Style::new().fg(palette.warning).bg(palette.raised))
        .render(Rect::new(modal.x + 2, modal.y + 4, modal.width - 4, 1), cx);
}

fn panel(
    cx: &mut RenderCx<'_>,
    area: Rect,
    title: &str,
    background: Color,
    border: Color,
    title_color: Color,
) -> Rect {
    let block = Block::bordered()
        .title(title)
        .style(Style::new().fg(title_color).bg(background))
        .border_style(Style::new().fg(border).bg(background))
        .title_style(Style::new().fg(title_color).bg(background));
    block.render(area, cx);
    block.inner(area)
}

fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_ymd(year, month, day).unwrap_or(Date::UNIX_EPOCH)
}

fn region_source() -> VecSource {
    let mut source = VecSource::new();
    for name in [
        "us-west-3",
        "eu-central-1",
        "ap-south-2",
        "us-east-1",
        "sa-east-1",
    ] {
        source.push_row(vec![name.to_owned()]);
    }
    source
}

fn deployment_source() -> VecSource {
    let mut source = VecSource::new();
    for row in [
        ["api-gateway", "us-west-3", "healthy"],
        ["image-worker", "eu-central-1", "rolling"],
        ["event-stream", "ap-south-2", "watching"],
        ["auth-service", "us-east-1", "needs approval"],
        ["web-console", "sa-east-1", "healthy"],
    ] {
        source.push_row(row.into_iter().map(str::to_owned).collect());
    }
    source
}

fn parse_options(args: &[String]) -> io::Result<Options> {
    let mut options = Options {
        width: 120,
        height: 36,
        frames: 240,
        fps: 30,
    };
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--snapshot" {
            options.frames = 1;
            index += 1;
            continue;
        }
        let Some(value) = args.get(index + 1) else {
            return Err(invalid(format!("missing value for {}", args[index])));
        };
        match args[index].as_str() {
            "--width" => options.width = parse_u16(value, "width")?,
            "--height" => options.height = parse_u16(value, "height")?,
            "--frames" => options.frames = parse_u32(value, "frames")?,
            "--fps" => options.fps = parse_u32(value, "fps")?,
            flag => return Err(invalid(format!("unknown argument: {flag}"))),
        }
        index += 2;
    }
    if options.width == 0 || options.height == 0 || options.frames == 0 || options.fps == 0 {
        return Err(invalid("width, height, frames and fps must be positive"));
    }
    Ok(options)
}

fn parse_u16(value: &str, name: &str) -> io::Result<u16> {
    value
        .parse()
        .map_err(|_| invalid(format!("{name} must be a positive integer")))
}

fn parse_u32(value: &str, name: &str) -> io::Result<u32> {
    value
        .parse()
        .map_err(|_| invalid(format!("{name} must be a positive integer")))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
