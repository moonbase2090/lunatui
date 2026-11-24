// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use lunatui_badge::{Badge, Kbd, Look, Marker};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_term::{ColorMode, Encoder, PixelImage, PixelLayer, PixelMode};
use lunatui_text::source::VecSource;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_ui_tokens::Tone;
use lunatui_widgets::{List, RenderCx, Sparkline, SparklineBar, Widget};

use crate::graphite_cases::{CELL_HEIGHT, CELL_WIDTH, Repaint};
use crate::{CountingWriter, WriteStats};

/// The most images `PixelLayer` lists in one frame.
pub const IMAGE_LIMIT: usize = 256;

/// Which screen the widgets draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    /// The activity monitor: a header, up to eight seats with lanes, a detail list and key hints.
    Monitor,
    /// The worst case: a seat row with a lane on every row between the header and the key hints,
    /// across the whole width.
    Dense,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Working,
    Waiting,
    Idle,
    Stuck,
    Failed,
    Down,
}

impl State {
    const ALL: [Self; 6] = [
        Self::Working,
        Self::Waiting,
        Self::Idle,
        Self::Stuck,
        Self::Failed,
        Self::Down,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Waiting => "waiting",
            Self::Idle => "idle",
            Self::Stuck => "stuck",
            Self::Failed => "failed",
            Self::Down => "down",
        }
    }

    const fn tone(self) -> Tone {
        match self {
            Self::Working => Tone::Success,
            Self::Waiting | Self::Stuck => Tone::Warning,
            Self::Failed => Tone::Danger,
            Self::Idle | Self::Down => Tone::Neutral,
        }
    }

    const fn marker(self) -> Marker {
        match self {
            Self::Working => Marker::Dot,
            Self::Waiting => Marker::Diamond,
            Self::Idle => Marker::Ring,
            Self::Stuck => Marker::Bang,
            Self::Failed => Marker::Cross,
            Self::Down => Marker::Square,
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Working => Color::Rgb(74, 222, 128),
            Self::Waiting | Self::Stuck => Color::Rgb(250, 190, 50),
            Self::Failed => Color::Rgb(248, 113, 113),
            Self::Idle | Self::Down => Color::Rgb(148, 163, 184),
        }
    }
}

const ACTIVITIES: [&str; 6] = [
    "running the workspace tests",
    "waiting for review",
    "idle",
    "no output for 20 minutes",
    "build failed",
    "pane closed",
];

const HINTS: [&[&str]; 7] = [
    &["↑", "↓"],
    &["⏎"],
    &["d"],
    &["Tab"],
    &["f"],
    &["s"],
    &["q"],
];

const MAX_SEATS: u16 = 8;
const LANE_MAX: u64 = 8;

fn mix(seat: u16, salt: u64, tick: u32) -> u64 {
    let mut x = (u64::from(seat) << 32) ^ salt.wrapping_add(u64::from(tick));
    x = x.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^ (x >> 29)
}

fn state(seat: u16, tick: u32) -> State {
    let index = usize::try_from(mix(seat, 7, tick / 4) % 6).unwrap_or(0);
    State::ALL[index]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    cols: u16,
    rows: u16,
    seats: u16,
    split: u16,
    list: u16,
}

impl Layout {
    fn new(cols: u16, rows: u16, density: Density) -> Result<Self, String> {
        if cols < 40 || rows < 8 {
            return Err(format!("{cols}x{rows} cells is too small for the monitor"));
        }
        let (seats, split) = match density {
            Density::Monitor => ((rows - 4).min(MAX_SEATS), cols * 11 / 20),
            Density::Dense => (rows - 3, cols),
        };
        Ok(Self {
            cols,
            rows,
            seats,
            split,
            list: (split / 2).min(40),
        })
    }

    const fn changed(self) -> Rect {
        Rect::new(0, 2, self.split, self.seats)
    }
}

