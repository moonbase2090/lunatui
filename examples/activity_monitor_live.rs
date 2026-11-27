// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;
use std::time::Instant;

use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_runtime::{App, ControlFlow, RenderFrame, RunOptions, is_quit, run};
use lunatui_term::{Event, terminal_size};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::RenderCx;

#[path = "activity_monitor_scene.rs"]
mod scene;

const FRAMES: usize = 10;

struct Live {
    theme: Theme,
    started: Instant,
}

impl Live {
    fn frame(&self) -> usize {
        usize::try_from(self.started.elapsed().as_secs()).unwrap_or(0) % FRAMES
    }
}

impl App for Live {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }

    fn render(&mut self, grid: &mut Grid, _area: Rect, clusters: &mut ClusterTable) {
        let frame = self.frame();
        scene::draw(&mut RenderCx::new(grid, clusters, &self.theme), frame);
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, _area: Rect) {
        let mode = frame.mode();
        let tick = self.frame();
        let (grid, clusters, images) = frame.parts();
        let mut cx = RenderCx::new(grid, clusters, &self.theme)
            .image_sink(images)
            .render_mode(mode);
        scene::draw(&mut cx, tick);
    }

    fn needs_animation_frames(&self) -> bool {
        true
    }
}

fn main() -> io::Result<()> {
    let (columns, rows) = terminal_size().unwrap_or((scene::COLUMNS, scene::ROWS));
    run(
        Live {
            theme: Theme::builtin(Variant::Dark),
            started: Instant::now(),
        },
        RunOptions::new(columns, rows),
    )
}
