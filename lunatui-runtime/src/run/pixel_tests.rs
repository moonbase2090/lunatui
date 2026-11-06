// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{Fit, ImageId, ImageRequest, ImageSource};
use lunatui_term::{Caps, CellPixels, Encoder, Event, GraphicsProtocol, PixelLayer, Protocols};

use super::{FrameState, render_present};
use crate::{App, ControlFlow, RenderFrame};

struct ChartSwitch {
    title: &'static str,
    pixels: ImageSource,
}

impl App for ChartSwitch {
    fn update(&mut self, _: &Event) -> ControlFlow {
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, _: Rect, clusters: &mut ClusterTable) {
        grid.put_str(
            0,
            0,
            Style::new()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(0, 0, 0)),
            self.title,
            clusters,
        );
    }

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, area: Rect) {
        let (grid, clusters, images) = frame.parts();
        self.render(grid, area, clusters);
        assert!(images.submit_raster(&self.pixels, Rect::new(2, 2, 10, 8), 0));
    }
}

fn command_id(stream: &[u8], action: &str) -> Result<String, String> {
    let text = std::str::from_utf8(stream).map_err(|error| error.to_string())?;
    text.split("\x1b_G")
        .skip(1)
        .filter_map(|command| command.split([';', '\x1b']).next())
        .find(|header| header.split(',').any(|part| part == action))
        .and_then(|header| header.split(',').find_map(|part| part.strip_prefix("i=")))
        .map(str::to_owned)
        .ok_or_else(|| format!("missing {action} image id"))
}