/// The cell rectangle a changed frame redraws for a `cols`×`rows` window.
///
/// # Errors
///
/// Returns an error when the window is too small for the scene.
pub fn changed_cells(cols: u16, rows: u16, density: Density) -> Result<Rect, String> {
    Layout::new(cols, rows, density).map(Layout::changed)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

struct Scene {
    layout: Layout,
    background: Style,
    text: Style,
    muted: Style,
    highlight: Style,
    accent: Style,
    tick: u32,
}

impl Scene {
    fn full(&self, cx: &mut RenderCx<'_>) {
        let area = Rect::new(0, 0, self.layout.cols, self.layout.rows);
        cx.fill(area, self.background, ' ');
        self.header(cx);
        self.seats(cx);
        self.detail(cx);
        self.footer(cx);
    }

    fn changed(&self, cx: &mut RenderCx<'_>) {
        cx.fill(self.layout.changed(), self.background, ' ');
        self.seats(cx);
    }

    fn header(&self, cx: &mut RenderCx<'_>) {
        let cols = self.layout.cols;
        let mut x = 1;
        for kind in State::ALL {
            let count = (0..self.layout.seats)
                .filter(|&seat| state(seat, self.tick) == kind)
                .count();
            let words = format!("{count} {}", kind.name());
            let badge = Badge::new(&words)
                .tone(kind.tone())
                .look(Look::Subtle)
                .marker(kind.marker());
            let width = badge.measure(Size::new(cols.saturating_sub(x), 1)).width;
            if width == 0 || x + width > cols {
                break;
            }
            badge.render(Rect::new(x, 0, width, 1), cx);
            x += width + 1;
        }
    }

    fn seats(&self, cx: &mut RenderCx<'_>) {
        let Layout {
            seats, split, list, ..
        } = self.layout;
        let rows: Vec<Vec<String>> = (0..seats)
            .map(|seat| {
                let doing =
                    ACTIVITIES[usize::try_from(mix(seat, 3, self.tick / 4) % 6).unwrap_or(0)];
                vec![format!("seat {:02} {doing}", seat + 1)]
            })
            .collect();
        let icons = (0..seats)
            .map(|seat| {
                let kind = state(seat, self.tick);
                (
                    kind.marker().glyph().to_string(),
                    Style::new().fg(kind.color()),
                )
            })
            .collect();
        let marks = (0..seats)
            .map(|seat| Some((format!("{}s", mix(seat, 5, self.tick) % 600), self.muted)))
            .collect();
        let selected = usize::from(u16::try_from(self.tick / 8).unwrap_or(0) % seats);
        List::new(VecSource::from_rows(rows))
            .style(self.text)
            .highlight_style(self.highlight)
            .highlight_symbol("▌", self.accent)
            .icons(icons)
            .marks(marks, 1)
            .selected(Some(selected))
            .render(Rect::new(0, 2, list, seats), cx);
        let lane = split.saturating_sub(list + 2);
        if lane == 0 {
            return;
        }
        for seat in 0..seats {
            let color = state(seat, self.tick).color();
            let bars: Vec<SparklineBar> = (0..lane)
                .map(|bin| {
                    let value = mix(seat, u64::from(bin) + 11, self.tick) % (LANE_MAX + 1);
                    SparklineBar::value(value).style(Style::new().fg(color))
                })
                .collect();
            let row = if usize::from(seat) == selected {
                self.highlight
            } else {
                self.background
            };
            Sparkline::new(&bars)
                .max(LANE_MAX)
                .style(Style::new().bg(row.bg))
                .render(Rect::new(list + 1, 2 + seat, lane, 1), cx);
        }
    }

    fn detail(&self, cx: &mut RenderCx<'_>) {
        let Layout {
            cols, rows, split, ..
        } = self.layout;
        if split + 2 >= cols {
            return;
        }
        let height = rows - 4;
        let events: Vec<Vec<String>> = (0..height)
            .map(|row| {
                let what = ACTIVITIES[usize::try_from(mix(row, 13, 0) % 6).unwrap_or(0)];
                vec![format!("{what}, step {}", row + 1)]
            })
            .collect();
        let marks = (0..height)
            .map(|row| Some((format!("{}m", u32::from(row) * 3 + 1), self.muted)))
            .collect();
        List::new(VecSource::from_rows(events))
            .style(self.muted)
            .marks(marks, 1)
            .render(Rect::new(split + 1, 2, cols - split - 2, height), cx);
    }

    fn footer(&self, cx: &mut RenderCx<'_>) {
        let Layout { cols, rows, .. } = self.layout;
        let mut x = 1;
        for keys in HINTS {
            let kbd = Kbd::new(keys);
            let width = kbd.measure(Size::new(cols.saturating_sub(x), 1)).width;
            if width == 0 || x + width > cols {
                break;
            }
            kbd.render(Rect::new(x, rows - 1, width, 1), cx);
            x += width + 2;
        }
    }
}

struct Host {
    cell: (u32, u32),
    background: Option<[u8; 3]>,
    images: Vec<(Rect, ImageSource)>,
}

impl ImageSink for Host {
    fn supports_pixels(&self) -> bool {
        true
    }

    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        let size = PixelSize::new(
            u32::from(area.width) * self.cell.0,
            u32::from(area.height) * self.cell.1,
        )?;
        Some(PixelTarget::new(size, false, self.background))
    }

    fn submit(&mut self, _: ImageRequest<'_>) -> bool {
        false
    }

    fn submit_raster(&mut self, source: &ImageSource, area: Rect, _: i32) -> bool {
        self.images.push((area, source.clone()));
        true
    }
}

