// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};

use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};
use lunatui_core::{
    Cell, CellWidth, ClusterTable, Color, GlyphSlot, Grid, Meta, Rect, Style, width,
};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const LINES: [&str; 4] = [
    "Hello from lunatui",
    "PR 07: hello_box on Grid",
    "日本語 한국어 🦀 café",
    "👩\u{200D}💻 ❤\u{FE0F} 🇺🇸",
];

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let show_metrics = args.iter().any(|arg| arg == "--metrics");
    let show_cells = args.iter().any(|arg| arg == "--cells");

    let mut stdout = CountingWriter::new(io::stdout().lock());
    let (drawn, alloc) = ALLOC.measure(|| {
        stdout.write_all(render_box(&LINES).as_bytes())?;
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
    fn the_demo_box_keeps_its_right_border_aligned() {
        let text_width = LINES.iter().map(|line| width(line)).max().unwrap_or(0);
        let border = "─".repeat(text_width + 2);
        let pad = |line: &str| " ".repeat(text_width - width(line));
        let expected = format!(
            "┌{border}┐\n│ Hello from lunatui{pad0} │\n│ PR 07: hello_box on Grid{pad1} │\n│ 日本語 한국어 🦀 café{pad2} │\n│ 👩\u{200D}💻 ❤\u{FE0F} 🇺🇸{pad3} │\n└{border}┘\n",
            pad0 = pad(LINES[0]),
            pad1 = pad(LINES[1]),
            pad2 = pad(LINES[2]),
            pad3 = pad(LINES[3]),
        );
        assert_eq!(render_box(&LINES), expected);
    }
}
