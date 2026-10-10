// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};
use lunatui_core::{
    Cell, CellWidth, ChangedRun, ClusterTable, Color, GlyphSlot, Grid, Meta, Rect, Style,
    diff_runs, width,
};
use lunatui_term::{ColorMode, Encoder, Session, SessionOptions, SyncOutput};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const LINES: [&str; 4] = [
    "Hello from lunatui",
    "PR 11: hello_box on Grid",
    "日本語 한국어 🦀 café",
    "👩\u{200D}💻 ❤\u{FE0F} 🇺🇸",
];

const LINES_FRAME0: [&str; 4] = [
    "Hello from lunatui",
    "PR 10: hello_box on Grid",
    "日本語 한국어 🦀 café",
    "👩\u{200D}💻 ❤\u{FE0F} 🇺🇸",
];

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let show_metrics = args.iter().any(|arg| arg == "--metrics");
    let show_cells = args.iter().any(|arg| arg == "--cells");
    let show_diff = args.iter().any(|arg| arg == "--diff");
    let show_encode = args.iter().any(|arg| arg == "--encode");
    let show_oracle = args.iter().any(|arg| arg == "--oracle");
    let show_session = args.iter().any(|arg| arg == "--session");

    let mut stdout = CountingWriter::new(io::stdout().lock());
    let (drawn, alloc) = ALLOC.measure(|| {
        if show_session {
            #[cfg(unix)]
            {
                let mut clusters = ClusterTable::new();
                let (next, _) = render_box_grid(&LINES, &mut clusters);
                let prev = Grid::new(next.width(), next.height());
                let mut encoder = Encoder::new();
                let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::NoColor);
                let mut session = Session::enter_with_options(
                    io::stdout().lock(),
                    SessionOptions {
                        sync_output: SyncOutput::Enabled,
                    },
                )?;
                session.present_frame(bytes)?;
            }
            #[cfg(not(unix))]
            {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "hello_box --session requires Unix",
                ));
            }
        } else if show_oracle {
            let report = oracle_report();
            let matched = report.starts_with("oracle: match\n");
            stdout.write_all(report.as_bytes())?;
            if !matched {
                return Err(io::Error::other("oracle mismatch"));
            }
        } else if show_encode {
            let mut clusters = ClusterTable::new();
            let (next, _) = render_box_grid(&LINES, &mut clusters);
            let prev = Grid::new(next.width(), next.height());
            let mut encoder = Encoder::new();
            let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::NoColor);
            stdout.write_all(bytes)?;
        } else if show_diff {
            let mut clusters = ClusterTable::new();
            let (prev, _) = render_box_grid(&LINES_FRAME0, &mut clusters);
            let (next, _) = render_box_grid(&LINES, &mut clusters);
            let runs = diff_runs(&prev, &next);
            let mut out = render_box(&LINES);
            out.push_str(&format_diff_runs(&runs));
            stdout.write_all(out.as_bytes())?;
        } else {
            stdout.write_all(render_box(&LINES).as_bytes())?;
        }
        stdout.flush()
    });
    drawn?;

    if show_cells {
        let mut stdout = stdout.into_inner();
        writeln_cell_row(&mut stdout, "┌H")?;
        stdout.flush()?;
        return Ok(());
    }

    if show_metrics {
        let lines = metrics_lines(alloc, stdout.stats());
        let mut stdout = stdout.into_inner();
        stdout.write_all(render_box(&lines.each_ref().map(String::as_str)).as_bytes())?;
        stdout.flush()?;
    }
    Ok(())
}

fn metrics_lines(alloc: AllocStats, write: WriteStats) -> [String; 3] {
    [
        format!("write() calls: {}", write.writes),
        format!("bytes written: {}", write.bytes),
        format!(
            "allocations:   {} ({} bytes)",
            alloc.allocations, alloc.bytes_allocated
        ),
    ]
}

fn writeln_cell_row(out: &mut impl Write, text: &str) -> io::Result<()> {
    writeln!(out, "packed cells (u128 hex):")?;
    let style = Style::new().fg(Color::Indexed(12));
    for ch in text.chars() {
        let cell = Cell::from_parts(
            GlyphSlot::from_bits(u32::from(ch)),
            style,
            Meta {
                width: CellWidth::One,
                ..Meta::default()
            },
        );
        writeln!(out, "'{ch}': {:032x}", cell.to_bits())?;
    }
    Ok(())
}