struct Placed {
    area: Rect,
    source: ImageSource,
}

fn pixel_image(source: &ImageSource) -> io::Result<PixelImage<'_>> {
    let size = source.size();
    PixelImage::new(size.width(), size.height(), source.rgba8_pixels())
        .ok_or_else(|| io::Error::other("a widget image is not a valid terminal image"))
}

fn rgb(color: Color) -> Option<[u8; 3]> {
    match color {
        Color::Rgb(red, green, blue) => Some([red, green, blue]),
        _ => None,
    }
}

struct Frames {
    layout: Layout,
    repaint: Repaint,
    theme: Theme,
    prev: Grid,
    next: Grid,
    clusters: ClusterTable,
    encoder: Encoder,
    layer: PixelLayer,
    host: Host,
    kept: Vec<Placed>,
    tick: u32,
    images: usize,
}

const KEPT_GENERATION: u64 = 1;
const CHANGED_IDS: u32 = 1024;

impl Frames {
    fn new(
        density: Density,
        repaint: Repaint,
        scale: usize,
        cols: u16,
        rows: u16,
    ) -> Result<Self, String> {
        let layout = Layout::new(cols, rows, density)?;
        let theme = Theme::builtin(Variant::Dark);
        let background =
            rgb(theme.resolve(Token::Surface0)).ok_or("the theme background is not RGB")?;
        let narrow = |value: usize| u32::try_from(value).map_err(|error| error.to_string());
        let mut run = Self {
            layout,
            repaint,
            prev: Grid::new(cols, rows),
            next: Grid::new(cols, rows),
            clusters: ClusterTable::new(),
            encoder: Encoder::new(),
            layer: PixelLayer::new(PixelMode::Kitty),
            host: Host {
                cell: (narrow(CELL_WIDTH * scale)?, narrow(CELL_HEIGHT * scale)?),
                background: Some(background),
                images: Vec::new(),
            },
            kept: Vec::new(),
            tick: 0,
            images: 0,
            theme,
        };
        if repaint == Repaint::Changed {
            run.draw(Repaint::Full);
            let changed = layout.changed();
            run.kept = run
                .host
                .images
                .drain(..)
                .filter(|(area, _)| !overlaps(*area, changed))
                .map(|(area, source)| Placed { area, source })
                .collect();
            run.layer.begin_frame();
            for (index, placed) in run.kept.iter().enumerate() {
                let id = u32::try_from(index + 1).map_err(|error| error.to_string())?;
                let image = pixel_image(&placed.source).map_err(|error| error.to_string())?;
                run.layer.image(id, KEPT_GENERATION, &image, placed.area, 0);
            }
            let _ = run.layer.finish_frame();
            run.prev.clone_from(&run.next);
        }
        Ok(run)
    }

