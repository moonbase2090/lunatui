// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Grid, Rect, Size, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{Block, Padding, Paragraph, RenderCx, Spinner, SpinnerPreset, Widget};

const LABEL_COLUMNS: u16 = 22;

struct Options {
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    snapshot: bool,
}

fn label(preset: SpinnerPreset) -> String {
    let ms = preset.interval().as_millis();
    format!("{:<12}{ms:>4} ms", preset.name())
}

fn frame_index(preset: SpinnerPreset, tick: u32, fps: u32) -> u32 {
    let fps = u64::from(fps.max(1));
    let elapsed = u64::from(tick).saturating_mul(1_000) / fps;
    let interval = u64::try_from(preset.interval().as_millis()).unwrap_or(u64::MAX);
    let interval = interval.max(1);
    u32::try_from(elapsed / interval).unwrap_or(u32::MAX)
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, tick: u32, fps: u32) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    let text = cx.token(Token::TextPrimary);
    let accent = cx.token(Token::Accent);
    let surface = cx.token(Token::Surface0);
    let ground = Style::new().fg(text).bg(surface);
    let frame = Block::bordered()
        .title(" presets ")
        .style(ground)
        .border_style(Style::new().fg(accent).bg(surface))
        .title_style(Style::new().fg(accent).bg(surface))
        .padding(Padding {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        });
    frame.render(area, &mut cx);
    let inner = frame.inner(area);
    let spin_style = Style::new().fg(accent).bg(surface);
    for (index, preset) in SpinnerPreset::ALL.iter().enumerate() {
        let Ok(row) = u16::try_from(index) else {
            break;
        };
        let y = inner.y.saturating_add(row);
        if y >= inner.y.saturating_add(inner.height) || inner.width == 0 {
            break;
        }
        Paragraph::new(label(*preset)).style(ground).render(
            Rect::new(inner.x, y, LABEL_COLUMNS.min(inner.width), 1),
            &mut cx,
        );
        let origin = inner.x.saturating_add(LABEL_COLUMNS);
        if origin >= inner.x.saturating_add(inner.width) {
            continue;
        }
        let room = inner.x.saturating_add(inner.width).saturating_sub(origin);
        let spinner = Spinner::new(frame_index(*preset, tick, fps))
            .preset(*preset)
            .style(spin_style);
        let columns = spinner.measure(Size::new(room, 1)).width;
        if columns == 0 {
            continue;
        }
        spinner.render(Rect::new(origin, y, columns, 1), &mut cx);
    }
}

fn snapshot(options: &Options) -> io::Result<()> {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    draw(&mut grid, &mut clusters, &theme, 0, options.fps);
    let mut out = io::stdout().lock();
    out.write_all(grid.display_text(&clusters).as_bytes())?;
    Ok(())
}

fn play(out: &mut impl Write, options: &Options) -> io::Result<()> {
    let theme = Theme::builtin(Variant::Dark);
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;
    for tick in 0..options.frames {
        let started = Instant::now();
        draw(&mut next, &mut clusters, &theme, tick, options.fps);
        out.write_all(encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor))?;
        out.flush()?;
        std::mem::swap(&mut previous, &mut next);
        thread::sleep(period.saturating_sub(started.elapsed()));
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let options = parse(&env::args().skip(1).collect::<Vec<_>>())?;
    if options.snapshot {
        return snapshot(&options);
    }
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let result = play(&mut out, &options);
    out.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
    result
}

fn parse(args: &[String]) -> io::Result<Options> {
    let mut options = Options {
        width: 64,
        height: 12,
        frames: 48,
        fps: 12,
        snapshot: false,
    };
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag == "--snapshot" {
            options.snapshot = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| invalid(format!("{flag} needs a value")))?;
        match flag {
            "--width" => options.width = positive(value, flag)?,
            "--height" => options.height = positive(value, flag)?,
            "--frames" => options.frames = positive(value, flag)?,
            "--fps" => options.fps = positive(value, flag)?,
            _ => return Err(invalid(format!("unknown flag: {flag}"))),
        }
        index += 2;
    }
    Ok(options)
}

fn positive<T>(value: &str, flag: &str) -> io::Result<T>
where
    T: std::str::FromStr + Default + PartialEq,
{
    let parsed = value
        .parse()
        .map_err(|_| invalid(format!("invalid {flag}")))?;
    if parsed == T::default() {
        return Err(invalid(format!("{flag} must be greater than zero")));
    }
    Ok(parsed)
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::{draw, frame_index};
    use lunatui_core::{ClusterTable, Grid};
    use lunatui_theme::{Theme, Variant};
    use lunatui_widgets::SpinnerPreset;

    fn screen(tick: u32) -> String {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = Grid::new(64, 12);
        let mut clusters = ClusterTable::new();
        draw(&mut grid, &mut clusters, &theme, tick, 10);
        grid.display_text(&clusters)
    }

    fn row_with<'a>(screen: &'a str, name: &str) -> &'a str {
        screen
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or("")
    }

    #[test]
    fn the_screen_names_every_preset() {
        let frame = screen(0);
        for preset in SpinnerPreset::ALL {
            assert!(frame.contains(preset.name()), "{}", preset.name());
        }
        let dots = row_with(&frame, "dots");
        assert!(dots.contains('⠋'));
        assert!(dots.contains("80 ms"));
        let line = row_with(&frame, "line");
        assert!(line.contains('|'));
        let later = screen(2);
        let line = row_with(&later, "line");
        assert!(line.contains('/'));
        assert!(!line.contains('|'));
        let next = screen(1);
        let bounce_now = row_with(&frame, "bounce");
        let bounce_next = row_with(&next, "bounce");
        assert!(bounce_now.contains('●'));
        assert!(bounce_next.contains('●'));
        assert_ne!(bounce_now, bounce_next);
        assert_eq!(frame_index(SpinnerPreset::LINE, 0, 10), 0);
        assert_eq!(frame_index(SpinnerPreset::LINE, 2, 10), 1);
    }
}
