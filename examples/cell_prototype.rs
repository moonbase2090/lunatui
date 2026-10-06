// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("README.md")]

use std::convert::Infallible;
use std::hint::black_box;
use std::io;
use std::time::Duration;

use lunatui_bench::ceiling::{FrameTimes, sample};
use lunatui_bench::prototype::{PackedCell, Packer, changed_cells, changed_words};
use lunatui_bench::scenes::ratatui::dashboard::Frames;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;

const SAMPLES: usize = 2000;

struct FramePair<'a> {
    name: &'static str,
    previous: &'a Buffer,
    next: &'a Buffer,
    packed_previous: &'a [PackedCell],
    packed_next: &'a [PackedCell],
    words_previous: &'a [u128],
    words_next: &'a [u128],
}

struct Comparison {
    grid: String,
    frames: &'static str,
    changed: usize,
    ratatui: Duration,
    packed: Duration,
    words: Duration,
}

fn main() -> io::Result<()> {
    println!("cell prototype (ADR 0001), {}", build_profile());
    println!(
        "ratatui Cell: {} bytes; packed cell: {} bytes, aligned to {}\n",
        size_of::<Cell>(),
        size_of::<PackedCell>(),
        align_of::<PackedCell>()
    );
    println!("{}", header());
    for (width, height) in [(200_u16, 60_u16), (400, 120)] {
        for comparison in compare(Rect::new(0, 0, width, height))? {
            println!("{}", row(&comparison));
        }
    }
    Ok(())
}

fn compare(area: Rect) -> io::Result<Vec<Comparison>> {
    let frames = Frames::new(area);
    let identical = frames.scroll.clone();
    let mut packer = Packer::new();
    let first = packer.pack(&frames.first);
    let tick = packer.pack(&frames.tick);
    let scroll = packer.pack(&frames.scroll);
    let [first_words, tick_words, scroll_words] = [&first, &tick, &scroll].map(|cells| {
        cells
            .iter()
            .map(|cell| cell.to_bits())
            .collect::<Vec<u128>>()
    });
    let columns = usize::from(area.width);
    let pairs = [
        FramePair {
            name: "identical",
            previous: &frames.scroll,
            next: &identical,
            packed_previous: &scroll,
            packed_next: &scroll,
            words_previous: &scroll_words,
            words_next: &scroll_words,
        },
        FramePair {
            name: "tick",
            previous: &frames.first,
            next: &frames.tick,
            packed_previous: &first,
            packed_next: &tick,
            words_previous: &first_words,
            words_next: &tick_words,
        },
        FramePair {
            name: "scroll",
            previous: &frames.tick,
            next: &frames.scroll,
            packed_previous: &tick,
            packed_next: &scroll,
            words_previous: &tick_words,
            words_next: &scroll_words,
        },
    ];
    pairs
        .into_iter()
        .map(|pair| {
            Ok(Comparison {
                grid: format!("{}x{}", area.width, area.height),
                frames: pair.name,
                changed: changed_cells(pair.packed_previous, pair.packed_next, columns),
                ratatui: median(|| black_box(pair.previous.diff_iter(pair.next).count()))?,
                packed: median(|| {
                    black_box(changed_cells(
                        pair.packed_previous,
                        pair.packed_next,
                        columns,
                    ))
                })?,
                words: median(|| {
                    black_box(changed_words(pair.words_previous, pair.words_next, columns))
                })?,
            })
        })
        .collect()
}

fn median(mut diff: impl FnMut() -> usize) -> io::Result<Duration> {
    let samples = sample(SAMPLES, || {
        diff();
        Ok::<(), Infallible>(())
    });
    let Ok(samples) = samples;
    FrameTimes::from_samples(&samples)
        .map(|times| times.p50)
        .ok_or_else(|| io::Error::other("no samples"))
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug build"
    } else {
        "release build"
    }
}

fn header() -> String {
    format!(
        "{:<9}{:<11}{:>9}{:>14}{:>13}{:>11}{:>9}",
        "grid", "frames", "changed", "ratatui diff", "packed diff", "u128 diff", "speedup"
    )
}

fn row(comparison: &Comparison) -> String {
    format!(
        "{:<9}{:<11}{:>9}{:>14}{:>13}{:>11}{:>8.1}x",
        comparison.grid,
        comparison.frames,
        comparison.changed,
        micros(comparison.ratatui),
        micros(comparison.packed),
        micros(comparison.words),
        comparison.ratatui.as_secs_f64() / comparison.words.as_secs_f64().max(f64::MIN_POSITIVE),
    )
}

fn micros(duration: Duration) -> String {
    format!("{:.2} µs", duration.as_secs_f64() * 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Comparison, header, row};

    #[test]
    fn a_row_shows_counts_times_and_speedup_under_the_header() {
        let comparison = Comparison {
            grid: "200x60".to_owned(),
            frames: "identical",
            changed: 0,
            ratatui: Duration::from_micros(60),
            packed: Duration::from_micros(6),
            words: Duration::from_micros(3),
        };
        assert_eq!(
            header(),
            "grid     frames       changed  ratatui diff  packed diff  u128 diff  speedup"
        );
        assert_eq!(
            row(&comparison),
            "200x60   identical          0      60.00 µs      6.00 µs    3.00 µs    20.0x"
        );
    }
}
