// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_bench::CountingAllocator;
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn paint(grid: &mut Grid, clusters: &mut ClusterTable, phase: u16) {
    let base = Style::new().bg(Color::Rgb(26, 27, 38));
    grid.fill(Rect::new(0, 0, 60, 12), base, ' ', clusters);
    for row in 0..12_u16 {
        let shade = u8::try_from((row * 20 + phase * 7) % 256).unwrap_or(0);
        let style = base.fg(Color::Rgb(shade, 200, 255 - shade));
        grid.put_str(2 + phase % 5, row, style, "diff and encode", clusters);
        let indexed = base.fg(Color::Indexed(u8::try_from(row + phase).unwrap_or(0)));
        grid.put_str(30, row, indexed, "indexed", clusters);
    }
    let bold = base.fg(Color::Indexed(1));
    grid.put_str(40, 3, bold, "bold", clusters);
    grid.set_style(
        Rect::new(40, 3, 4, 1),
        bold.with_attrs(Attrs::default().set_bold(true)),
    );
    grid.put_str(1, 11, base.fg(Color::Default), "│", clusters);
}

const TRAILER: &[u8] = b"\x1b[m\x1b[m\x1b[m\x1b[0m";

fn screen_of(grid: &Grid, clusters: &ClusterTable) -> Vec<Vec<char>> {
    (0..grid.height())
        .map(|row| grid.row_display(row, clusters).chars().collect())
        .collect()
}

fn replay(output: &[u8], screen: &mut [Vec<char>]) -> Result<(), String> {
    let text = std::str::from_utf8(output).map_err(|error| error.to_string())?;
    let (mut row, mut col) = (0_usize, 0_usize);
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            let cell = screen
                .get_mut(row)
                .and_then(|line| line.get_mut(col))
                .ok_or_else(|| format!("write outside the screen at {row},{col}"))?;
            *cell = ch;
            col += 1;
            continue;
        }
        if chars.next() != Some('[') {
            return Err("escape without CSI".to_owned());
        }
        let mut params = String::new();
        let mut last = ' ';
        for next in chars.by_ref() {
            if next.is_ascii_alphabetic() {
                last = next;
                break;
            }
            params.push(next);
        }
        if last == 'H' {
            let mut parts = params
                .split(';')
                .map(|part| part.parse::<usize>().unwrap_or(1));
            row = parts.next().unwrap_or(1).saturating_sub(1);
            col = parts.next().unwrap_or(1).saturating_sub(1);
        } else if last != 'm' {
            return Err(format!("unexpected sequence {params}{last}"));
        }
    }
    Ok(())
}

fn encode_checked(
    encoder: &mut Encoder,
    previous: &Grid,
    next: &Grid,
    clusters: &ClusterTable,
    expect_changes: bool,
) -> Result<(), String> {
    let output = encoder
        .encode_diff(previous, next, clusters, ColorMode::TrueColor)
        .to_vec();
    if !output.ends_with(TRAILER) {
        return Err("output lacks the reset trailer".to_owned());
    }
    if expect_changes && output.len() <= TRAILER.len() {
        return Err("a changed frame produced only the trailer".to_owned());
    }
    if !expect_changes && output.len() != TRAILER.len() {
        return Err("an unchanged frame produced more than the trailer".to_owned());
    }
    let mut screen = screen_of(previous, clusters);
    replay(&output, &mut screen)?;
    if screen != screen_of(next, clusters) {
        return Err("replaying the output does not reproduce the frame".to_owned());
    }
    Ok(())
}

fn measured_allocations(
    encoder: &mut Encoder,
    previous: &Grid,
    next: &Grid,
    clusters: &ClusterTable,
) -> usize {
    let (_, stats) = ALLOC.measure(|| {
        encoder
            .encode_diff(previous, next, clusters, ColorMode::TrueColor)
            .len()
    });
    stats.allocations
}

#[test]
fn steady_state_encoding_allocates_nothing_and_still_draws() -> Result<(), String> {
    let mut clusters = ClusterTable::new();
    let mut previous = Grid::new(60, 12);
    let mut next = Grid::new(60, 12);
    let mut encoder = Encoder::new();
    for phase in 0..40 {
        paint(&mut next, &mut clusters, phase);
        encode_checked(&mut encoder, &previous, &next, &clusters, true)?;
        std::mem::swap(&mut previous, &mut next);
    }
    let mut allocations = 0;
    for phase in 40..80 {
        paint(&mut next, &mut clusters, phase);
        allocations += measured_allocations(&mut encoder, &previous, &next, &clusters);
        encode_checked(&mut encoder, &previous, &next, &clusters, true)?;
        std::mem::swap(&mut previous, &mut next);
    }
    for _ in 0..5 {
        paint(&mut next, &mut clusters, 79);
        allocations += measured_allocations(&mut encoder, &previous, &next, &clusters);
        encode_checked(&mut encoder, &previous, &next, &clusters, false)?;
        std::mem::swap(&mut previous, &mut next);
    }
    if allocations == 0 {
        Ok(())
    } else {
        Err(format!(
            "encode_diff allocated {allocations} times in steady state"
        ))
    }
}
