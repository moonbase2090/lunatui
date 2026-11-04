// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use super::MermaidGlyphs;
use super::canvas::{Dir, Glyph, Weight};

const LIGHT: [&str; 16] = [
    " ", "╵", "╶", "└", "╷", "│", "┌", "├", "╴", "┘", "─", "┴", "┐", "┤", "┬", "┼",
];
const ROUND: [&str; 16] = [
    " ", "╵", "╶", "╰", "╷", "│", "╭", "├", "╴", "╯", "─", "┴", "╮", "┤", "┬", "┼",
];
const HEAVY: [&str; 16] = [
    " ", "╹", "╺", "┗", "╻", "┃", "┏", "┣", "╸", "┛", "━", "┻", "┓", "┫", "┳", "╋",
];
const HORIZONTAL: u8 = 10;
const VERTICAL: u8 = 5;

fn unicode_line(mask: u8, weight: Weight, round: bool) -> &'static str {
    let at = usize::from(mask & 15);
    let table = match (weight, mask) {
        (Weight::Thick, _) => &HEAVY,
        (Weight::Dotted, HORIZONTAL) => return "┄",
        (Weight::Dotted, VERTICAL) => return "┆",
        _ if round => &ROUND,
        _ => &LIGHT,
    };
    table.get(at).copied().unwrap_or(" ")
}

fn ascii_line(mask: u8, weight: Weight) -> &'static str {
    let horizontal = mask & (super::canvas::EAST | super::canvas::WEST);
    let vertical = mask & (super::canvas::NORTH | super::canvas::SOUTH);
    match (mask & 15, weight) {
        (0, _) => " ",
        (_, _) if horizontal != 0 && vertical != 0 => "+",
        (_, Weight::Solid) if horizontal != 0 => "-",
        (_, Weight::Solid) => "|",
        (_, Weight::Dotted) if horizontal != 0 => ".",
        (_, Weight::Dotted) => ":",
        (_, Weight::Thick) if horizontal != 0 => "=",
        (_, Weight::Thick) => "#",
    }
}

fn ascii_deco(ch: char) -> char {
    match ch {
        '╱' => '/',
        '╲' => '\\',
        '✕' => 'x',
        '⟩' => ')',
        '↺' => '@',
        other if other.is_ascii() => other,
        _ => '*',
    }
}

pub(super) fn text(glyph: Glyph, mode: MermaidGlyphs, buffer: &mut [u8; 4]) -> &str {
    let ascii = mode == MermaidGlyphs::Ascii;
    match glyph {
        Glyph::Empty | Glyph::Tail | Glyph::Cluster(_) => " ",
        Glyph::Line {
            mask,
            weight,
            round,
            ..
        } => {
            if ascii {
                ascii_line(mask, weight)
            } else {
                unicode_line(mask, weight, round)
            }
        }
        Glyph::Arrow(dir) => match (dir, ascii) {
            (Dir::North, false) => "▲",
            (Dir::East, false) => "▶",
            (Dir::South, false) => "▼",
            (Dir::West, false) => "◀",
            (Dir::North, true) => "^",
            (Dir::East, true) => ">",
            (Dir::South, true) => "v",
            (Dir::West, true) => "<",
        },
        Glyph::Deco(ch) => {
            let ch = if ascii { ascii_deco(ch) } else { ch };
            ch.encode_utf8(buffer)
        }
        Glyph::Text(ch) => ch.encode_utf8(buffer),
    }
}
