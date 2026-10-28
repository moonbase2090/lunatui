// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_term::{ColorMode, Encoder};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Alignment, Block, BorderType, Gauge, Image, ImageFit, ImageSource, Paragraph, RenderCx,
    StatusBar, TitlePosition, Widget, WidgetState,
};

const BG: Color = Color::Rgb(10, 13, 28);
const PANEL: Color = Color::Rgb(17, 22, 49);
const RAISED: Color = Color::Rgb(26, 32, 68);
const TRACK: Color = Color::Rgb(32, 38, 76);
const LINE: Color = Color::Rgb(46, 53, 96);
const LAV: Color = Color::Rgb(185, 167, 240);
const INK: Color = Color::Rgb(236, 231, 218);
const MUTED: Color = Color::Rgb(170, 176, 204);
const DIM: Color = Color::Rgb(100, 107, 140);
const MOON: Color = Color::Rgb(243, 226, 176);
const GREEN: Color = Color::Rgb(155, 227, 168);

const THUMB: (u16, u16) = (18, 9);
const PREVIEW: (u16, u16) = (50, 19);

const SELECTION: [(u32, usize); 6] = [
    (0, 0),
    (1500, 1),
    (2800, 2),
    (4100, 3),
    (6200, 4),
    (7500, 5),
];
const FIT_KEYS: [u32; 2] = [5100, 8700];
const IMPORTS: [(usize, u32); 2] = [(4, 2800), (5, 6200)];

struct Options {
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    snapshot: bool,
}

struct Item {
    name: &'static str,
    kind: &'static str,
    pixels: &'static str,
    size: &'static str,
    captured: &'static str,
    exposure: &'static str,
    tags: &'static str,
    thumb: ImageSource,
    contain: ImageSource,
    cover: ImageSource,
    tones: [f32; 5],
}

#[derive(Clone)]
struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Pixels {
    fn of(source: &ImageSource) -> Self {
        Self {
            width: source.size().width(),
            height: source.size().height(),
            rgba: source.rgba8_pixels().to_vec(),
        }
    }

    fn filled(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Self {
        let mut rgba = Vec::with_capacity(len(width, height));
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&color(x, y));
            }
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let offset = len(self.width, y) + len(x, 1);
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
            self.rgba[offset + 3],
        ]
    }

    fn resized(&self, width: u32, height: u32) -> Self {
        Self::filled(width, height, |x, y| {
            let (x0, x1) = span(x, width, self.width);
            let (y0, y1) = span(y, height, self.height);
            let mut sum = [0_u64; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let [r, g, b, a] = self.at(sx, sy);
                    let a = u64::from(a);
                    sum[0] += u64::from(r) * a;
                    sum[1] += u64::from(g) * a;
                    sum[2] += u64::from(b) * a;
                    sum[3] += a;
                }
            }
            let count = u64::from((x1 - x0) * (y1 - y0));
            if sum[3] == 0 {
                return [0, 0, 0, 0];
            }
            let channel = |total: u64| u8::try_from(total / sum[3]).unwrap_or(u8::MAX);
            [
                channel(sum[0]),
                channel(sum[1]),
                channel(sum[2]),
                u8::try_from(sum[3] / count).unwrap_or(u8::MAX),
            ]
        })
    }

    fn mapped(&self, color: impl Fn([u8; 4]) -> [u8; 4]) -> Self {
        Self::filled(self.width, self.height, |x, y| color(self.at(x, y)))
    }

    fn layered(&self, top: &Self, left: u32, top_y: u32) -> Self {
        Self::filled(self.width, self.height, |x, y| {
            let under = self.at(x, y);
            if x < left || y < top_y || x - left >= top.width || y - top_y >= top.height {
                return under;
            }
            let over = top.at(x - left, y - top_y);
            let alpha = u16::from(over[3]);
            let mix = |front: u8, back: u8| {
                u8::try_from(
                    (u16::from(front) * alpha + u16::from(back) * (255 - alpha) + 127) / 255,
                )
                .unwrap_or(u8::MAX)
            };
            [
                mix(over[0], under[0]),
                mix(over[1], under[1]),
                mix(over[2], under[2]),
                under[3].max(over[3]),
            ]
        })
    }

    fn fitted(&self, cells: (u16, u16), cover: bool) -> io::Result<ImageSource> {
        let (width, height) = (u32::from(cells.0), u32::from(cells.1) * 2);
        let wider =
            u64::from(width) * u64::from(self.height) <= u64::from(height) * u64::from(self.width);
        let size = if wider == cover {
            (scaled(self.width, height, self.height), height)
        } else {
            (width, scaled(self.height, width, self.width))
        };
        self.resized(size.0, size.1).source()
    }

    fn source(&self) -> io::Result<ImageSource> {
        ImageSource::rgba8(
            self.width,
            self.height,
            self.rgba.clone().into_boxed_slice(),
        )
        .map_err(|error| invalid(error.to_string()))
    }

    fn tones(&self) -> [f32; 5] {
        let mut bins = [0_u32; 5];
        for pixel in self.rgba.as_chunks::<4>().0 {
            if pixel[3] < 128 {
                continue;
            }
            let band = [3, 29, 90, 190]
                .iter()
                .take_while(|edge| luma(*pixel) >= **edge)
                .count();
            bins[band] += 1;
        }
        let peak = bins.iter().copied().max().unwrap_or(1);
        bins.map(|bin| ratio(bin, peak))
    }
}

