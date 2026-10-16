// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder, write_cursor_state};
use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::buffer::Cell as RCell;
use ratatui::layout::{Position, Size};

use crate::convert::apply_cell;

/// ratatui [`Backend`] that accumulates cells into a lunatui [`Grid`] and encodes on [`flush`](Self::flush).
#[derive(Debug)]
pub struct LunatuiBackend<W> {
    width: u16,
    height: u16,
    grid: Grid,
    previous: Grid,
    clusters: ClusterTable,
    writer: W,
    encoder: Encoder,
    color_mode: ColorMode,
    cursor: Position,
    cursor_visible: bool,
    bytes_flushed: u64,
}

impl<W: Write> LunatuiBackend<W> {
    /// New backend with a fixed viewport size and output writer.
    #[must_use]
    pub fn new(width: u16, height: u16, writer: W) -> Self {
        let grid = Grid::new(width, height);
        Self {
            width,
            height,
            grid,
            previous: Grid::new(width, height),
            clusters: ClusterTable::new(),
            writer,
            encoder: Encoder::new(),
            color_mode: ColorMode::TrueColor,
            cursor: Position::ORIGIN,
            cursor_visible: true,
            bytes_flushed: 0,
        }
    }

    /// Selects how colors are encoded on flush.
    #[must_use]
    pub fn with_color_mode(mut self, mode: ColorMode) -> Self {
        self.color_mode = mode;
        self
    }

    /// Packed grid after the last [`draw`](Backend::draw).
    #[must_use]
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Cluster table paired with [`grid`](Self::grid).
    #[must_use]
    pub fn clusters(&self) -> &ClusterTable {
        &self.clusters
    }

    /// Total encoded bytes written across all flushes.
    #[must_use]
    pub fn bytes_flushed(&self) -> u64 {
        self.bytes_flushed
    }

    fn reset_grid(&mut self) {
        self.clusters = ClusterTable::new();
        self.grid = Grid::new(self.width, self.height);
        let blank = Style::new();
        let area = Rect::new(0, 0, self.width, self.height);
        self.grid.fill(area, blank, ' ', &mut self.clusters);
    }
}

impl<W: Write> Backend for LunatuiBackend<W> {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a RCell)>,
    {
        for (x, y, cell) in content {
            apply_cell(&mut self.grid, &mut self.clusters, x, y, cell);
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.cursor_visible = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.cursor_visible = true;
        Ok(())
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(self.cursor)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.cursor = position.into();
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.reset_grid();
        Ok(())
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        if clear_type == ClearType::All {
            return self.clear();
        }
        let blank = Style::new();
        match clear_type {
            ClearType::All => self.reset_grid(),
            ClearType::AfterCursor => {
                let y = self.cursor.y;
                let x = self.cursor.x;
                for row in y..self.height {
                    let start_x = if row == y { x } else { 0 };
                    for col in start_x..self.width {
                        self.grid.put_str(col, row, blank, " ", &mut self.clusters);
                    }
                }
            }
            ClearType::BeforeCursor => {
                let y = self.cursor.y;
                let x = self.cursor.x;
                for row in 0..=y.min(self.height.saturating_sub(1)) {
                    let end_x = if row == y {
                        x
                    } else {
                        self.width.saturating_sub(1)
                    };
                    for col in 0..=end_x.min(self.width.saturating_sub(1)) {
                        self.grid.put_str(col, row, blank, " ", &mut self.clusters);
                    }
                }
            }
            ClearType::CurrentLine => {
                let row = self.cursor.y;
                if row < self.height {
                    for col in 0..self.width {
                        self.grid.put_str(col, row, blank, " ", &mut self.clusters);
                    }
                }
            }
            ClearType::UntilNewLine => {
                let row = self.cursor.y;
                if row < self.height {
                    for col in self.cursor.x..self.width {
                        self.grid.put_str(col, row, blank, " ", &mut self.clusters);
                    }
                }
            }
        }
        Ok(())
    }

    fn size(&self) -> io::Result<Size> {
        Ok(Size::new(self.width, self.height))
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize {
            columns_rows: Size::new(self.width, self.height),
            pixels: Size::new(self.width * 8, self.height * 16),
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut bytes = self
            .encoder
            .encode_diff(&self.previous, &self.grid, &self.clusters, self.color_mode)
            .to_vec();
        write_cursor_state(
            &mut bytes,
            self.cursor.x,
            self.cursor.y,
            self.cursor_visible,
        );
        self.writer.write_all(&bytes)?;
        self.writer.flush()?;
        self.bytes_flushed += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        self.previous = self.grid.clone();
        Ok(())
    }
}
