// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::io::{self, BufWriter, LineWriter};

use lunatui_bench::CountingWriter;
use lunatui_bench::scenes::ratatui::dashboard::Frames;
use lunatui_bench::scenes::ratatui::{ColorOutput, Encoded, encode, gradient, set_color_output};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const SMALL: Rect = Rect::new(0, 0, 200, 60);
const LARGE: Rect = Rect::new(0, 0, 400, 120);
const MEASURED_GRADIENT_FRAME_BYTES: usize = 68_424;

fn encoded(changed_cells: usize, bytes: usize, csi_sequences: usize) -> Encoded {
    Encoded {
        changed_cells,
        bytes,
        csi_sequences,
    }
}

fn counted_writes(
    writer: impl FnOnce(&mut CountingWriter<io::Sink>) -> io::Result<()>,
) -> io::Result<(usize, usize)> {
    let mut counted = CountingWriter::new(io::sink());
    writer(&mut counted)?;
    Ok((counted.stats().writes, counted.stats().bytes))
}

#[test]
fn dashboard_reproduces_the_measured_encoder_output_at_200x60() -> io::Result<()> {
    set_color_output(ColorOutput::Disabled);
    let frames = Frames::new(SMALL);
    let mut out = Vec::new();
    assert_eq!(
        encode(&frames.first, &frames.tick, &mut out)?,
        encoded(14, 209, 32)
    );
    assert_eq!(
        encode(&frames.tick, &frames.scroll, &mut out)?,
        encoded(3745, 9612, 708)
    );
    assert_eq!(
        encode(&frames.empty, &frames.scroll, &mut out)?,
        encoded(12_000, 16_086, 556)
    );
    Ok(())
}

#[test]
fn dashboard_reproduces_the_measured_encoder_output_at_400x120() -> io::Result<()> {
    set_color_output(ColorOutput::Disabled);
    let frames = Frames::new(LARGE);
    let mut out = Vec::new();
    assert_eq!(
        encode(&frames.first, &frames.tick, &mut out)?,
        encoded(25, 359, 52)
    );
    assert_eq!(
        encode(&frames.tick, &frames.scroll, &mut out)?,
        encoded(1318, 7704, 810)
    );
    assert_eq!(
        encode(&frames.empty, &frames.scroll, &mut out)?,
        encoded(48_000, 56_424, 1114)
    );
    Ok(())
}

#[test]
fn gradient_reproduces_the_measured_writes_and_bytes() -> io::Result<()> {
    set_color_output(ColorOutput::Disabled);
    let frame = gradient::frame(SMALL);
    let empty = Buffer::empty(SMALL);
    let through_line_writer = counted_writes(|counted| {
        let mut backend = CrosstermBackend::new(LineWriter::new(counted));
        backend.draw(empty.diff_iter(&frame))?;
        Backend::flush(&mut backend)
    })?;
    let through_buf_writer = counted_writes(|counted| {
        let mut backend = CrosstermBackend::new(BufWriter::with_capacity(1 << 20, counted));
        backend.draw(empty.diff_iter(&frame))?;
        Backend::flush(&mut backend)
    })?;
    assert_eq!(through_line_writer, (67, MEASURED_GRADIENT_FRAME_BYTES));
    assert_eq!(through_buf_writer, (1, MEASURED_GRADIENT_FRAME_BYTES));
    Ok(())
}