fn len(width: u32, height: u32) -> usize {
    usize::try_from(u64::from(width) * u64::from(height) * 4).unwrap_or(usize::MAX)
}

fn span(index: u32, out: u32, input: u32) -> (u32, u32) {
    let start = to_u32(u64::from(index) * u64::from(input) / u64::from(out));
    let end = to_u32(u64::from(index + 1) * u64::from(input) / u64::from(out));
    (start, end.max(start + 1).min(input))
}

fn to_u32(value: impl TryInto<u32>) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}

fn scaled(value: u32, numerator: u32, denominator: u32) -> u32 {
    let product = u64::from(value) * u64::from(numerator);
    let denominator = u64::from(denominator.max(1));
    to_u32((product + denominator / 2) / denominator).max(1)
}

fn ratio(part: u32, whole: u32) -> f32 {
    let permille = u64::from(part) * 1000 / u64::from(whole.max(1));
    f32::from(u16::try_from(permille).unwrap_or(u16::MAX)) / 1000.0
}

fn byte(value: u32) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

fn luma(pixel: [u8; 4]) -> u32 {
    (u32::from(pixel[0]) * 30 + u32::from(pixel[1]) * 59 + u32::from(pixel[2]) * 11) / 100
}

fn narrowband(sky: &Pixels, tint: [u32; 3]) -> Pixels {
    sky.mapped(|pixel| {
        let level = (luma(pixel).saturating_sub(15) * 1000 / 140).min(1000);
        let channel = |weight: u32| byte(6 + level * weight * 249 / 1_000_000);
        [channel(tint[0]), channel(tint[1]), channel(tint[2]), 255]
    })
}

