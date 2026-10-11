// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::fmt::Write as _;
use std::io::{self, Write};

use lunatui_bench::layout_prototype::{Constraint, Layout, Rect};
use lunatui_core::width;

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(picture().as_bytes())?;
    stdout.flush()
}

fn picture() -> String {
    let rows = [panes(40), panes(80), fills()];
    draw(&rows)
}

fn panes(area: u16) -> String {
    let solved = Layout::horizontal([
        Constraint::Length(10),
        Constraint::Fill(1),
        Constraint::Fill(2),
        Constraint::Length(8),
    ])
    .split(Rect::new(0, 0, area, 1));
    let mut line = format!("width {area:<3}");
    for rect in &solved.segments {
        let _ = write!(line, " {:>2}", rect.width);
    }
    line
}

fn fills() -> String {
    let solved = Layout::horizontal(std::iter::repeat_n(Constraint::Fill(1), 20))
        .split(Rect::new(0, 0, 200, 1));
    let uniform = solved.segments.iter().all(|rect| rect.width == 10);
    if uniform && !solved.needs_solver {
        String::from("20 fill   200 -> 10 each")
    } else {
        String::from("20 fill   solver")
    }
}

fn draw(rows: &[String]) -> String {
    let inner = rows.iter().map(|row| width(row)).max().unwrap_or(0);
    let mut out = String::new();
    push_border(&mut out, '┌', '┐', inner, true);
    for row in rows {
        let pad = inner - width(row);
        out.push('│');
        out.push(' ');
        out.push_str(row);
        for _ in 0..pad {
            out.push(' ');
        }
        out.push(' ');
        out.push('│');
        out.push('\n');
    }
    push_border(&mut out, '└', '┘', inner, false);
    out
}

fn push_border(out: &mut String, left: char, right: char, inner: usize, titled: bool) {
    let span = inner + 2;
    out.push(left);
    if titled {
        let label = " layout ";
        let label_width = width(label);
        out.push_str(label);
        if span > label_width {
            for _ in 0..(span - label_width) {
                out.push('─');
            }
        }
    } else {
        for _ in 0..span {
            out.push('─');
        }
    }
    out.push(right);
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::picture;

    #[test]
    fn resize_keeps_the_fixed_panes_and_splits_the_rest_by_weight() {
        assert_eq!(picture(), LAYOUT_BOX);
    }

    const LAYOUT_BOX: &str = "\
┌ layout ──────────────────┐
│ width 40  10  7 15  8    │
│ width 80  10 21 41  8    │
│ 20 fill   200 -> 10 each │
└──────────────────────────┘
";
}
