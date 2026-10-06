// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `hello_box`: draws a bordered box with text (PRD M13, PR 01).
//!
//! lunatui has no grid, encoder or `Block` widget yet, so this example builds
//! the box as a plain string and prints it. Later PRs move it onto lunatui's
//! own pipeline (PRD §8, Phase 0 and Phase 1 Demo lines).
//!
//! Run with `cargo run --example hello_box`.

use std::io::{self, Write};
use std::iter;

const LINES: [&str; 2] = ["Hello from lunatui", "PR 01: workspace scaffold"];

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(render_box(&LINES).as_bytes())?;
    stdout.flush()
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
    use super::{LINES, render_box};

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
