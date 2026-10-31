// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io;

use lunatui::{App, ControlFlow, RunOptions, is_quit, run};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_runtime::RenderFrame;
use lunatui_term::Event;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Block, Image, ImageFit, ImageId, ImageSource, Paragraph, RenderCx, Widget};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 32;

const FRAME: &[u8] = include_bytes!("data/graphite-status-palette.rgba");
const FRAME_WIDTH: u32 = 320;
const FRAME_HEIGHT: u32 = 96;

fn source() -> Result<ImageSource, io::Error> {
    ImageSource::rgba8(FRAME_WIDTH, FRAME_HEIGHT, FRAME.to_vec().into_boxed_slice())
        .map_err(|error| io::Error::other(error.to_string()))
}

fn main() -> io::Result<()> {
    let source = source()?;
    run(GraphitePixels { source }, RunOptions::new(WIDTH, HEIGHT))
}

struct GraphitePixels {
    source: ImageSource,
}

impl App for GraphitePixels {
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

impl GraphitePixels {
    fn draw(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let panel = Block::bordered().title("LUNATUI  /  GRAPHITE PIXELS");
        panel.render(area, cx);
        let inner = panel.inner(area);
        Paragraph::new("A Graphite PixelBuffer frame, presented through the PixelLayer")
            .style(Style::new().fg(Color::Rgb(184, 167, 240)))
            .render(Rect::new(inner.x, inner.y, inner.width, 1), cx);
        let Some(id) = ImageId::new(81) else {
            return;
        };
        Image::new(self.source.clone())
            .id(id)
            .fit(ImageFit::Contain)
            .render(
                Rect::new(
                    inner.x + 2,
                    inner.y + 2,
                    inner.width.saturating_sub(4),
                    inner.height.saturating_sub(5),
                ),
                cx,
            );
        Paragraph::new("320 x 96 pixels, straight-alpha RGBA8, drawn by Graphite    q quit")
            .style(Style::new().fg(Color::Rgb(164, 174, 193)))
            .render(
                Rect::new(
                    inner.x,
                    inner.y + inner.height.saturating_sub(1),
                    inner.width,
                    1,
                ),
                cx,
            );
    }
}

#[cfg(test)]
mod tests {
    use super::source;

    #[test]
    fn the_committed_frame_becomes_an_image_source_of_its_size() -> Result<(), std::io::Error> {
        let source = source()?;
        assert_eq!((source.size().width(), source.size().height()), (320, 96));
        Ok(())
    }
}