    fn draw(&mut self, repaint: Repaint) {
        let theme = &self.theme;
        let background = Style::new().bg(theme.resolve(Token::Surface0));
        let scene = Scene {
            layout: self.layout,
            background,
            text: background.fg(theme.resolve(Token::TextPrimary)),
            muted: background.fg(theme.resolve(Token::TextMuted)),
            highlight: Style::new()
                .bg(theme.resolve(Token::Surface1))
                .fg(theme.resolve(Token::TextPrimary))
                .with_attrs(Attrs::new().set_bold(true)),
            accent: Style::new().fg(theme.resolve(Token::Accent)),
            tick: self.tick,
        };
        self.host.images.clear();
        let mut cx = RenderCx::new(&mut self.next, &mut self.clusters, theme)
            .image_sink(&mut self.host)
            .render_mode(RenderMode::Graphite);
        match repaint {
            Repaint::Full => scene.full(&mut cx),
            Repaint::Changed => scene.changed(&mut cx),
        }
    }

    fn frame_into<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        if self.repaint == Repaint::Changed {
            self.next.clone_from(&self.prev);
        }
        self.draw(self.repaint);
        let cells =
            self.encoder
                .encode_diff(&self.prev, &self.next, &self.clusters, ColorMode::TrueColor);
        out.write_all(cells)?;
        let generation = u64::from(self.tick) + 2;
        self.images = self.kept.len() + self.host.images.len();
        if self.images > IMAGE_LIMIT {
            return Err(io::Error::other(format!(
                "{} images is more than PixelLayer lists in a frame",
                self.images
            )));
        }
        self.layer.begin_frame();
        for (index, placed) in self.kept.iter().enumerate() {
            let id = u32::try_from(index + 1).map_err(io::Error::other)?;
            self.layer.image(
                id,
                KEPT_GENERATION,
                &pixel_image(&placed.source)?,
                placed.area,
                0,
            );
        }
        for (index, (area, source)) in self.host.images.iter().enumerate() {
            let id = CHANGED_IDS + u32::try_from(index + 1).map_err(io::Error::other)?;
            self.layer
                .image(id, generation, &pixel_image(source)?, *area, 0);
        }
        out.write_all(self.layer.finish_frame())?;
        std::mem::swap(&mut self.prev, &mut self.next);
        self.tick = self.tick.wrapping_add(1);
        Ok(())
    }
}

/// The activity monitor drawn by widgets in Graphite mode, at one window size and scale.
pub struct MonitorRun {
    frames: Frames,
    sink: CountingWriter<io::Sink>,
}

impl MonitorRun {
    /// Sets up the scene for a window of `cols`×`rows` cells at `scale` (1 or 2).
    ///
    /// A changed run draws the whole window once here, keeps the images outside the seat rows,
    /// and then redraws only the seat rows each frame.
    ///
    /// # Errors
    ///
    /// Returns an error when the window is too small or the theme has no RGB background.
    pub fn new(
        density: Density,
        repaint: Repaint,
        scale: usize,
        cols: u16,
        rows: u16,
    ) -> Result<Self, String> {
        Ok(Self {
            frames: Frames::new(density, repaint, scale, cols, rows)?,
            sink: CountingWriter::new(io::sink()),
        })
    }

    /// Frames drawn so far.
    #[must_use]
    pub const fn tick(&self) -> u32 {
        self.frames.tick
    }

    /// Images listed in the last frame, including those kept from the first draw.
    #[must_use]
    pub const fn images(&self) -> usize {
        self.frames.images
    }

    /// Bytes and `write()` calls sent to the counting sink so far.
    #[must_use]
    pub fn stats(&self) -> WriteStats {
        self.sink.stats()
    }

    /// The cells of the last frame, with the cluster table to read them.
    #[must_use]
    pub const fn grid(&self) -> (&Grid, &ClusterTable) {
        (&self.frames.prev, &self.frames.clusters)
    }

    /// Draws the next tick, encodes the cell diff and the widget images, and writes them to the
    /// counting sink.
    ///
    /// # Errors
    ///
    /// Returns an error if an image is invalid, the frame has more images than `PixelLayer`
    /// lists, or writing fails.
    pub fn frame(&mut self) -> io::Result<()> {
        self.frames.frame_into(&mut self.sink)
    }

    /// Like [`frame`](Self::frame), but writes the frame's bytes to `out` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if an image is invalid, the frame has more images than `PixelLayer`
    /// lists, or writing fails.
    pub fn frame_into<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        self.frames.frame_into(out)
    }
}
