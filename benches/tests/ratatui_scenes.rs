// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::io::{self, BufWriter, LineWriter};

use lunatui_bench::CountingWriter;
use lunatui_bench::scenes::ratatui::dashboard::{self, Frames, Model};
use lunatui_bench::scenes::ratatui::{
    ColorOutput, Encoded, encode, gradient, log, set_color_output, table,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const SMALL: Rect = Rect::new(0, 0, 200, 60);
const LARGE: Rect = Rect::new(0, 0, 400, 120);
const GRADIENT_FRAME_BYTES: usize = 283_430;

fn encoded(changed_cells: usize, bytes: usize, csi_sequences: usize) -> Encoded {
    Encoded {
        changed_cells,
        bytes,
        csi_sequences,
    }
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
}

fn screen_text(buf: &Buffer) -> String {
    (0..buf.area.height).map(|y| row_text(buf, y)).collect()
}

#[test]
fn dashboard_encodes_with_colors_at_200x60() -> io::Result<()> {
    set_color_output(ColorOutput::Enabled);
    let frames = Frames::new(SMALL);
    let mut out = Vec::new();
    assert_eq!(
        encode(&frames.first, &frames.tick, &mut out)?,
        encoded(14, 547, 32)
    );
    assert_eq!(
        encode(&frames.tick, &frames.scroll, &mut out)?,
        encoded(3745, 10_143, 708)
    );
    assert_eq!(
        encode(&frames.empty, &frames.scroll, &mut out)?,
        encoded(12_000, 25_537, 556)
    );
    Ok(())
}

#[test]
fn dashboard_encodes_with_colors_at_400x120() -> io::Result<()> {
    set_color_output(ColorOutput::Enabled);
    let frames = Frames::new(LARGE);
    let mut out = Vec::new();
    assert_eq!(
        encode(&frames.first, &frames.tick, &mut out)?,
        encoded(25, 977, 52)
    );
    assert_eq!(
        encode(&frames.tick, &frames.scroll, &mut out)?,
        encoded(1318, 8847, 810)
    );
    assert_eq!(
        encode(&frames.empty, &frames.scroll, &mut out)?,
        encoded(48_000, 75_485, 1114)
    );
    Ok(())
}

#[test]
fn dashboard_shows_its_panels() {
    let mut buf = Buffer::empty(SMALL);
    dashboard::render(&Model::new(), SMALL, &mut buf);
    let screen = screen_text(&buf);
    for text in [" Checks ", " Log ", " Progress ", "check-0000", "scorecard"] {
        assert!(screen.contains(text), "dashboard is missing {text:?}");
    }
}

#[test]
fn gradient_frame_with_colors_has_a_known_size() -> io::Result<()> {
    set_color_output(ColorOutput::Enabled);
    let frame = gradient::frame(SMALL);
    let mut out = Vec::new();
    let full = encode(&Buffer::empty(SMALL), &frame, &mut out)?;
    assert_eq!(full.bytes, GRADIENT_FRAME_BYTES);
    Ok(())
}

#[test]
fn gradient_frame_with_colors_takes_277_writes_through_a_line_writer_and_1_through_a_large_buf_writer()
-> io::Result<()> {
    set_color_output(ColorOutput::Enabled);
    let frame = gradient::frame(SMALL);
    let empty = Buffer::empty(SMALL);

    let mut line_counted = CountingWriter::new(io::sink());
    let mut backend = CrosstermBackend::new(LineWriter::new(&mut line_counted));
    backend.draw(empty.diff_iter(&frame))?;
    Backend::flush(&mut backend)?;
    drop(backend);

    let mut buf_counted = CountingWriter::new(io::sink());
    let mut backend = CrosstermBackend::new(BufWriter::with_capacity(1 << 20, &mut buf_counted));
    backend.draw(empty.diff_iter(&frame))?;
    Backend::flush(&mut backend)?;
    drop(backend);

    assert_eq!(
        (line_counted.stats().writes, line_counted.stats().bytes),
        (277, GRADIENT_FRAME_BYTES)
    );
    assert_eq!(
        (buf_counted.stats().writes, buf_counted.stats().bytes),
        (1, GRADIENT_FRAME_BYTES)
    );
    Ok(())
}

#[test]
fn unwrapped_log_scrolled_to_4000_starts_at_line_4000() {
    let text = log::text(5000);
    let mut buf = Buffer::empty(LARGE);
    log::render(&text, 4000, log::Layout::Unwrapped, LARGE, &mut buf);
    assert!(row_text(&buf, 0).starts_with("[4000] lint warning"));
}

#[test]
fn log_has_one_line_per_entry() {
    assert_eq!(log::text(5000).lines().count(), 5000);
    assert_eq!(
        log::visible_slice(&log::text(5000), 4000, 120)
            .lines()
            .count(),
        120
    );
}

#[test]
fn table_renders_only_the_rows_it_builds() {
    let rows = table::rows(500);
    let mut buf = Buffer::empty(LARGE);
    table::render(&rows, 118, LARGE, &mut buf);
    assert!(row_text(&buf, 0).starts_with("check-0000"));
    assert!(row_text(&buf, 117).starts_with("check-0117"));
    assert_eq!(row_text(&buf, 118).trim(), "");
}