fn library() -> io::Result<Vec<Item>> {
    let decode = |bytes: &[u8]| {
        ImageSource::decode(bytes)
            .map(|source| Pixels::of(&source))
            .map_err(|error| invalid(error.to_string()))
    };
    let sky = decode(include_bytes!("../site/assets/sky.webp"))?.resized(320, 180);
    let hills = decode(include_bytes!("../site/assets/hills.webp"))?;
    let moon = decode(include_bytes!("../site/assets/moon.webp"))?;

    let dawn = dawn(&hills);
    let horizon = horizon(&sky, &hills, &moon);

    let entries = [
        (
            "sgr09-rgb.png",
            "RGB stack",
            "1280 × 720",
            "2.6 MB",
            "Oct 27 02:14",
            "36 × 120 s",
            "#sgr09 #broadband",
            sky.clone(),
        ),
        (
            "sgr09-ha.fits",
            "H-alpha 7 nm",
            "1280 × 720",
            "3.5 MB",
            "Oct 27 02:51",
            "24 × 300 s",
            "#sgr09 #narrowband",
            narrowband(&sky, [1000, 240, 300]),
        ),
        (
            "sgr09-oiii.fits",
            "OIII 7 nm",
            "1280 × 720",
            "3.5 MB",
            "Oct 27 03:40",
            "24 × 300 s",
            "#sgr09 #narrowband",
            narrowband(&sky, [160, 780, 860]),
        ),
        (
            "moon-badge.png",
            "Badge, alpha",
            "1024 × 1024",
            "96 KB",
            "Oct 20 18:02",
            "drawn",
            "#badge #transparent",
            moon.resized(256, 256),
        ),
        (
            "ridge-dawn.png",
            "Panorama",
            "2048 × 651",
            "1.2 MB",
            "Oct 26 06:48",
            "1/250 s",
            "#ridge #dawn",
            dawn,
        ),
        (
            "horizon-pano.png",
            "Composite",
            "1280 × 720",
            "4.1 MB",
            "Oct 27 04:05",
            "3 layers",
            "#sgr09 #ridge #composite",
            horizon,
        ),
    ];
    let mut items = Vec::with_capacity(entries.len());
    for (name, kind, pixels, size, captured, exposure, tags, image) in entries {
        items.push(Item {
            name,
            kind,
            pixels,
            size,
            captured,
            exposure,
            tags,
            thumb: image.fitted(THUMB, true)?,
            contain: image.fitted(PREVIEW, false)?,
            cover: image.fitted(PREVIEW, true)?,
            tones: image.tones(),
        });
    }
    Ok(items)
}

fn dawn(hills: &Pixels) -> Pixels {
    Pixels::filled(512, 163, |_, y| {
        let t = (y * 1000 / 70).min(1000);
        let mix = |top: u32, glow: u32| byte(top + (glow - top) * t * t / 1_000_000);
        [mix(18, 238), mix(22, 150), mix(58, 104), 255]
    })
    .layered(&hills.resized(512, 163), 0, 0)
}

fn horizon(sky: &Pixels, hills: &Pixels, moon: &Pixels) -> Pixels {
    let ridge = hills.resized(320, 102).mapped(|[r, g, b, a]| {
        let shade = |value: u8| byte(u32::from(value) * 55 / 100);
        [shade(r), shade(g), shade(b), a]
    });
    sky.layered(&ridge, 0, 78)
        .layered(&moon.resized(60, 60), 196, 14)
}

fn main() -> io::Result<()> {
    let options = parse_options(&env::args().skip(1).collect::<Vec<_>>())?;
    let items = library()?;

    if options.snapshot {
        let mut grid = Grid::new(options.width, options.height);
        let mut clusters = ClusterTable::new();
        draw(&mut grid, &mut clusters, 0, &items);
        return io::stdout()
            .lock()
            .write_all(grid.display_text(&clusters).as_bytes());
    }

    let mut output = io::stdout().lock();
    output.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let result = play(&mut output, &options, &items);
    output.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    output.flush()?;
    result
}

fn play(output: &mut impl Write, options: &Options, items: &[Item]) -> io::Result<()> {
    let mut previous = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let period = Duration::from_secs(1) / options.fps;
    let started = Instant::now();

    for tick in 0..options.frames {
        let mut next = Grid::new(options.width, options.height);
        draw(&mut next, &mut clusters, tick * 1000 / options.fps, items);
        output.write_all(encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor))?;
        output.flush()?;
        previous = next;
        let deadline = started + period * (tick + 1);
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
    Ok(())
}

fn theme() -> Theme {
    Theme::with_overrides(
        Variant::Dark,
        &[
            (Token::Surface0, BG),
            (Token::Surface1, PANEL),
            (Token::TextPrimary, INK),
            (Token::Accent, LAV),
            (Token::FocusRing, LAV),
        ],
    )
}

struct Moment {
    ms: u32,
    selected: usize,
    cover: bool,
    key: Option<&'static str>,
}

