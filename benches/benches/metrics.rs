// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![doc = include_str!("../README.md")]

use std::hint::black_box;
use std::io;

use lunatui_bench::calibration::{LARGE_FRAME_BYTES, frame, grow_buffer, write_frame};
use lunatui_bench::report::{Metrics, table};
use lunatui_bench::{CountingAllocator, CountingWriter};

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
