// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};

#[test]
fn empty_to_two_ascii_cells_matches_transcript() {
    let mut clusters = ClusterTable::new();
    let prev = Grid::new(2, 1);
    let mut next = Grid::new(2, 1);
    next.put_str(0, 0, Style::new(), "AB", &mut clusters);
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::NoColor);
    assert_eq!(bytes, b"\x1b[1;1HAB\x1b[m\x1b[m\x1b[m\x1b[0m");
}

#[test]
fn style_change_emits_sgr_before_glyph() {
    let mut clusters = ClusterTable::new();
    let prev = Grid::new(1, 1);
    let mut next = Grid::new(1, 1);
    next.put_str(
        0,
        0,
        Style::new().fg(lunatui_core::Color::Indexed(1)),
        "X",
        &mut clusters,
    );
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
    assert!(bytes.starts_with(b"\x1b["));
    assert!(bytes.ends_with(b"X\x1b[m\x1b[m\x1b[m\x1b[0m"));
}

#[test]
fn ascii_run_with_style_changes_and_wide_cell_matches_transcript() {
    let mut clusters = ClusterTable::new();
    let prev = Grid::new(8, 1);
    let mut next = Grid::new(8, 1);
    let red = Style::new().fg(lunatui_core::Color::Indexed(1));
    next.put_str(0, 0, Style::new(), "ab", &mut clusters);
    next.put_str(2, 0, red, "cd", &mut clusters);
    next.put_str(4, 0, red, "\u{65e5}", &mut clusters);
    next.put_str(6, 0, red, "ef", &mut clusters);
    let mut encoder = Encoder::new();
    let bytes = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
    assert_eq!(
        bytes,
        "\x1b[1;1Hab\x1b[38;5;1mcd\u{65e5}ef\x1b[m\x1b[m\x1b[m\x1b[0m".as_bytes()
    );
}

fn encode_row(cells: &[(Style, &str)]) -> Vec<u8> {
    let mut clusters = ClusterTable::new();
    let width = u16::try_from(cells.len()).unwrap_or(u16::MAX);
    let prev = Grid::new(width, 1);
    let mut next = Grid::new(width, 1);
    for (x, (style, text)) in cells.iter().enumerate() {
        let x = u16::try_from(x).unwrap_or(0);
        next.put_str(x, 0, *style, text, &mut clusters);
        next.set_style(Rect::new(x, 0, 1, 1), *style);
    }
    let mut encoder = Encoder::new();
    encoder
        .encode_diff(&prev, &next, &clusters, ColorMode::TrueColor)
        .to_vec()
}

#[test]
fn color_changes_emit_only_the_changed_channel_without_reset() {
    let fg1 = Style::new().fg(Color::Rgb(1, 2, 3)).bg(Color::Rgb(9, 9, 9));
    let fg2 = Style::new().fg(Color::Rgb(4, 5, 6)).bg(Color::Rgb(9, 9, 9));
    let bg2 = Style::new().fg(Color::Rgb(4, 5, 6)).bg(Color::Rgb(7, 7, 7));
    let bytes = encode_row(&[(fg1, "a"), (fg2, "b"), (bg2, "c")]);
    assert_eq!(
        bytes,
        b"\x1b[1;1H\x1b[38;2;1;2;3;48;2;9;9;9ma\x1b[38;2;4;5;6mb\x1b[48;2;7;7;7mc\x1b[m\x1b[m\x1b[m\x1b[0m"
    );
}

#[test]
fn color_returning_to_default_emits_default_code() {
    let red = Style::new().fg(Color::Indexed(1));
    let bytes = encode_row(&[(red, "a"), (Style::new(), "b")]);
    assert_eq!(
        bytes,
        b"\x1b[1;1H\x1b[38;5;1ma\x1b[39mb\x1b[m\x1b[m\x1b[m\x1b[0m"
    );
}

#[test]
fn attribute_turning_off_resets_and_restores_colors() {
    let bold_red = Style::new()
        .fg(Color::Indexed(1))
        .with_attrs(Attrs::new().set_bold(true));
    let red = Style::new().fg(Color::Indexed(1));
    let bytes = encode_row(&[(bold_red, "a"), (red, "b")]);
    assert_eq!(
        bytes,
        b"\x1b[1;1H\x1b[1;38;5;1ma\x1b[0;38;5;1mb\x1b[m\x1b[m\x1b[m\x1b[0m"
    );
}

