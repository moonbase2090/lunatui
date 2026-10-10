// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::io::{self, Write};

use lunatui_core::{Color, width};
use lunatui_theme::Profile;

const RAMP: [[u8; 3]; 8] = [
    [255, 0, 0],
    [255, 255, 0],
    [0, 255, 0],
    [0, 255, 255],
    [0, 0, 255],
    [255, 0, 255],
    [255, 128, 0],
    [255, 255, 255],
];

fn main() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(picture().as_bytes())?;
    stdout.flush()
}

fn picture() -> String {
    let rows = [
        row("truecolor", Profile::TrueColor),
        row("ansi256", Profile::Ansi256),
        row("ansi16", Profile::Ansi16),
        row("mono", Profile::Mono),
    ];
    draw(&rows)
}

fn row(label: &str, profile: Profile) -> String {
    let mut line = format!("{label:<10}");
    for rgb in RAMP {
        line.push(' ');
        line.push_str(&swatch(
            profile.downsample(Color::Rgb(rgb[0], rgb[1], rgb[2])),
        ));
    }
    line
}

fn swatch(color: Color) -> String {
    match color {
        Color::Rgb(red, green, blue) => format!("{red:02x}{green:02x}{blue:02x}"),
        Color::Indexed(index) => format!("{index:>6}"),
        Color::Default => String::from("     d"),
        Color::Token(id) => format!("{id:>6}"),
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
        out.push_str(&" ".repeat(pad));
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
        let label = " gradient ";
        let label_width = width(label);
        out.push_str(label);
        if span > label_width {
            out.push_str(&"─".repeat(span - label_width));
        }
    } else {
        out.push_str(&"─".repeat(span));
    }
    out.push(right);
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::picture;

    #[test]
    fn the_four_profiles_stay_aligned_and_the_16_color_row_keeps_its_hues() {
        assert_eq!(picture(), GRADIENT_BOX);
    }

    const GRADIENT_BOX: &str = "\
┌ gradient ──────────────────────────────────────────────────────────┐
│ truecolor  ff0000 ffff00 00ff00 00ffff 0000ff ff00ff ff8000 ffffff │
│ ansi256         9     11     10     14     21     13    208     15 │
│ ansi16          9     11     10     14      4     13      9     15 │
│ mono           15     15     15     15      0     15     15     15 │
└────────────────────────────────────────────────────────────────────┘
";
}
