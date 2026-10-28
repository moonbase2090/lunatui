// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, Gauge, Image, ImageFit, ImageSource, Paragraph, RenderCx, StatusBar, Widget, WidgetState,
};

struct Options {
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    snapshot: bool,
}

#[derive(Clone, Copy)]
struct Palette {
    bg: Color,
    raised: Color,
    text: Color,
    muted: Color,
    accent: Color,
    focus: Color,
    success: Color,
}

fn main() -> io::Result<()> {
    let options = parse_options(&env::args().skip(1).collect::<Vec<_>>())?;
    let sky = ImageSource::decode(include_bytes!("../site/assets/sky.webp"))
        .map_err(|error| invalid(error.to_string()))?;
    let moon = ImageSource::decode(include_bytes!("../site/assets/moon.webp"))
        .map_err(|error| invalid(error.to_string()))?;

    if options.snapshot {
        let mut grid = Grid::new(options.width, options.height);
        let mut clusters = ClusterTable::new();
        draw(&mut grid, &mut clusters, 0, &sky, &moon);
        return io::stdout()
            .lock()
            .write_all(grid.display_text(&clusters).as_bytes());
    }

    let mut output = io::stdout().lock();
    output.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let result = play(&mut output, &options, &sky, &moon);
    output.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    output.flush()?;
    result
}

fn play(
    output: &mut impl Write,
    options: &Options,
    sky: &ImageSource,
    moon: &ImageSource,
) -> io::Result<()> {
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;

    for tick in 0..options.frames {
        let started = Instant::now();
        draw(&mut next, &mut clusters, tick, sky, moon);
        output.write_all(encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor))?;
        output.flush()?;
        std::mem::swap(&mut previous, &mut next);
        thread::sleep(period.saturating_sub(started.elapsed()));
    }
    Ok(())
}

fn draw(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    tick: u32,
    sky: &ImageSource,
    moon: &ImageSource,
) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let theme = Theme::builtin(Variant::Dark);
    let palette = Palette {
        bg: theme.resolve(Token::Surface0),
        raised: theme.resolve(Token::Surface1),
        text: theme.resolve(Token::TextPrimary),
        muted: Color::Rgb(132, 145, 160),
        accent: theme.resolve(Token::Accent),
        focus: theme.resolve(Token::FocusRing),
        success: Color::Rgb(155, 227, 168),
    };

    grid.fill(
        area,
        Style::new().fg(palette.text).bg(palette.bg),
        ' ',
        clusters,
    );
    let mut cx = RenderCx::new(grid, clusters, &theme);
    Block::bordered()
        .title(" NIGHT OBSERVATORY / LIVE IMAGING ")
        .style(Style::new().fg(palette.text).bg(palette.bg))
        .border_style(Style::new().fg(palette.focus).bg(palette.bg))
        .title_style(Style::new().fg(palette.accent).bg(palette.bg))
        .render(area, &mut cx);

    let width = area.width;
    let height = area.height;
    if width < 120 || height < 36 {
        return;
    }

    draw_header(&mut cx, width, tick, palette);
    draw_sky(&mut cx, sky, palette);
    draw_telemetry(&mut cx, width, tick, moon, palette);
    draw_footer(&mut cx, width, height, palette);
}

fn draw_header(cx: &mut RenderCx<'_>, width: u16, tick: u32, palette: Palette) {
    let frame_label = format!("FRAME {:04} / LIVE", tick % 10_000);
    StatusBar::new(" SGR-09 ")
        .style(Style::new().fg(palette.bg).bg(palette.accent))
        .render(Rect::new(1, 1, 9, 1), cx);
    StatusBar::new("  deep-sky survey  ")
        .style(Style::new().fg(palette.text).bg(palette.raised))
        .render(Rect::new(10, 1, 24, 1), cx);
    StatusBar::new(frame_label)
        .style(Style::new().fg(palette.success).bg(palette.raised))
        .render(Rect::new(width.saturating_sub(30), 1, 17, 1), cx);
    StatusBar::new(" RX LOCKED ")
        .style(Style::new().fg(palette.bg).bg(palette.success))
        .render(Rect::new(width.saturating_sub(12), 1, 11, 1), cx);
}

fn draw_sky(cx: &mut RenderCx<'_>, sky: &ImageSource, palette: Palette) {
    let image_area = panel(
        cx,
        Rect::new(2, 3, 75, 28),
        " TARGET / NEBULA-04 ",
        palette.bg,
        palette.focus,
        palette.accent,
    );
    Image::new(sky.clone())
        .fit(ImageFit::Cover)
        .render(image_area, cx);
}

