// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Print coverage masks for each startup glyph tier and smooth bar heads.

use lunatui_core::subcell::{GlyphSet, horizontal_eighths, vertical_eighths};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut out = io::stdout().lock();
    for tier in [
        GlyphSet::Octants,
        GlyphSet::Braille,
        GlyphSet::Blocks,
        GlyphSet::Ascii,
    ] {
        writeln!(out, "{tier:?}")?;
        for row in 0_u8..16 {
            for col in 0_u8..16 {
                write!(out, "{}", tier.glyph(row * 16 + col))?;
            }
            writeln!(out)?;
        }
    }
    write!(out, "Horizontal:")?;
    for step in 0..=8 {
        write!(out, "{}", horizontal_eighths(step))?;
    }
    write!(out, "\nVertical:")?;
    for step in 0..=8 {
        write!(out, "{}", vertical_eighths(step))?;
    }
    writeln!(out)
}
