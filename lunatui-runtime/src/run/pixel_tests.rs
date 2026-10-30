// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::ImageSource;
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
    Ok(())
}