#[test]
fn attribute_turning_on_keeps_colors_and_adds_only_the_new_code() {
    let red = Style::new().fg(Color::Indexed(1));
    let bold_red = red.with_attrs(Attrs::new().set_bold(true));
    let bytes = encode_row(&[(red, "a"), (bold_red, "b")]);
    assert_eq!(
        bytes,
        b"\x1b[1;1H\x1b[38;5;1ma\x1b[1mb\x1b[m\x1b[m\x1b[m\x1b[0m"
    );
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Sim {
    fg: Option<String>,
    bg: Option<String>,
    attrs: u8,
}

/// Applies the SGR params to `sim`; supports the codes the encoder emits for this test's styles.
fn apply_sgr(sim: &mut Sim, params: &str) -> bool {
    let mut parts = params.split(';').peekable();
    while let Some(part) = parts.next() {
        match part {
            "" | "0" => *sim = Sim::default(),
            "1" => sim.attrs |= 1,
            "2" => sim.attrs |= 2,
            "3" => sim.attrs |= 4,
            "4" => sim.attrs |= 8,
            "7" => sim.attrs |= 16,
            "39" => sim.fg = None,
            "49" => sim.bg = None,
            "38" | "48" => {
                let mode = parts.next().unwrap_or("");
                let count = if mode == "2" { 3 } else { 1 };
                let mut color = mode.to_owned();
                for _ in 0..count {
                    color.push(';');
                    color.push_str(parts.next().unwrap_or(""));
                }
                if part == "38" {
                    sim.fg = Some(color);
                } else {
                    sim.bg = Some(color);
                }
            }
            _ => return false,
        }
    }
    true
}

fn sim_for(style: Style) -> Sim {
    let color = |c: Color| match c {
        Color::Default | Color::Token(_) => None,
        Color::Indexed(i) => Some(format!("5;{i}")),
        Color::Rgb(r, g, b) => Some(format!("2;{r};{g};{b}")),
    };
    let a = style.attrs;
    let attrs = u8::from(a.bold())
        | u8::from(a.dim()) << 1
        | u8::from(a.italic()) << 2
        | u8::from(a.underline() != lunatui_core::UnderlineStyle::None) << 3
        | u8::from(a.reverse()) << 4;
    Sim {
        fg: color(style.fg),
        bg: color(style.bg),
        attrs,
    }
}

#[test]
fn random_style_runs_decode_to_the_requested_styles() {
    let colors = [
        Color::Default,
        Color::Indexed(1),
        Color::Indexed(14),
        Color::Rgb(1, 2, 3),
        Color::Rgb(26, 27, 38),
    ];
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..500 {
        let cells: Vec<Style> = (0..24)
            .map(|_| {
                let bits = next();
                let attrs = Attrs::new()
                    .set_bold(bits & 1 != 0)
                    .set_dim(bits & 2 != 0)
                    .set_italic(bits & 4 != 0)
                    .set_reverse(bits & 8 != 0)
                    .set_underline(if bits & 16 != 0 {
                        lunatui_core::UnderlineStyle::Single
                    } else {
                        lunatui_core::UnderlineStyle::None
                    });
                let pick = |n: u64| colors[usize::try_from(n % 5).unwrap_or(0)];
                Style::new()
                    .fg(pick(bits >> 8))
                    .bg(pick(bits >> 16))
                    .with_attrs(attrs)
            })
            .collect();
        let row: Vec<(Style, &str)> = cells.iter().map(|s| (*s, "x")).collect();
        let bytes = encode_row(&row);
        let text = String::from_utf8(bytes).unwrap_or_default();
        let mut sim = Sim::default();
        let mut printed = Vec::new();
        let mut rest = text.as_str();
        while let Some(ch) = rest.chars().next() {
            if let Some(seq) = rest.strip_prefix("\x1b[") {
                let end = seq.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(0);
                if seq.as_bytes()[end] == b'm' {
                    assert!(
                        apply_sgr(&mut sim, &seq[..end]),
                        "unsupported sgr in {seq:?}"
                    );
                }
                rest = &seq[end + 1..];
            } else {
                if ch == 'x' {
                    printed.push(sim.clone());
                }
                rest = &rest[ch.len_utf8()..];
            }
        }
        // The trailing resets come after the last glyph, so `printed` holds one state per cell.
        let want: Vec<Sim> = cells.iter().map(|s| sim_for(*s)).collect();
        assert_eq!(printed, want);
    }
}
