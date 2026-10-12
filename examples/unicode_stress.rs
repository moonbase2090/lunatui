// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal, Write};

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder, Session, SessionOptions, SyncOutput};

const WIDTH: u16 = 72;
const HEIGHT: u16 = 22;

fn main() -> io::Result<()> {
    let show_session = env::args().any(|arg| arg == "--session");
    let mut clusters = ClusterTable::new();
    let grid = render_stress_grid(&mut clusters);
    if show_session {
        present_session(&grid, &clusters)?;
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(grid.display_text(&clusters).as_bytes())?;
        stdout.flush()?;
    }
    Ok(())
}

fn render_stress_grid(clusters: &mut ClusterTable) -> Grid {
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let label = Style::new().fg(Color::Rgb(0x7a, 0xa2, 0xf7));
    let body = Style::new();
    let edge = Style::new().fg(Color::Rgb(0xf7, 0x7a, 0xa2));

    grid.fill(
        Rect {
            x: 0,
            y: 0,
            width: WIDTH,
            height: HEIGHT,
        },
        Style::new().bg(Color::Rgb(0x1a, 0x1b, 0x26)),
        ' ',
        clusters,
    );

    let rows: [(&str, &str); 10] = [
        ("ZWJ family", "👩\u{200D}💻 👨\u{200D}👩\u{200D}👧 🧑\u{200D}🎨"),
        ("VS16 color", "❤\u{FE0F} ☀\u{FE0F} ⚡\u{FE0F}"),
        ("flags", "🇺🇸 🇯🇵 🇧🇷"),
        ("combining", "e\u{301} cafe\u{301} na\u{EF}ve"),
        ("wide edge L", "日本語日本語日本語日本語日本語"),
        ("wide edge R", "xxxx日本"),
        ("split wide", "abc🦀def"),
        ("rtl mark", "\u{200F}ABC"),
        ("emoji skin", "👍🏽 👋🏻"),
        ("empty width", "a\u{200B}b"),
    ];

    grid.put_str(1, 0, label, "unicode stress (Grid)", clusters);
    for (index, (title, sample)) in rows.iter().enumerate() {
        let y = u16::try_from(index + 2).unwrap_or(u16::MAX);
        grid.put_str(1, y, label, title, clusters);
        let title_len = lunatui_core::width(title) + 2;
        let start = 1 + u16::try_from(title_len).unwrap_or(0);
        if start < WIDTH - 2 {
            grid.put_str(start, y, body, sample, clusters);
        }
        grid.put_str(WIDTH - 2, y, edge, "│", clusters);
    }

    let border_y = HEIGHT - 2;
    grid.put_str(0, border_y, edge, "└", clusters);
    grid.fill(
        Rect {
            x: 1,
            y: border_y,
            width: WIDTH - 2,
            height: 1,
        },
        edge,
        '─',
        clusters,
    );
    grid.put_str(WIDTH - 1, border_y, edge, "┘", clusters);
    grid
}

#[cfg(unix)]
fn present_session(grid: &Grid, clusters: &ClusterTable) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "unicode_stress --session requires a terminal",
        ));
    }
    let prev = Grid::new(grid.width(), grid.height());
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, grid, clusters, ColorMode::TrueColor);
    let stdout = io::stdout();
    let mut session = Session::enter_with_options(
        stdout,
        SessionOptions {
            sync_output: SyncOutput::Enabled,
        },
    )?;
    session.present_frame(bytes)?;
    Ok(())
}

#[cfg(not(unix))]
fn present_session(_grid: &Grid, _clusters: &ClusterTable) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "unicode_stress --session requires Unix",
    ))
}

#[cfg(test)]
mod tests {
    use lunatui_core::ClusterTable;

    use super::{HEIGHT, WIDTH, render_stress_grid};

    #[test]
    fn stress_grid_contains_zwj_and_flags() {
        let mut clusters = ClusterTable::new();
        let grid = render_stress_grid(&mut clusters);
        let text = grid.display_text(&clusters);
        assert!(text.contains("👩\u{200D}💻"));
        assert!(text.contains("🇺🇸"));
        assert_eq!(grid.width(), WIDTH);
        assert_eq!(grid.height(), HEIGHT);
    }
}
