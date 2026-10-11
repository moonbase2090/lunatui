// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::io::{self, Write};

use lunatui_core::{Color, char_width, width};
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
    let params = match color {
        Color::Rgb(red, green, blue) => {
            format!("38;2;{red};{green};{blue};48;2;{red};{green};{blue}")
        }
        Color::Indexed(index) => format!("38;5;{index};48;5;{index}"),
        Color::Default => String::from("39;49"),
        Color::Token(id) => {
            let index = id % 256;
            format!("38;5;{index};48;5;{index}")
        }
    };
    format!("\u{1b}[{params}m██\u{1b}[0m")
}

fn display_width(text: &str) -> usize {
    let mut total = 0;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for end in chars.by_ref() {
                if end.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            total += char_width(ch);
        }
    }
    total
}

fn draw(rows: &[String]) -> String {
    let inner = rows.iter().map(|row| display_width(row)).max().unwrap_or(0);
    let mut out = String::new();
    push_border(&mut out, '┌', '┐', inner, true);
    for row in rows {
        let pad = inner - display_width(row);
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
    fn the_four_profiles_paint_swatches_and_the_16_color_row_keeps_its_hues() {
        let frame = picture();
        assert_eq!(frame, GRADIENT_BOX);
        assert!(frame.contains("\u{1b}[38;2;255;0;0;48;2;255;0;0m"));
        assert!(frame.contains("\u{1b}[38;5;208;48;5;208m"));
        assert!(frame.contains("\u{1b}[38;5;4;48;5;4m"));
        assert!(frame.contains("\u{1b}[38;5;0;48;5;0m"));
        assert!(!frame.contains("ff0000"));
    }

    #[test]
    fn the_capture_replays_the_swatches() {
        let cast = include_str!("captures/gradient.cast");
        let event = cast.lines().nth(1).unwrap_or("");
        let raw = event
            .find(r#""o", ""#)
            .map_or("", |start| &event[start + 6..event.len().saturating_sub(2)]);
        assert_eq!(unescape(raw), picture());
    }

    fn unescape(raw: &str) -> String {
        let mut out = String::new();
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                out.push(ch);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('"') => out.push('"'),
                Some('u') => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        hex.push(chars.next().unwrap_or('0'));
                    }
                    let code = u32::from_str_radix(&hex, 16).unwrap_or(0);
                    if let Some(decoded) = char::from_u32(code) {
                        out.push(decoded);
                    }
                }
                Some('\\') | None => out.push('\\'),
                Some(other) => out.push(other),
            }
        }
        out
    }

    const GRADIENT_BOX: &str = "┌ gradient ──────────────────────────┐\n│ truecolor  \u{1b}[38;2;255;0;0;48;2;255;0;0m██\u{1b}[0m \u{1b}[38;2;255;255;0;48;2;255;255;0m██\u{1b}[0m \u{1b}[38;2;0;255;0;48;2;0;255;0m██\u{1b}[0m \u{1b}[38;2;0;255;255;48;2;0;255;255m██\u{1b}[0m \u{1b}[38;2;0;0;255;48;2;0;0;255m██\u{1b}[0m \u{1b}[38;2;255;0;255;48;2;255;0;255m██\u{1b}[0m \u{1b}[38;2;255;128;0;48;2;255;128;0m██\u{1b}[0m \u{1b}[38;2;255;255;255;48;2;255;255;255m██\u{1b}[0m │\n│ ansi256    \u{1b}[38;5;9;48;5;9m██\u{1b}[0m \u{1b}[38;5;11;48;5;11m██\u{1b}[0m \u{1b}[38;5;10;48;5;10m██\u{1b}[0m \u{1b}[38;5;14;48;5;14m██\u{1b}[0m \u{1b}[38;5;21;48;5;21m██\u{1b}[0m \u{1b}[38;5;13;48;5;13m██\u{1b}[0m \u{1b}[38;5;208;48;5;208m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m │\n│ ansi16     \u{1b}[38;5;9;48;5;9m██\u{1b}[0m \u{1b}[38;5;11;48;5;11m██\u{1b}[0m \u{1b}[38;5;10;48;5;10m██\u{1b}[0m \u{1b}[38;5;14;48;5;14m██\u{1b}[0m \u{1b}[38;5;4;48;5;4m██\u{1b}[0m \u{1b}[38;5;13;48;5;13m██\u{1b}[0m \u{1b}[38;5;9;48;5;9m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m │\n│ mono       \u{1b}[38;5;15;48;5;15m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m \u{1b}[38;5;0;48;5;0m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m \u{1b}[38;5;15;48;5;15m██\u{1b}[0m │\n└────────────────────────────────────┘\n";
}