impl Moment {
    fn at(ms: u32) -> Self {
        let selected = SELECTION
            .iter()
            .rev()
            .find(|(start, _)| ms >= *start)
            .map_or(0, |(_, index)| *index);
        let cover = FIT_KEYS.iter().filter(|start| ms >= **start).count() % 2 == 1;
        let recent = |start: u32| ms >= start && ms < start + 450;
        let key = if SELECTION.iter().skip(1).any(|(start, _)| recent(*start)) {
            Some(" → ")
        } else if FIT_KEYS.iter().any(|start| recent(*start)) {
            Some(" f ")
        } else {
            None
        };
        Self {
            ms,
            selected,
            cover,
            key,
        }
    }

    fn importing(&self, index: usize) -> Option<f32> {
        IMPORTS
            .iter()
            .find(|(item, done)| *item == index && self.ms < *done)
            .map(|(_, done)| ratio(self.ms, *done))
    }

    fn ready(&self, count: usize) -> usize {
        (0..count)
            .filter(|index| self.importing(*index).is_none())
            .count()
    }
}

fn draw(grid: &mut Grid, clusters: &mut ClusterTable, ms: u32, items: &[Item]) {
    let area = Rect::new(0, 0, grid.width(), grid.height());
    let theme = theme();
    let mut cx = RenderCx::new(grid, clusters, &theme);
    Block::new().style(on(INK, BG)).render(area, &mut cx);
    if area.width < 120 || area.height < 36 {
        return;
    }
    let moment = Moment::at(ms);
    draw_header(&mut cx, area, &moment, items.len());
    draw_library(
        &mut cx,
        Rect::new(0, 1, 68, area.height - 2),
        &moment,
        items,
    );
    let right = Rect::new(68, 1, area.width - 68, area.height - 2);
    draw_preview(
        &mut cx,
        Rect::new(right.x, right.y, right.width, 21),
        &moment,
        items,
    );
    draw_details(
        &mut cx,
        Rect::new(right.x, right.y + 21, right.width, right.height - 21),
        &items[moment.selected],
    );
    draw_footer(&mut cx, area, &moment, items.len());
}

fn draw_header(cx: &mut RenderCx<'_>, area: Rect, moment: &Moment, count: usize) {
    Block::new()
        .style(on(INK, RAISED))
        .render(Rect::new(0, 0, area.width, 1), cx);
    chip(cx, 0, 0, " Archive ", on(BG, LAV).with_attrs(bold()));
    chip(
        cx,
        9,
        0,
        " Survey 09 › Night sky › Frames ",
        on(MUTED, RAISED),
    );
    let seconds = 7 + moment.ms / 1000;
    let clock = format!(" 04:14:{seconds:02} UTC ");
    let ready = moment.ready(count);
    let (sync, sync_style) = if ready < count {
        (
            format!(" syncing {} ", count - ready),
            on(BG, MOON).with_attrs(bold()),
        )
    } else {
        (" synced ".to_owned(), on(BG, GREEN).with_attrs(bold()))
    };
    let clock_x = area.width - width(&clock);
    chip(cx, clock_x, 0, &clock, on(MUTED, RAISED));
    chip(cx, clock_x - width(&sync) - 1, 0, &sync, sync_style);
}

fn draw_library(cx: &mut RenderCx<'_>, area: Rect, moment: &Moment, items: &[Item]) {
    let ready = moment.ready(items.len());
    let summary = format!(" {} frames · {ready} ready ", items.len());
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(on(INK, PANEL))
        .border_style(on(LINE, PANEL))
        .title(" Library ")
        .title_style(on(LAV, PANEL).with_attrs(bold()))
        .title_inset(1)
        .title_at(
            summary,
            TitlePosition::Top,
            Alignment::Right,
            on(MUTED, PANEL),
        );
    block.render(area, cx);
    let inner = block.inner(area);

    for (index, item) in items.iter().enumerate() {
        let column = u16::try_from(index % 3).unwrap_or(0);
        let row = u16::try_from(index / 3).unwrap_or(0);
        let tile = Rect::new(inner.x + 1 + column * 22, inner.y + 1 + row * 15, 20, 13);
        draw_tile(
            cx,
            tile,
            item,
            index == moment.selected,
            moment.importing(index),
        );
    }

    let used = 18.2
        + 4.1 * (1.0 - moment.importing(5).map_or(0.0, |done| 1.0 - done))
        + 1.2 * (1.0 - moment.importing(4).map_or(0.0, |done| 1.0 - done));
    let storage_y = inner.y + inner.height - 1;
    label(cx, inner.x + 1, storage_y, 8, "Storage", MUTED, PANEL);
    meter(
        cx,
        Rect::new(inner.x + 10, storage_y, 40, 1),
        used / 64.0,
        LAV,
    );
    label(
        cx,
        inner.x + 52,
        storage_y,
        13,
        &format!("{used:.1} / 64 GB"),
        MUTED,
        PANEL,
    );
}

