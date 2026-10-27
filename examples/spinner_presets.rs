// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{ClusterTable, Grid};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Variant};

#[path = "deploy_board.rs"]
mod deploy_board;

struct Options {
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    snapshot: bool,
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, tick: u32, fps: u32) {
    let fps = u64::from(fps.max(1));
    let elapsed = u64::from(tick).saturating_mul(1_000) / fps;
    deploy_board::draw(grid, clusters, theme, elapsed);
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
    use super::deploy_board::frame_index;
    use super::draw;
    use lunatui_core::{ClusterTable, Grid};
    use lunatui_theme::{Theme, Variant};
    use lunatui_widgets::SpinnerPreset;

    fn at(preset: SpinnerPreset, tick: u32, fps: u32) -> u32 {
        let fps = u64::from(fps.max(1));
        let elapsed = u64::from(tick).saturating_mul(1_000) / fps;
        frame_index(preset, elapsed)
    }

    fn screen(width: u16, height: u16, tick: u32) -> String {
        let theme = Theme::builtin(Variant::Dark);
        let mut grid = Grid::new(width, height);
        let mut clusters = ClusterTable::new();
        draw(&mut grid, &mut clusters, &theme, tick, 10);
        grid.display_text(&clusters)
    }

    fn row_with<'a>(screen: &'a str, needle: &str) -> &'a str {
        screen
            .lines()
            .find(|line| line.contains(needle))
            .unwrap_or("")
    }

    #[test]
    fn the_board_is_a_deploy_screen() {
        let frame = screen(120, 36, 0);
        assert!(frame.contains("production deploy"));
        assert!(frame.contains("release 2.4.1"));
        assert!(frame.contains("compiling workspace"));
        assert!(frame.contains("418 passed"));
        assert!(frame.contains("10 percent of traffic"));
        assert!(frame.contains("layer 4 of 8"));
        assert!(frame.contains("revision 48"));
        assert!(frame.contains("62 regions"));
        assert!(frame.contains("draining connections"));
        assert!(frame.contains("job 19"));
        assert!(frame.contains("site sync"));
        assert!(frame.contains("3 regions left"));
        assert!(!frame.contains("80 ms"));
        assert!(!frame.contains("dots"));
        let checks = row_with(&frame, "418 passed");
        assert!(checks.contains('|'));
        let later = screen(120, 36, 2);
        let checks_later = row_with(&later, "418 passed");
        assert!(checks_later.contains('/'));
        let migrate = row_with(&frame, "migrate");
        let next = screen(120, 36, 1);
        let migrate_next = row_with(&next, "migrate");
        assert!(migrate.contains('●'));
        assert!(migrate_next.contains('●'));
        assert_ne!(migrate, migrate_next);
        assert!(!frame.contains('█'));
        assert!(screen(120, 36, 40).contains('█'));
        assert!(screen(20, 8, 0).contains("release"));
        assert_eq!(at(SpinnerPreset::LINE, 0, 10), 0);
        assert_eq!(at(SpinnerPreset::LINE, 2, 10), 1);
    }
}
