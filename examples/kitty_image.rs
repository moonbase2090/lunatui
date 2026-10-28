// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::Event;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, Image, ImageFit, ImageId, ImageSource, RenderCx, Widget};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 32;

fn main() -> io::Result<()> {
    let source = ImageSource::decode(include_bytes!("../site/assets/sky.webp"))
        .map_err(|error| io::Error::other(error.to_string()))?;
    run(KittyImage { source }, RunOptions::new(WIDTH, HEIGHT))
}

struct KittyImage {
    source: ImageSource,
}

impl App for KittyImage {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) {
            ControlFlow::Break
        } else {
            ControlFlow::Continue
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(grid, clusters, &theme);
        self.draw(area, &mut cx);
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let theme = Theme::builtin(Variant::Dark);
        let (grid, clusters, images) = frame.parts();
        let mut cx = RenderCx::new(grid, clusters, &theme).image_sink(images);
        self.draw(area, &mut cx);
    }
}

impl KittyImage {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let panel = Block::bordered().title("LUNATUI  /  IMAGE");
        panel.render(area, cx);
        let inner = panel.inner(area);
        cx.put_str(
            inner.x,
            inner.y,
            "KITTY GRAPHICS  ·  auto-detected, cell fallback",
            Style::new().fg(Color::Rgb(184, 167, 240)),
        );
        let image_area = Rect::new(
            inner.x + 2,
            inner.y + 2,
            inner.width.saturating_sub(4),
            inner.height.saturating_sub(5),
        );
        let Some(image_id) = ImageId::new(42) else {
            return;
        };
        Image::new(self.source.clone())
            .id(image_id)
            .fit(ImageFit::Contain)
            .render(image_area, cx);
        cx.put_str(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            "Press q to leave  ·  Kitty sends pixels; other terminals use the cell renderer",
            Style::new().fg(Color::Rgb(164, 174, 193)),
        );
    }
}