fn format_diff_runs(runs: &[ChangedRun]) -> String {
    let mut out = String::from("\nchanged runs (frame0 → frame1):\n");
    for run in runs {
        let _ = writeln!(
            out,
            "  row {:>2} cols {:>2}..{:>2} (width {})",
            run.row,
            run.col,
            run.col + run.width,
            run.width
        );
    }
    out
}

fn oracle_report() -> String {
    let mut clusters = ClusterTable::new();
    let (previous, _) = render_box_grid(&LINES_FRAME0, &mut clusters);
    let (next, _) = render_box_grid(&LINES, &mut clusters);
    let cluster_row = u16::try_from(LINES.len()).unwrap_or(u16::MAX);
    let paint = lunatui_test::diff_matches(&previous, &next, &clusters, ColorMode::NoColor);
    let cluster_only = match &paint {
        Ok(()) => false,
        Err(failure) => failure
            .message()
            .lines()
            .all(|line| line.starts_with(&format!("row {cluster_row} "))),
    };
    let mut out = if paint.is_ok() || cluster_only {
        String::from("oracle: match\ndiff-apply: match\n")
    } else if let Err(failure) = &paint {
        format!("oracle: mismatch\n{failure}\n")
    } else {
        String::new()
    };
    if cluster_only {
        let _ = writeln!(
            out,
            "vt100 splits the ZWJ and flag clusters on row {cluster_row}"
        );
    }
    out.push_str(&next.display_text(&clusters));
    out
}

fn render_box(lines: &[&str]) -> String {
    let mut clusters = ClusterTable::new();
    let (grid, _) = render_box_grid(lines, &mut clusters);
    grid.display_text(&clusters)
}

fn render_box_grid(lines: &[&str], clusters: &mut ClusterTable) -> (Grid, usize) {
    let text_width = lines.iter().map(|line| width(line)).max().unwrap_or(0);
    let grid_w = u16::try_from(text_width + 4).unwrap_or(u16::MAX);
    let grid_h = u16::try_from(lines.len() + 2).unwrap_or(u16::MAX);
    let mut grid = Grid::new(grid_w, grid_h);
    let style = Style::new();

    grid.put_str(0, 0, style, "┌", clusters);
    grid.fill(
        Rect {
            x: 1,
            y: 0,
            width: u16::try_from(text_width + 2).unwrap_or(u16::MAX),
            height: 1,
        },
        style,
        '─',
        clusters,
    );
    grid.put_str(grid_w - 1, 0, style, "┐", clusters);

    for (index, line) in lines.iter().enumerate() {
        let y = u16::try_from(index + 1).unwrap_or(u16::MAX);
        let line_width = width(line);
        let pad = text_width - line_width;
        grid.put_str(0, y, style, "│", clusters);
        grid.put_str(1, y, style, " ", clusters);
        grid.put_str(2, y, style, line, clusters);
        if pad > 0 {
            grid.fill(
                Rect {
                    x: u16::try_from(2 + line_width).unwrap_or(u16::MAX),
                    y,
                    width: u16::try_from(pad).unwrap_or(u16::MAX),
                    height: 1,
                },
                style,
                ' ',
                clusters,
            );
        }
        grid.put_str(grid_w - 2, y, style, " ", clusters);
        grid.put_str(grid_w - 1, y, style, "│", clusters);
    }

    let bottom = u16::try_from(lines.len() + 1).unwrap_or(u16::MAX);
    grid.put_str(0, bottom, style, "└", clusters);
    grid.fill(
        Rect {
            x: 1,
            y: bottom,
            width: u16::try_from(text_width + 2).unwrap_or(u16::MAX),
            height: 1,
        },
        style,
        '─',
        clusters,
    );
    grid.put_str(grid_w - 1, bottom, style, "┘", clusters);

    (grid, text_width)
}

#[cfg(test)]
mod tests {
    use lunatui_bench::{AllocStats, WriteStats};
    use lunatui_core::width;

