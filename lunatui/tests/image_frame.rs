// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Runtime frame handoff tests for the Image widget.

use lunatui_core::{ClusterTable, Color, DecodedGlyph, Grid, Rect};
use lunatui_image::{ImageId, ImageSource};
use lunatui_runtime::{App, ControlFlow, RenderFrame, render_once};
use lunatui_term::Event;
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{Image, RenderCx, Widget};

struct ImageApp {
    image: Image,
    theme: Theme,
}

impl App for ImageApp {
    fn update(&mut self, _event: &Event) -> ControlFlow {
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let mut cx = RenderCx::new(grid, clusters, &self.theme);
        self.image.render(area, &mut cx);
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let (grid, clusters, images) = frame.parts();
        let mut cx = RenderCx::new(grid, clusters, &self.theme).image_sink(images);
        self.image.render(area, &mut cx);
    }
}

#[test]
fn render_once_uses_the_compatibility_cell_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let source = ImageSource::rgba8(
        1,
        2,
        vec![255, 0, 0, 255, 0, 0, 255, 255].into_boxed_slice(),
    )?;
    let mut app = ImageApp {
        image: Image::new(source).id(ImageId::new(1).ok_or("zero image ID")?),
        theme: Theme::builtin(Variant::Dark),
    };

    let (grid, _) = render_once(&mut app, 1, 1);
    let cell = grid.get(0, 0).ok_or("image cell missing")?;
    assert_eq!(cell.glyph().decode(), DecodedGlyph::Inline('▀'));
    assert_eq!(cell.style().fg, Color::Rgb(255, 0, 0));
    assert_eq!(cell.style().bg, Color::Rgb(0, 0, 255));
    Ok(())
}
