// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style, diff_runs};
use lunatui_term::{ColorMode, Encoder, Session, SessionOptions, SyncOutput};

const STORM_SIZES: [(u16, u16); 4] = [(12, 4), (200, 60), (40, 8), (120, 40)];

static RESIZE_COUNT: AtomicU32 = AtomicU32::new(0);

fn main() -> io::Result<()> {
    let storm = env::args().any(|arg| arg == "--storm");
    let frames = parse_frames(env::args().skip(1).collect())?;
    #[cfg(unix)]
    {
        if !io::stdout().is_terminal() {
            return run_headless(frames);
        }
        return run_session(storm, frames);
    }
    #[cfg(not(unix))]
    {
        let _ = (storm, frames);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "resize_torture requires Unix",
        ))
    }
}

fn parse_frames(args: Vec<String>) -> io::Result<u32> {
    let mut frames = 120_u32;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--frames" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "--frames needs a value")
            })?;
            frames = value
                .parse()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid --frames"))?;
        }
        index += 1;
    }
    Ok(frames)
}

#[cfg(unix)]
fn run_headless(frames: u32) -> io::Result<()> {
    let mut width = 80_u16;
    let mut height = 24_u16;
    let mut prev = Grid::new(width, height);
    let mut next = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    for frame in 0..frames {
        if frame.is_multiple_of(8) {
            let (w, h) = STORM_SIZES[usize::try_from(frame / 8 % 4).unwrap_or(0)];
            width = w;
            height = h;
            prev = Grid::new(width, height);
            next = Grid::new(width, height);
            RESIZE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        render_size(&mut next, &mut clusters, width, height, frame);
        let _runs = diff_runs(&prev, &next);
        let _payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
        prev = next;
        next = Grid::new(width, height);
    }
    writeln_summary(frames, true)?;
    Ok(())
}

#[cfg(unix)]
fn run_session(storm: bool, frames: u32) -> io::Result<()> {
    if storm {
        thread::spawn(move || {
            for index in 0..frames {
                if index.is_multiple_of(4) {
                    let (width, height) =
                        STORM_SIZES[usize::try_from(index / 4 % 4).unwrap_or(0)];
                    set_tty_size(width, height);
                    RESIZE_COUNT.fetch_add(1, Ordering::Relaxed);
                }
                thread::sleep(Duration::from_millis(30));
            }
        });
    }

    let stdout = io::stdout();
    let mut session = Session::enter_with_options(
        stdout,
        SessionOptions {
            sync_output: SyncOutput::Enabled,
        },
    )?;
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut prev = Grid::new(80, 24);
    let mut next = Grid::new(80, 24);
    let mut width = 80_u16;
    let mut height = 24_u16;

    for frame in 0..frames {
        if let Some((w, h)) = read_tty_size() {
            if w != width || h != height {
                width = w;
                height = h;
                prev = Grid::new(width, height);
                next = Grid::new(width, height);
                RESIZE_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }
        render_size(&mut next, &mut clusters, width, height, frame);
        let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
        session.present_frame(payload)?;
        prev = next;
        next = Grid::new(width, height);
        thread::sleep(Duration::from_millis(16));
    }
    writeln_summary(frames, storm)?;
    Ok(())
}

fn render_size(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    width: u16,
    height: u16,
    frame: u32,
) {
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    grid.fill(
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        },
        Style::new().bg(bg),
        ' ',
        clusters,
    );
    let label = format!("resize torture  {width}x{height}  frame {frame}");
    grid.put_str(1, 1, Style::new().fg(Color::Rgb(0x7a, 0xa2, 0xf7)).bg(bg), &label, clusters);
    let hint = "SIGWINCH storms: run with --storm (Unix tty)";
    if height > 3 {
        grid.put_str(1, height - 2, Style::new().fg(Color::Rgb(0x56, 0x5f, 0x89)).bg(bg), hint, clusters);
    }
}

fn writeln_summary(frames: u32, storm: bool) -> io::Result<()> {
    let resizes = RESIZE_COUNT.load(Ordering::Relaxed);
    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "resize_torture: {frames} frames, {resizes} resizes, storm={storm}"
    )?;
    stdout.flush()?;
    Ok(())
}

#[cfg(unix)]
fn read_tty_size() -> Option<(u16, u16)> {
    use std::os::fd::AsRawFd;

    let stdout = io::stdout();
    let mut winsize = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: ioctl reads geometry when stdout is a tty.
    let ok = unsafe {
        libc::ioctl(stdout.as_raw_fd(), libc::TIOCGWINSZ, &raw mut winsize) == 0
    };
    if !ok || winsize.ws_col == 0 || winsize.ws_row == 0 {
        return None;
    }
    Some((winsize.ws_col, winsize.ws_row))
}

#[cfg(unix)]
fn set_tty_size(width: u16, height: u16) {
    use std::os::fd::AsRawFd;

    let stdout = io::stdout();
    let winsize = libc::winsize {
        ws_row: height,
        ws_col: width,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: ioctl sets geometry on the controlling tty when permitted.
    unsafe {
        libc::ioctl(stdout.as_raw_fd(), libc::TIOCSWINSZ, &winsize);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_frames;

    #[test]
    fn parse_frames_defaults() {
        assert_eq!(parse_frames(vec![]).expect("parse"), 120);
    }

    #[test]
    fn parse_frames_reads_flag() {
        assert_eq!(
            parse_frames(vec!["--frames".into(), "5".into()]).expect("parse"),
            5
        );
    }
}
