// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::collections::HashSet;

use lunatui_badge::{Badge, Kbd, Look, Marker};
use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_theme::{Theme, Variant};
use lunatui_ui_tokens::{Role, Tone};
use lunatui_widgets::{RenderCx, Widget};

const CELL: (u32, u32) = (16, 32);
const BACK: Color = Color::Rgb(10, 20, 30);

struct Host {
    accept: bool,
    background: Option<[u8; 3]>,
    images: Vec<(Rect, ImageSource)>,
}

impl Host {
    fn accepting() -> Self {
        Self {
            accept: true,
            background: Some([0, 0, 0]),
            images: Vec::new(),
        }
    }
}

impl ImageSink for Host {
    fn supports_pixels(&self) -> bool {
        true
    }
    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        Some(PixelTarget::new(
            PixelSize::new(
                u32::from(area.width) * CELL.0,
                u32::from(area.height) * CELL.1,
            )?,
            false,
            self.background,
        ))
    }
    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }
    fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
        if self.accept {
            self.images.push((area, source.clone()));
        }
        self.accept
    }
}

fn backdrop(width: u16, background: Color) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(width, 1);
    let mut clusters = ClusterTable::new();
    grid.fill(
        Rect::new(0, 0, width, 1),
        Style::new().bg(background),
        'x',
        &mut clusters,
    );
    (grid, clusters)
}

fn cells(grid: &Grid) -> Vec<u128> {
    (0..grid.width()).map(|x| grid.cell_bits(x, 0)).collect()
}

fn render(
    widget: &dyn Widget,
    width: u16,
    mode: RenderMode,
    host: Option<&mut Host>,
    background: Color,
) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let (mut grid, mut clusters) = backdrop(width + 2, background);
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme).render_mode(mode);
    if let Some(host) = host {
        cx = cx.image_sink(host);
    }
    widget.render(Rect::new(0, 0, width, 1), &mut cx);
    (grid, clusters)
}

fn pixel(image: &ImageSource, x: u32, y: u32) -> [u8; 3] {
    let index = usize::try_from((y * image.size().width() + x) * 4).unwrap_or(usize::MAX);
    let bytes = image.rgba8_pixels();
    [bytes[index], bytes[index + 1], bytes[index + 2]]
}

fn rgb(color: Color) -> [u8; 3] {
    assert!(
        matches!(color, Color::Rgb(..)),
        "{color:?} is not an RGB color"
    );
    match color {
        Color::Rgb(red, green, blue) => [red, green, blue],
        _ => [0, 0, 0],
    }
}

fn token(role: Role) -> [u8; 3] {
    rgb(Theme::builtin(Variant::Dark).resolve(role.token()))
}

fn tone(tone: Tone) -> [u8; 3] {
    rgb(Theme::builtin(Variant::Dark).resolve(tone.fill()))
}

fn on_tone(tone: Tone) -> [u8; 3] {
    rgb(Theme::builtin(Variant::Dark).resolve(tone.on_fill()))
}

fn one_image(host: &Host) -> (Rect, &ImageSource) {
    assert_eq!(host.images.len(), 1, "one image");
    let (area, image) = &host.images[0];
    (*area, image)
}

fn badges() -> Vec<Badge<'static>> {
    let mut badges = Vec::new();
    for look in [Look::Filled, Look::Subtle] {
        for tone in [
            Tone::Neutral,
            Tone::Primary,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
        ] {
            badges.push(Badge::new("deploying").tone(tone).look(look));
            badges.push(
                Badge::new("1 failed")
                    .tone(tone)
                    .look(look)
                    .marker(Marker::Cross),
            );
        }
    }
    badges
}

#[test]
fn declined_pixels_leave_exactly_the_cell_mode_cells() {
    let keys: [&[&str]; 3] = [&["Ctrl", "K"], &["↑", "↓"], &["Tab"]];
    let mut widgets: Vec<Box<dyn Widget>> = Vec::new();
    for badge in badges() {
        widgets.push(Box::new(badge));
    }
    for keys in keys {
        widgets.push(Box::new(Kbd::new(keys)));
    }
    for widget in &widgets {
        for width in [0, 3, 7, 20] {
            let (expected, _) = render(widget.as_ref(), width, RenderMode::Cell, None, BACK);
            let mut declining = Host {
                accept: false,
                ..Host::accepting()
            };
            let mut no_backdrop = Host {
                background: None,
                ..Host::accepting()
            };
            let runs = [
                render(widget.as_ref(), width, RenderMode::Graphite, None, BACK),
                render(
                    widget.as_ref(),
                    width,
                    RenderMode::Graphite,
                    Some(&mut declining),
                    BACK,
                ),
                render(
                    widget.as_ref(),
                    width,
                    RenderMode::Cell,
                    Some(&mut Host::accepting()),
                    BACK,
                ),
            ];
            for (grid, _) in &runs {
                assert_eq!(cells(grid), cells(&expected), "width {width}");
            }
            let (expected, _) = render(
                widget.as_ref(),
                width,
                RenderMode::Cell,
                None,
                Color::Default,
            );
            let (grid, _) = render(
                widget.as_ref(),
                width,
                RenderMode::Graphite,
                Some(&mut no_backdrop),
                Color::Default,
            );
            assert_eq!(
                cells(&grid),
                cells(&expected),
                "no RGB backdrop, width {width}"
            );
            assert_eq!(no_backdrop.images.len(), 0);
        }
    }
}

