// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::iter;

use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};
use lunatui_core::width;

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const LINES: [&str; 3] = [
    "Hello from lunatui",
    "PR 04: Unicode width tables",
    "日本語 한국어 🦀 café",
];

fn main() -> io::Result<()> {
    let show_metrics = env::args().skip(1).any(|arg| arg == "--metrics");

    let mut stdout = CountingWriter::new(io::stdout().lock());
    let (drawn, alloc) = ALLOC.measure(|| {
        stdout.write_all(render_box(&LINES).as_bytes())?;
        stdout.flush()
    });
    drawn?;

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

fn render_box(lines: &[&str]) -> String {
    let text_width = lines.iter().map(|line| width(line)).max().unwrap_or(0);
    let border = "─".repeat(text_width + 2);

    let top = format!("┌{border}┐\n");
    let rows = lines
        .iter()
        .map(|line| format!("│ {line}{} │\n", " ".repeat(text_width - width(line))));
    let bottom = format!("└{border}┘\n");

    iter::once(top)
        .chain(rows)
        .chain(iter::once(bottom))
        .collect()
}

#[cfg(test)]
mod tests {
    use lunatui_bench::{AllocStats, WriteStats};

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
        let expected = format!(
            "┌{border}┐\n│ Hello from lunatui{} │\n│ PR 04: Unicode width tables │\n│ 日本語 한국어 🦀 café{} │\n└{border}┘\n",
            " ".repeat(9),
            " ".repeat(6),
            border = "─".repeat(29),
        );
        assert_eq!(render_box(&LINES), expected);
    }
}