fn draw_telemetry(
    cx: &mut RenderCx<'_>,
    width: u16,
    tick: u32,
    moon: &ImageSource,
    palette: Palette,
) {
    let telemetry = panel(
        cx,
        Rect::new(79, 3, width.saturating_sub(81), 28),
        " CAPTURE TELEMETRY ",
        palette.bg,
        palette.accent,
        palette.accent,
    );
    draw_target_summary(cx, telemetry, moon, palette);
    draw_capture_readings(cx, telemetry, tick, palette);
}

fn draw_target_summary(
    cx: &mut RenderCx<'_>,
    telemetry: Rect,
    moon: &ImageSource,
    palette: Palette,
) {
    let inset_x = telemetry.x.saturating_add(1);
    let thumb_area = Rect::new(inset_x, telemetry.y, 15, 10);
    Image::new(moon.clone())
        .fit(ImageFit::Contain)
        .state(WidgetState::Focused)
        .render(thumb_area, cx);

    let reading_x = inset_x.saturating_add(17);
    let reading_width = telemetry.width.saturating_sub(19);
    draw_text(
        cx,
        Rect::new(reading_x, telemetry.y.saturating_add(1), reading_width, 1),
        "TARGET  SGR-09",
        palette.text,
        palette.bg,
    );
    draw_text(
        cx,
        Rect::new(reading_x, telemetry.y.saturating_add(3), reading_width, 1),
        "RA  18h 36m 56s",
        palette.muted,
        palette.bg,
    );
    draw_text(
        cx,
        Rect::new(reading_x, telemetry.y.saturating_add(5), reading_width, 1),
        "DEC -23° 14' 08\"",
        palette.muted,
        palette.bg,
    );
}

fn draw_capture_readings(cx: &mut RenderCx<'_>, telemetry: Rect, tick: u32, palette: Palette) {
    let inset_x = telemetry.x.saturating_add(1);
    let phase = u8::try_from(tick % 100).unwrap_or(0);
    let exposure_ms = 38 + u16::from(phase % 17);
    let drift = phase % 6;
    draw_text(
        cx,
        Rect::new(
            inset_x,
            telemetry.y.saturating_add(11),
            telemetry.width.saturating_sub(2),
            1,
        ),
        "INTEGRATION PROGRESS",
        palette.muted,
        palette.bg,
    );
    let progress = 0.35 + f32::from(phase) / 100.0 * 0.6;
    Gauge::new(progress)
        .gauge_style(Style::new().fg(palette.accent).bg(palette.bg))
        .render(
            Rect::new(
                inset_x,
                telemetry.y.saturating_add(12),
                telemetry.width.saturating_sub(2),
                1,
            ),
            cx,
        );

    let exposure = format!("EXPOSURE    0.{exposure_ms:03}s");
    let drift_text = format!("TRACK DRIFT  {drift}.{} px", phase % 10);
    draw_text(
        cx,
        Rect::new(
            inset_x,
            telemetry.y.saturating_add(15),
            telemetry.width.saturating_sub(2),
            1,
        ),
        &exposure,
        palette.text,
        palette.bg,
    );
    draw_text(
        cx,
        Rect::new(
            inset_x,
            telemetry.y.saturating_add(17),
            telemetry.width.saturating_sub(2),
            1,
        ),
        &drift_text,
        palette.text,
        palette.bg,
    );
    draw_text(
        cx,
        Rect::new(
            inset_x,
            telemetry.y.saturating_add(20),
            telemetry.width.saturating_sub(2),
            1,
        ),
        "FILTER  H-ALPHA   •   GAIN  1.8×",
        palette.muted,
        palette.bg,
    );
}

fn draw_footer(cx: &mut RenderCx<'_>, width: u16, height: u16, palette: Palette) {
    let footer_y = height.saturating_sub(2);
    StatusBar::new(" IMAGE FALLBACK ")
        .style(Style::new().fg(palette.bg).bg(palette.accent))
        .render(Rect::new(1, footer_y, 16, 1), cx);
    StatusBar::new("  2× vertical pixels per cell  ")
        .style(Style::new().fg(palette.text).bg(palette.raised))
        .render(Rect::new(17, footer_y, 30, 1), cx);
    StatusBar::new(" FIELD 02 / 04 ")
        .style(Style::new().fg(palette.text).bg(palette.raised))
        .render(Rect::new(width.saturating_sub(27), footer_y, 15, 1), cx);
    StatusBar::new(" STREAM ")
        .style(Style::new().fg(palette.bg).bg(palette.success))
        .render(Rect::new(width.saturating_sub(12), footer_y, 11, 1), cx);
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

fn draw_text(cx: &mut RenderCx<'_>, area: Rect, content: &str, fg: Color, bg: Color) {
    Paragraph::new(content)
        .style(Style::new().fg(fg).bg(bg))
        .render(area, cx);
}

fn parse_options(args: &[String]) -> io::Result<Options> {
    let mut options = Options {
        width: 120,
        height: 36,
        frames: 240,
        fps: 30,
        snapshot: false,
    };
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--snapshot" {
            options.snapshot = true;
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