#[test]
fn switching_saturation_to_latency_finishes_upload_before_the_new_title_and_retires_old_pixels()
-> Result<(), String> {
    let area = Rect::new(0, 0, 20, 12);
    let caps = Caps {
        protocols: Protocols {
            graphics: GraphicsProtocol::Kitty,
            ..Protocols::default()
        },
        cell_pixels: CellPixels::new(13, 25),
        ..Caps::default()
    };
    let mut frames = FrameState {
        prev: Grid::new(20, 12),
        next: Grid::new(20, 12),
        clusters: ClusterTable::new(),
        encoder: Encoder::new(),
        last_palette: None,
        images: PixelLayer::from_caps(&caps),
    };
    let mut app = ChartSwitch {
        title: "Saturation",
        pixels: ImageSource::rgba8(130, 200, [255, 0, 0, 255].repeat(26_000).into_boxed_slice())
            .map_err(|error| error.to_string())?,
    };
    let mut transactions = Vec::new();
    render_present(&mut app, area, &mut frames, |parts| {
        transactions.push(parts.iter().map(|part| part.to_vec()).collect::<Vec<_>>());
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(
        transactions.len(),
        2,
        "upload and visible update need separate sync frames"
    );
    let old_id = command_id(&transactions[0][0], "a=t")?;
    assert_eq!(command_id(&transactions[1][1], "a=p")?, old_id);
    let mut screen = lunatui_test::Oracle::new(12, 20);
    screen.process(&transactions[1][0]);
    assert_eq!(screen.contents(), "Saturation");

    app.title = "Latency scatter";
    app.pixels = ImageSource::rgba8(130, 200, [0, 0, 255, 255].repeat(26_000).into_boxed_slice())
        .map_err(|error| error.to_string())?;
    render_present(&mut app, area, &mut frames, |parts| {
        transactions.push(parts.iter().map(|part| part.to_vec()).collect::<Vec<_>>());
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(transactions.len(), 4);
    assert_eq!(
        transactions[2].len(),
        1,
        "upload cannot include cell output"
    );
    let upload = std::str::from_utf8(&transactions[2][0]).map_err(|error| error.to_string())?;
    assert!(upload.starts_with("\x1b_Ga=t,"));
    assert!(
        upload.contains(";AAD//wAA//8AAP//"),
        "new pixels must be blue"
    );
    assert!(upload.contains("\x1b_Gm=1;"), "exercise chunked transfer");
    assert!(
        upload.contains("\x1b_Gm=0;"),
        "finish the upload before changing labels"
    );
    assert!(!upload.contains("Latency scatter"));
    let new_id = command_id(upload.as_bytes(), "a=t")?;
    assert_ne!(
        new_id, old_id,
        "upload cannot replace the visible image in place"
    );
    assert_eq!(transactions[3].len(), 2);
    screen.process(&transactions[3][0]);
    assert_eq!(screen.contents(), "Latency scatter");
    let updates = &transactions[3][1];
    assert_eq!(command_id(updates, "a=p")?, new_id);
    assert_eq!(
        command_id(updates, "a=d")?,
        old_id,
        "old plot must not survive the new title frame"
    );
    assert!(
        !std::str::from_utf8(updates)
            .map_err(|error| error.to_string())?
            .contains("a=t")
    );
    render_present(&mut app, area, &mut frames, |parts| {
        transactions.push(parts.iter().map(|part| part.to_vec()).collect::<Vec<_>>());
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(
        transactions.len(),
        4,
        "unchanged Kitty frame must skip output"
    );
    Ok(())
}

struct ImageSwitch {
    position: Option<Rect>,
    id: ImageId,
    pixels: ImageSource,
}

impl App for ImageSwitch {
    fn update(&mut self, _: &Event) -> ControlFlow {
        ControlFlow::Continue
    }

    fn render(&mut self, _: &mut Grid, _: Rect, _: &mut ClusterTable) {}

    fn render_frame(&mut self, frame: &mut RenderFrame<'_>, _: Rect) {
        if let Some(area) = self.position {
            let (_, _, images) = frame.parts();
            assert!(images.submit(ImageRequest::new(
                self.id,
                0,
                &self.pixels,
                area,
                Fit::Stretch,
                0,
                255,
            )));
        }
    }
}

fn immediate_image_repaints_blank_cells(
    graphics: GraphicsProtocol,
    image_command: &str,
    next_position: Option<Rect>,
) -> Result<(), String> {
    let area = Rect::new(0, 0, 6, 4);
    let caps = Caps {
        protocols: Protocols {
            graphics,
            ..Protocols::default()
        },
        cell_pixels: CellPixels::new(1, 1),
        ..Caps::default()
    };
    let mut frames = FrameState {
        prev: Grid::new(6, 4),
        next: Grid::new(6, 4),
        clusters: ClusterTable::new(),
        encoder: Encoder::new(),
        last_palette: None,
        images: PixelLayer::from_caps(&caps),
    };
    let mut app = ImageSwitch {
        position: Some(Rect::new(2, 1, 2, 2)),
        id: ImageId::new(1).ok_or("nonzero image identity")?,
        pixels: ImageSource::rgba8(2, 2, [255, 0, 0, 255].repeat(4).into_boxed_slice())
            .map_err(|error| error.to_string())?,
    };
    let mut output = Vec::new();
    render_present(&mut app, area, &mut frames, |parts| {
        output.extend(parts.iter().flat_map(|part| part.iter()).copied());
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    assert!(String::from_utf8_lossy(&output).contains(image_command));

    let mut screen = lunatui_test::Oracle::new(4, 6);
    screen.process(b"\x1b[2;3HXX\x1b[3;3HXX");
    app.position = next_position;
    let mut presents = 0;
    render_present(&mut app, area, &mut frames, |parts| {
        presents += 1;
        screen.process(parts[0]);
        Ok(())
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(presents, 1, "{graphics:?} needs a cell repaint frame");
    assert_eq!(
        screen.contents(),
        "      \n      \n      \n      ",
        "{graphics:?} must overwrite old pixels"
    );
    if next_position.is_none() {
        render_present(&mut app, area, &mut frames, |_| {
            presents += 1;
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        assert_eq!(
            presents, 1,
            "an unchanged empty frame must skip presentation"
        );
    }
    Ok(())
}

#[test]
fn omitted_iterm2_images_repaint_default_blank_cells() -> Result<(), String> {
    immediate_image_repaints_blank_cells(GraphicsProtocol::Iterm2, "\x1b]1337;File=inline=1;", None)
}

#[test]
fn omitted_sixel_images_repaint_default_blank_cells() -> Result<(), String> {
    immediate_image_repaints_blank_cells(GraphicsProtocol::Sixel, "\x1bP0;1;0q", None)
}

#[test]
fn moved_iterm2_images_repaint_default_blank_cells() -> Result<(), String> {
    immediate_image_repaints_blank_cells(
        GraphicsProtocol::Iterm2,
        "\x1b]1337;File=inline=1;",
        Some(Rect::new(0, 0, 2, 2)),
    )
}

#[test]
fn moved_sixel_images_repaint_default_blank_cells() -> Result<(), String> {
    immediate_image_repaints_blank_cells(
        GraphicsProtocol::Sixel,
        "\x1bP0;1;0q",
        Some(Rect::new(0, 0, 2, 2)),
    )
}
