// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};

const LOG_LINES: usize = 200;

struct Model {
    tick: u32,
    scroll: usize,
    log: Vec<String>,
}

impl Model {
    fn new() -> Self {
        let log = (0..LOG_LINES)
            .map(|i| {
                format!(
                    "[{i:03}] worker-{worker}: processed batch {i} ({rows} rows)",
                    worker = i % 8,
                    rows = (i * 17) % 4096
                )
            })
            .collect();
        Self {
            tick: 0,
            scroll: 0,
            log,
        }
    }
}

fn render_frame(
    model: &Model,
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    width: u16,
    height: u16,
) {
    let bg = Color::Rgb(0x1a, 0x1b, 0x26);
    let accent = Color::Rgb(0x7a, 0xa2, 0xf7);
    let dim = Color::Rgb(0x56, 0x5f, 0x89);
    let bg_style = Style::new().bg(bg);
    grid.fill(
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        },
        bg_style,
        ' ',
        clusters,
    );

    let bold = Attrs::default().set_bold(true);
    let header_style = Style::new().fg(accent).bg(bg).with_attrs(bold);
    grid.put_str(1, 0, header_style, "lunatui encode bench", clusters);
    let counter = format!("tick {:>5}  scroll {:>4}", model.tick, model.scroll);
    grid.put_str(1, 1, Style::new().fg(dim).bg(bg), &counter, clusters);

    let log_top = 3_u16;
    let log_bottom = height.saturating_sub(2);
    if log_top < log_bottom {
        grid.put_str(
            1,
            2,
            Style::new().fg(accent).bg(bg),
            "scrolling log",
            clusters,
        );
        let visible = usize::from(log_bottom - log_top);
        for row in 0..visible {
            let line_index = (model.scroll + row) % model.log.len();
            let line = &model.log[line_index];
            let y = log_top + u16::try_from(row).unwrap_or(0);
            grid.put_str(1, y, Style::new().fg(Color::Default).bg(bg), line, clusters);
        }
    }

    let status = "dogfood encode throughput";
    grid.put_str(
        1,
        height - 1,
        Style::new().fg(accent).bg(bg),
        status,
        clusters,
    );
}

/// Dashboard-like frames at `width`×`height` for encoder benches.
pub struct EncodeFrames {
    /// Cleared grid before the first paint.
    pub empty: Grid,
    /// Frame after a small tick update.
    pub tick: Grid,
    /// Frame after a log scroll step.
    pub scroll: Grid,
    /// Shared cluster table used by all frames.
    pub clusters: ClusterTable,
}

impl EncodeFrames {
    /// Builds empty, tick, and scroll frames for encoder throughput benches.
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        let mut clusters = ClusterTable::new();
        let empty = Grid::new(width, height);
        let mut tick_model = Model::new();
        tick_model.tick = 4;
        let mut tick = Grid::new(width, height);
        render_frame(&tick_model, &mut tick, &mut clusters, width, height);

        let mut scroll_model = Model::new();
        scroll_model.tick = 8;
        scroll_model.scroll = 12;
        let mut scroll = Grid::new(width, height);
        render_frame(&scroll_model, &mut scroll, &mut clusters, width, height);

        Self {
            empty,
            tick,
            scroll,
            clusters,
        }
    }
}