fn draw_tile(
    cx: &mut RenderCx<'_>,
    tile: Rect,
    item: &Item,
    selected: bool,
    importing: Option<f32>,
) {
    let frame = Rect::new(tile.x, tile.y, tile.width, THUMB.1 + 2);
    let image = Image::new(item.thumb.clone()).fit(ImageFit::Cover);
    if selected {
        image.state(WidgetState::Focused).render(frame, cx);
    } else {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(on(LINE, PANEL));
        block.render(frame, cx);
        let state = if importing.is_some() {
            WidgetState::Disabled
        } else {
            WidgetState::Normal
        };
        image.state(state).render(block.inner(frame), cx);
    }

    let name_style = if selected {
        on(LAV, PANEL).with_attrs(bold())
    } else {
        on(INK, PANEL)
    };
    Paragraph::new(item.name).style(name_style).render(
        Rect::new(tile.x + 1, frame.y + frame.height, tile.width - 1, 1),
        cx,
    );
    let meta_y = frame.y + frame.height + 1;
    if let Some(done) = importing {
        meter(cx, Rect::new(tile.x + 1, meta_y, 12, 1), done, MOON);
        label(
            cx,
            tile.x + 14,
            meta_y,
            5,
            &format!("{:>3.0}%", done * 100.0),
            MOON,
            PANEL,
        );
    } else {
        label(
            cx,
            tile.x + 1,
            meta_y,
            tile.width - 1,
            &format!("{} · {}", item.pixels, item.size),
            DIM,
            PANEL,
        );
    }
}

