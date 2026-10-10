// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Style, UnderlineStyle};
use lunatui_term::ColorMode;

use crate::oracle::{Oracle, OracleFailure, apply_diff};

const WIDTH: u16 = 16;
const HEIGHT: u16 = 4;
const GLYPHS: [char; 8] = [' ', 'a', 'b', 'Z', '日', '中', 'A', '-'];

/// Interprets `data` as grid writes, encodes each diff, and checks the vt100 screen.
///
/// Short or empty input is a successful no-op. The function does not panic.
///
/// # Errors
///
/// Returns the first frame whose encoded diff does not match the grid.
pub fn fuzz_bytes(data: &[u8]) -> Result<(), OracleFailure> {
    let mut oracle = Oracle::new(HEIGHT, WIDTH);
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    for chunk in data.chunks(4) {
        let mut op = [0u8; 4];
        op[..chunk.len()].copy_from_slice(chunk);
        let mut next = grid.clone();
        apply_op(&mut next, &mut clusters, op);
        apply_diff(&mut oracle, &grid, &next, &clusters, ColorMode::TrueColor)?;
        grid = next;
    }
    Ok(())
}

/// Runs `execs` stateful encoder frames from a fixed seed.
///
/// # Errors
///
/// Returns the seed window and the oracle mismatch for the first failing frame.
pub fn fuzz_campaign(execs: u64) -> Result<u64, OracleFailure> {
    let mut state = 0x00C0_FFEE_u64;
    let mut done = 0u64;
    while done < execs {
        let window = (execs - done).min(1000);
        let seed = state;
        run_window(seed, window, &mut state).map_err(|failure| failure.with_seed(seed))?;
        done += window;
    }
    Ok(execs)
}

fn run_window(seed: u64, execs: u64, state: &mut u64) -> Result<(), OracleFailure> {
    *state = seed;
    let mut oracle = Oracle::new(HEIGHT, WIDTH);
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut clusters = ClusterTable::new();
    for exec in 0..execs {
        let mut op = [0u8; 4];
        for byte in &mut op {
            *byte = u8::try_from(mix(state) & 0xFF).unwrap_or(0);
        }
        let mut next = grid.clone();
        apply_op(&mut next, &mut clusters, op);
        apply_diff(&mut oracle, &grid, &next, &clusters, ColorMode::TrueColor)
            .map_err(|failure| failure.at_exec(exec))?;
        grid = next;
    }
    Ok(())
}

fn apply_op(grid: &mut Grid, clusters: &mut ClusterTable, op: [u8; 4]) {
    let col = u16::from(op[0]) % WIDTH;
    let row = u16::from(op[1]) % HEIGHT;
    let glyph = GLYPHS[usize::from(op[2]) % GLYPHS.len()];
    let mut text = String::new();
    text.push(glyph);
    grid.put_str(col, row, style_from(op[3]), &text, clusters);
}

fn style_from(byte: u8) -> Style {
    let mut attrs = Attrs::new();
    if byte & 2 != 0 {
        attrs = attrs.set_dim(true);
    } else if byte & 1 != 0 {
        attrs = attrs.set_bold(true);
    }
    if byte & 4 != 0 {
        attrs = attrs.set_italic(true);
    }
    if byte & 8 != 0 {
        attrs = attrs.set_underline(UnderlineStyle::Single);
    }
    if byte & 16 != 0 {
        attrs = attrs.set_reverse(true);
    }
    let background = byte.rotate_left(3);
    Style::new()
        .fg(Color::Indexed(byte))
        .bg(Color::Indexed(background))
        .with_attrs(attrs)
}

fn mix(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    if value == 0 {
        value = 0xA5A5_5A5A_C3C3_3C3C;
    }
    *state = value;
    value
}
