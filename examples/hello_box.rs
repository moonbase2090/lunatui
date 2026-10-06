// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hello_box`: draws a bordered box with text (PRD M13, PR 01).
//!
//! lunatui has no grid, encoder or `Block` widget yet, so this example builds
//! the box as a plain string and prints it. Later PRs move it onto lunatui's
//! own pipeline (PRD §8, Phase 0 and Phase 1 Demo lines).
//!
//! With `--metrics` it also prints what drawing the box cost, measured with
//! the bench harness counters (PR 02): allocations, `write()` calls and bytes.
//!
//! Run with `cargo run --example hello_box`, or
//! `cargo run --example hello_box -- --metrics`.

use std::env;
use std::io::{self, Write};
use std::iter;

use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

const LINES: [&str; 2] = ["Hello from lunatui", "PR 02: bench harness"];

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

/// Describes the cost of drawing the box, one line per counter.
fn metrics_lines(_alloc: AllocStats, _write: WriteStats) -> [String; 3] {
    [String::new(), String::new(), String::new()]
}

/// Renders `lines` inside a single-line border, padded by one space on each side.
///
/// Widths are counted in `char`s, which is correct for the ASCII text drawn
/// here. Display widths for wide and multi-codepoint graphemes arrive with the
/// width tables (PR 04) and grapheme slot (PR 05).
fn render_box(lines: &[&str]) -> String {
    let text_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let border = "─".repeat(text_width + 2);

    let top = format!("┌{border}┐\n");
    let rows = lines
        .iter()
        .map(|line| format!("│ {line:<text_width$} │\n"));
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
    fn demo_rows_all_have_the_same_width() {
        let rendered = render_box(&LINES);
        let widths: Vec<usize> = rendered.lines().map(|row| row.chars().count()).collect();
        assert_eq!(widths.len(), LINES.len() + 2);
        assert!(widths.iter().all(|&width| width == widths[0]));
    }
}