fn draw_preview(cx: &mut RenderCx<'_>, area: Rect, moment: &Moment, items: &[Item]) {
    let item = &items[moment.selected];
    let (fit, source, fit_label) = if moment.cover {
        (ImageFit::Cover, &item.cover, " fit: cover ")
    } else {
        (ImageFit::Contain, &item.contain, " fit: contain ")
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(on(INK, BG))
        .border_style(on(LINE, BG))
        .title(" Preview ")
        .title_style(on(LAV, BG).with_attrs(bold()))
        .title_inset(1)
        .title_at(
            fit_label,
            TitlePosition::Top,
            Alignment::Right,
            on(MOON, BG),
        );
    block.render(area, cx);
    let inner = block.inner(area);
    Image::new(source.clone()).fit(fit).render(inner, cx);
}

fn draw_details(cx: &mut RenderCx<'_>, area: Rect, item: &Item) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .style(on(INK, PANEL))
        .border_style(on(LINE, PANEL))
        .title(" Details ")
        .title_style(on(LAV, PANEL).with_attrs(bold()))
        .title_inset(1);
    block.render(area, cx);
    let inner = block.inner(area);
    let x = inner.x + 1;

    Paragraph::new(item.name)
        .style(on(INK, PANEL).with_attrs(bold()))
        .render(Rect::new(x, inner.y, 26, 1), cx);
    label(cx, x, inner.y + 1, 26, item.kind, MUTED, PANEL);
    let rows = [
        ("Pixels", item.pixels),
        ("Size", item.size),
        ("Captured", item.captured),
        ("Exposure", item.exposure),
    ];
    for (offset, (key, value)) in (3_u16..).zip(rows) {
        label(cx, x, inner.y + offset, 10, key, DIM, PANEL);
        label(cx, x + 10, inner.y + offset, 16, value, INK, PANEL);
    }
    label(cx, x, inner.y + inner.height - 1, 26, item.tags, LAV, PANEL);

    let tones_x = inner.x + 28;
    label(cx, tones_x, inner.y, 20, "Tones", MUTED, PANEL);
    let bands = ["deep", "shadow", "mid", "light", "high"];
    let colors = [
        Color::Rgb(84, 92, 150),
        Color::Rgb(120, 116, 196),
        LAV,
        Color::Rgb(214, 204, 236),
        MOON,
    ];
    for (offset, ((band, share), color)) in (2_u16..).zip(bands.iter().zip(item.tones).zip(colors))
    {
        label(cx, tones_x, inner.y + offset, 7, band, DIM, PANEL);
        meter(
            cx,
            Rect::new(tones_x + 7, inner.y + offset, 14, 1),
            share,
            color,
        );
    }
}

fn draw_footer(cx: &mut RenderCx<'_>, area: Rect, moment: &Moment, count: usize) {
    let y = area.height - 1;
    Block::new()
        .style(on(MUTED, RAISED))
        .render(Rect::new(0, y, area.width, 1), cx);
    StatusBar::new("  ← → select    f fit    enter open    / search")
        .style(on(MUTED, RAISED))
        .render(Rect::new(0, y, area.width, 1), cx);
    let position = format!(" {} / {count} ", moment.selected + 1);
    let position_x = area.width - width(&position);
    chip(cx, position_x, y, &position, on(BG, LAV).with_attrs(bold()));
    if let Some(key) = moment.key {
        chip(
            cx,
            position_x - width(key) - 1,
            y,
            key,
            on(BG, MOON).with_attrs(bold()),
        );
    }
}

fn meter(cx: &mut RenderCx<'_>, area: Rect, share: f32, color: Color) {
    Block::new().style(on(INK, TRACK)).render(area, cx);
    Gauge::new(share)
        .gauge_style(on(color, TRACK))
        .render(area, cx);
}

fn chip(cx: &mut RenderCx<'_>, x: u16, y: u16, text: &str, style: Style) {
    StatusBar::new(text)
        .style(style)
        .render(Rect::new(x, y, width(text), 1), cx);
}

fn label(cx: &mut RenderCx<'_>, x: u16, y: u16, width: u16, text: &str, fg: Color, bg: Color) {
    Paragraph::new(text)
        .style(on(fg, bg))
        .render(Rect::new(x, y, width, 1), cx);
}

fn width(text: &str) -> u16 {
    u16::try_from(lunatui_core::text_display_width(text)).unwrap_or(u16::MAX)
}

fn on(fg: Color, bg: Color) -> Style {
    Style::new().fg(fg).bg(bg)
}

fn bold() -> Attrs {
    Attrs::new().set_bold(true)
}

fn parse_options(args: &[String]) -> io::Result<Options> {
    let mut options = Options {
        width: 120,
        height: 36,
        frames: 300,
        fps: 30,
        snapshot: false,
    };
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--snapshot" {
            options.snapshot = true;
            index += 1;
            continue;
        }
        let Some(value) = args.get(index + 1) else {
            return Err(invalid(format!("missing value for {}", args[index])));
        };
        match args[index].as_str() {
            "--width" => options.width = parse_u16(value, "width")?,
            "--height" => options.height = parse_u16(value, "height")?,
            "--frames" => options.frames = parse_u32(value, "frames")?,
            "--fps" => options.fps = parse_u32(value, "fps")?,
            flag => return Err(invalid(format!("unknown argument: {flag}"))),
        }
        index += 2;
    }
    if options.width == 0 || options.height == 0 || options.frames == 0 || options.fps == 0 {
        return Err(invalid("width, height, frames and fps must be positive"));
    }
    Ok(options)
}

fn parse_u16(value: &str, name: &str) -> io::Result<u16> {
    value
        .parse()
        .map_err(|_| invalid(format!("{name} must be a positive integer")))
}

fn parse_u32(value: &str, name: &str) -> io::Result<u32> {
    value
        .parse()
        .map_err(|_| invalid(format!("{name} must be a positive integer")))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
