// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Grid};
use ratatui::buffer::Buffer;

use crate::convert::capture_buffer_to_grid;

#[derive(Debug)]
pub(crate) struct WidgetMemo {
    pub width: u16,
    pub height: u16,
    pub digest: u64,
    pub skip_mask: Vec<bool>,
    pub grid: Grid,
    pub clusters: ClusterTable,
    pub conversions: u32,
    pub hits: u32,
}

impl WidgetMemo {
    pub(crate) fn refresh_from_buffer(&mut self, buffer: &Buffer) {
        capture_buffer_to_grid(buffer, &mut self.grid, &mut self.clusters, &mut self.skip_mask);
    }

    pub(crate) fn clear_if_size(&mut self, width: u16, height: u16) {
        let len = usize::from(width) * usize::from(height);
        if self.width != width || self.height != height || self.skip_mask.len() != len {
            self.width = width;
            self.height = height;
            self.digest = 0;
            self.skip_mask.clear();
            self.skip_mask.resize(len, false);
            self.grid = Grid::new(width, height);
            self.clusters = ClusterTable::new();
        }
    }
}