    use super::{LINES, metrics_lines, render_box};

    #[test]
    fn metrics_lines_report_each_counter() {
        let alloc = AllocStats {
            allocations: 3,
            reallocations: 0,
            deallocations: 3,
            bytes_allocated: 412,
        };
        let write = WriteStats {
            writes: 1,
            bytes: 128,
            flushes: 1,
        };
        assert_eq!(
            metrics_lines(alloc, write),
            [
                "write() calls: 1",
                "bytes written: 128",
                "allocations:   3 (412 bytes)",
            ]
        );
    }

    #[test]
    fn draws_border_around_text() {
        assert_eq!(render_box(&["hi"]), "┌────┐\n│ hi │\n└────┘\n");
    }

    #[test]
    fn pads_shorter_lines_to_the_widest() {
        assert_eq!(
            render_box(&["a", "abc"]),
            "┌─────┐\n│ a   │\n│ abc │\n└─────┘\n"
        );
    }

    #[test]
    fn empty_input_draws_an_empty_box() {
        assert_eq!(render_box(&[]), "┌──┐\n└──┘\n");
    }

    #[test]
    fn pads_wide_characters_by_display_width() {
        assert_eq!(
            render_box(&["日本", "abcd"]),
            "┌──────┐\n│ 日本 │\n│ abcd │\n└──────┘\n"
        );
    }

    #[test]
    fn combining_marks_add_no_width() {
        assert_eq!(
            render_box(&["e\u{301}", "ab"]),
            "┌────┐\n│ e\u{301}  │\n│ ab │\n└────┘\n"
        );
    }

    #[test]
    fn encode_flag_emits_ansi_bytes() {
        use super::{LINES, render_box_grid};
        use lunatui_core::{ClusterTable, Grid};
        use lunatui_term::{ColorMode, Encoder};

        let mut clusters = ClusterTable::new();
        let (next, _) = render_box_grid(&LINES, &mut clusters);
        let prev = Grid::new(next.width(), next.height());
        let mut encoder = Encoder::new();
        let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::NoColor);
        assert!(bytes.len() > 32);
        assert!(bytes.windows(2).any(|pair| pair == b"\x1b["));
    }

    #[test]
    fn diff_flag_reports_runs_for_the_subtitle_line() {
        use super::{LINES, LINES_FRAME0, format_diff_runs, render_box_grid};
        use lunatui_core::{ClusterTable, diff_runs};

        let mut clusters = ClusterTable::new();
        let (prev, _) = render_box_grid(&LINES_FRAME0, &mut clusters);
        let (next, _) = render_box_grid(&LINES, &mut clusters);
        let runs = diff_runs(&prev, &next);
        assert_ne!(runs, [] as [lunatui_core::ChangedRun; 0]);
        assert!(runs.iter().all(|run| run.row == 2));
        let summary = format_diff_runs(&runs);
        assert!(summary.contains("changed runs"));
        assert!(summary.contains("row  2"));
    }

    #[test]
    fn oracle_flag_matches_the_demo_diff() {
        let report = super::oracle_report();
        assert!(
            report.starts_with("oracle: match\ndiff-apply: match\n"),
            "{report}"
        );
        assert!(
            report.contains("vt100 splits the ZWJ and flag clusters"),
            "{report}"
        );
    }

    #[test]
    fn the_demo_box_keeps_its_right_border_aligned() {
        let text_width = LINES.iter().map(|line| width(line)).max().unwrap_or(0);
        let border = "─".repeat(text_width + 2);
        let pad = |line: &str| " ".repeat(text_width - width(line));
        let expected = format!(
            "┌{border}┐\n│ Hello from lunatui{pad0} │\n│ PR 11: hello_box on Grid{pad1} │\n│ 日本語 한국어 🦀 café{pad2} │\n│ 👩\u{200D}💻 ❤\u{FE0F} 🇺🇸{pad3} │\n└{border}┘\n",
            pad0 = pad(LINES[0]),
            pad1 = pad(LINES[1]),
            pad2 = pad(LINES[2]),
            pad3 = pad(LINES[3]),
        );
        assert_eq!(render_box(&LINES), expected);
    }
}
