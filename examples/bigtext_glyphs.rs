// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{BigText, PixelSize, RenderCx, Widget};

fn main() -> io::Result<()> {
    let theme = Theme::builtin(Variant::Dark);
    let mut out = io::BufWriter::new(io::stdout().lock());
    for ch in "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ:.-%".chars() {
        for (name, size) in [
            ("Full", PixelSize::Full),
            ("HalfHeight", PixelSize::HalfHeight),
            ("Quadrant", PixelSize::Quadrant),
        ] {
            let (width, height) = size.cells();
            let mut grid = Grid::new(width, height);
            let mut clusters = ClusterTable::new();
            let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
            BigText::new(&ch.to_string())
                .pixel_size(size)
                .render(Rect::new(0, 0, width, height), &mut cx);
            for row in 0..height {
                writeln!(out, "{ch}\t{name}\t{}", grid.row_display(row, &clusters))?;
            }
        }
    }
    out.flush()
}