#[test]
fn a_graphite_badge_covers_its_cells_with_a_pill_in_the_tone_fill() {
    let badge = Badge::new("failed").tone(Tone::Danger);
    let mut host = Host::accepting();
    let (grid, clusters) = render(&badge, 10, RenderMode::Graphite, Some(&mut host), BACK);
    let (area, image) = one_image(&host);
    assert_eq!(area, Rect::new(0, 0, 8, 1), "the cells ` failed ` covers");
    assert_eq!(grid.row_display(0, &clusters), "        xxxx");
    for x in 0..8 {
        let style = grid.get(x, 0).map(lunatui_core::Cell::style);
        assert_eq!(style, Some(Style::new().bg(BACK)), "cell {x}");
    }
    let middle = CELL.1 / 2;
    assert_eq!(
        pixel(image, 18, middle),
        tone(Tone::Danger),
        "inside the pill"
    );
    assert_eq!(pixel(image, 1, 1), rgb(BACK), "outside the rounded corner");
    assert_eq!(pixel(image, 18, 1), rgb(BACK), "above the pill");
    let ink = on_tone(Tone::Danger);
    let inked = (0..image.size().width())
        .filter(|&x| (0..CELL.1).any(|y| pixel(image, x, y) == ink))
        .count();
    assert!(
        inked > 10,
        "the label is drawn in the tone's text color: {inked} columns"
    );
}

#[test]
fn a_subtle_badge_tints_its_pill_with_the_tone() {
    let badge = Badge::new("waiting").tone(Tone::Warning).look(Look::Subtle);
    let mut host = Host::accepting();
    render(&badge, 12, RenderMode::Graphite, Some(&mut host), BACK);
    let (_, image) = one_image(&host);
    let [back, warning] = [rgb(BACK), tone(Tone::Warning)];
    let fill = pixel(image, 20, CELL.1 / 2);
    for channel in 0..3 {
        let (under, over) = (f32::from(back[channel]), f32::from(warning[channel]));
        let tint = under + (over - under) * 0.2;
        assert!(
            (f32::from(fill[channel]) - tint).abs() <= 1.0,
            "{fill:?} is not a fifth of {warning:?} over {back:?}"
        );
    }
    let neutral = Badge::new("idle").look(Look::Subtle);
    let mut host = Host::accepting();
    render(&neutral, 8, RenderMode::Graphite, Some(&mut host), BACK);
    let (_, image) = one_image(&host);
    assert_eq!(pixel(image, 18, CELL.1 / 2), token(Role::Card));
}

#[test]
fn each_marker_draws_its_own_shape() {
    let markers = [
        None,
        Some(Marker::Dot),
        Some(Marker::Ring),
        Some(Marker::Diamond),
        Some(Marker::Cross),
        Some(Marker::Square),
        Some(Marker::Bang),
    ];
    let mut images = HashSet::new();
    let mut ink = Vec::new();
    let color = tone(Tone::Success);
    for marker in markers {
        let mut badge = Badge::new("").tone(Tone::Success).look(Look::Subtle);
        if let Some(marker) = marker {
            badge = badge.marker(marker);
        }
        let mut host = Host::accepting();
        render(&badge, 6, RenderMode::Graphite, Some(&mut host), BACK);
        let (_, image) = one_image(&host);
        images.insert(image.rgba8_pixels().to_vec());
        let size = image.size();
        let count = (0..size.width())
            .flat_map(|x| (0..size.height()).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel(image, x, y) == color)
            .count();
        ink.push(count);
    }
    assert_eq!(images.len(), markers.len(), "every marker looks different");
    assert_eq!(ink[0], 0, "no marker, no ink");
    assert!(
        ink[1] > ink[2],
        "a dot is filled and a ring is not: {ink:?}"
    );
    assert!(ink.iter().skip(1).all(|&count| count > 0), "{ink:?}");
}

#[test]
fn the_cell_marker_glyph_and_measure_include_the_marker() {
    let badge = Badge::new("1 working").marker(Marker::Dot);
    assert_eq!(
        badge.measure(lunatui_core::Size::new(40, 1)),
        lunatui_core::Size::new(13, 1)
    );
    let (grid, clusters) = render(&badge, 13, RenderMode::Cell, None, BACK);
    assert_eq!(grid.row_display(0, &clusters), " ● 1 working xx");
    assert_eq!(Marker::Diamond.glyph(), '◆');
}

#[test]
fn graphite_keycaps_sit_in_their_chips_with_the_gap_between_them() {
    let kbd = Kbd::new(&["Ctrl", "K"]);
    let mut host = Host::accepting();
    let (grid, clusters) = render(&kbd, 12, RenderMode::Graphite, Some(&mut host), BACK);
    let (area, image) = one_image(&host);
    assert_eq!(area, Rect::new(0, 0, 10, 1), "` Ctrl ` + gap + ` K `");
    assert_eq!(grid.row_display(0, &clusters), "          xxxx");
    let middle = CELL.1 / 2;
    let card = token(Role::Card);
    let above_label = 7;
    assert_eq!(pixel(image, 6 * 16 + 8, middle), rgb(BACK), "the gap cell");
    assert_eq!(pixel(image, 3 * 16, above_label), card, "the Ctrl cap");
    assert_eq!(pixel(image, 17 * 8, above_label), card, "the K cap");
    assert_eq!(
        pixel(image, 2, middle),
        rgb(BACK),
        "the Ctrl cap is narrower than ` Ctrl `"
    );
    assert_eq!(
        pixel(image, 7 * 16 + 2, middle),
        rgb(BACK),
        "the K cap is narrower than ` K `"
    );
    let border = token(Role::Border);
    let edged = (0..image.size().width())
        .filter(|&x| pixel(image, x, 5) == border || pixel(image, x, CELL.1 - 5) == border)
        .count();
    assert!(edged > 40, "the caps have an outline: {edged}");
}
