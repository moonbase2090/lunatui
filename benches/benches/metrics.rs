// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::io;

use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};
use lunatui_bench::report::{Metrics, table};
use lunatui_bench::scenes::ratatui::dashboard::{self, Model};
use lunatui_bench::scenes::ratatui::{ColorOutput, gradient, set_color_output};
use lunatui_bench::{AllocStats, CountingAllocator, CountingWriter, WriteStats};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::system();

fn main() -> io::Result<()> {
    let large = frame(LARGE_FRAME_BYTES);
    let rows = [
        measure("frame (68 KiB)", |_| {
            black_box(frame(LARGE_FRAME_BYTES));
            Ok(())
        })?,
        measure("write_frame (68 KiB)", |out| write_frame(out, &large))?,
        measure("grow_buffer (1 KiB)", |_| {
            black_box(grow_buffer(1024));
            Ok(())
        })?,
    ];
    print!("{}", table(&rows));

    set_color_output(ColorOutput::Enabled);
    let ratatui_rows = [
        dashboard_frames(
            "ratatui dashboard 200x60, 100 frames",
            Rect::new(0, 0, 200, 60),
        )?,
        dashboard_frames(
            "ratatui dashboard 400x120, 100 frames",
            Rect::new(0, 0, 400, 120),
        )?,
        gradient_frame("ratatui gradient 200x60, LineWriter", GradientWriter::Line)?,
        gradient_frame(
            "ratatui gradient 200x60, BufWriter 1 MiB",
            GradientWriter::Buffered,
        )?,
    ];
    print!("{}", table(&ratatui_rows));
    Ok(())
}

fn measure(
    name: &'static str,
    work: impl FnOnce(&mut CountingWriter<io::Sink>) -> io::Result<()>,
) -> io::Result<Metrics> {
    let mut out = CountingWriter::new(io::sink());
    let (result, alloc) = ALLOC.measure(|| work(&mut out));
    result?;
    Ok(Metrics {
        name,
        alloc,
        write: out.stats(),
    })
}

const WARM_UP_FRAMES: u32 = 5;
const MEASURED_FRAMES: u32 = 100;
const MEASURED_FRAMES_NONE: u32 = 0;

fn dashboard_frames(name: &'static str, area: Rect) -> io::Result<Metrics> {
    let (_, warm_up_writes) = draw_dashboard(area, MEASURED_FRAMES_NONE)?;
    let (alloc, total_writes) = draw_dashboard(area, MEASURED_FRAMES)?;
    Ok(Metrics {
        name,
        alloc,
        write: WriteStats {
            writes: total_writes.writes - warm_up_writes.writes,
            bytes: total_writes.bytes - warm_up_writes.bytes,
            flushes: total_writes.flushes - warm_up_writes.flushes,
        },
    })
}

fn draw_dashboard(area: Rect, measured_frames: u32) -> io::Result<(AllocStats, WriteStats)> {
    let mut out = CountingWriter::new(io::sink());
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(&mut out),
        TerminalOptions {
            viewport: Viewport::Fixed(area),
        },
    )?;
    let mut model = Model::new();
    for tick in 0..WARM_UP_FRAMES {
        model.tick = tick;
        terminal.draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))?;
    }
    let (drawn, alloc) = ALLOC.measure(|| -> io::Result<()> {
        for tick in WARM_UP_FRAMES..WARM_UP_FRAMES + measured_frames {
            model.tick = tick;
            model.scroll = usize::try_from(tick % 50).unwrap_or(0);
            terminal.draw(|frame| dashboard::render(&model, frame.area(), frame.buffer_mut()))?;
        }
        Ok(())
    });
    drawn?;
    drop(terminal);
    Ok((alloc, out.stats()))
}

#[derive(Clone, Copy)]
enum GradientWriter {
    Line,
    Buffered,
}

fn gradient_frame(name: &'static str, writer: GradientWriter) -> io::Result<Metrics> {
    let area = Rect::new(0, 0, 200, 60);
    let frame = gradient::frame(area);
    let empty = Buffer::empty(area);
    let mut counted = CountingWriter::new(io::sink());
    match writer {
        GradientWriter::Line => {
            let mut backend = CrosstermBackend::new(io::LineWriter::new(&mut counted));
            backend.draw(empty.diff_iter(&frame))?;
            Backend::flush(&mut backend)?;
        }
        GradientWriter::Buffered => {
            let mut backend =
                CrosstermBackend::new(io::BufWriter::with_capacity(1 << 20, &mut counted));
            backend.draw(empty.diff_iter(&frame))?;
            Backend::flush(&mut backend)?;
        }
    }
    Ok(Metrics {
        name,
        alloc: AllocStats::default(),
        write: counted.stats(),
    })
}
