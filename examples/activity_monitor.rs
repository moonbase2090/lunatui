// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use lunatui_badge::{Badge, Kbd, Look, Marker};
use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_image::{ImageRequest, ImageSink, ImageSource, PixelSize, PixelTarget};
use lunatui_runtime::RenderMode;
use lunatui_term::{ColorMode, Encoder};
use lunatui_text::source::VecSource;
use lunatui_theme::{Theme, Token, Variant};
use lunatui_ui_tokens::Tone;
use lunatui_widgets::graphite::{Face, Rect as PixelRect};
use lunatui_widgets::{List, RenderCx, Sparkline, SparklineBar, Widget};

const COLUMNS: u16 = 96;
const ROWS: u16 = 26;
const CELL: (u32, u32) = (16, 32);
const FRAMES: usize = 10;
const SPLIT: u16 = 48;
const LANE_BINS: usize = 16;
const DETAIL_BINS: u16 = 46;

type Failure = Box<dyn Error>;

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

    fn color(self, cx: &RenderCx<'_>) -> Color {
        match self.tone() {
            Tone::Neutral => cx.token(Token::TextMuted),
            tone => cx.token(tone.fill()),
        }
    }
}

struct Seat {
    name: &'static str,
    activity: &'static str,
    state: State,
    age: u32,
}

const KINDS: [(&str, State, char); 5] = [
    ("test_failed", State::Failed, '×'),
    ("test_started", State::Working, '▸'),
    ("edit_finished", State::Idle, '·'),
    ("command_started", State::Working, '▸'),
    ("waiting", State::Waiting, '◆'),
];

const HINTS: [(&[&str], &str); 9] = [
    (&["↑", "↓"], "move"),
    (&["⏎"], "timeline"),
    (&["d"], "details"),
    (&["Tab"], "host"),
    (&["f"], "filter"),
    (&["s"], "sort"),
    (&["i"], "IDs"),
    (&["?"], "keys"),
    (&["q"], "quit"),
];

fn seats(frame: usize) -> [Seat; 3] {
    let tick = u32::try_from(frame).unwrap_or(0);
    let builder = if frame < 4 {
        State::Working
    } else {
        State::Failed
    };
    let reviewer = if frame % 4 == 3 {
        State::Working
    } else {
        State::Waiting
    };
    [
        Seat {
            name: "builder",
            activity: if frame < 4 {
                "test_started"
            } else {
                "test_failed"
            },
            state: builder,
            age: 50 + tick,
        },
        Seat {
            name: "reviewer",
            activity: if reviewer == State::Working {
                "edit_finished"
            } else {
                "waiting"
            },
            state: reviewer,
            age: 49 + tick,
        },
        Seat {
            name: "worker",
            activity: "command_started",
            state: State::Working,
            age: 48 + tick,
        },
    ]
}

fn activity(seat: usize, bin: usize) -> u64 {
    let mixed = (bin * 7 + seat * 13 + (bin * bin) % 11) % 9;
    u64::try_from(mixed).unwrap_or(0).saturating_sub(1)
}

fn lane(
    cx: &RenderCx<'_>,
    seat: usize,
    bins: usize,
    frame: usize,
    end: State,
) -> Vec<SparklineBar> {
    (0..bins)
        .map(|bin| {
            let at = bin + frame;
            let state = if bin + 6 >= bins {
                end
            } else if (at / 5 + seat).is_multiple_of(3) {
                State::Waiting
            } else {
                State::Working
            };
            SparklineBar::value(activity(seat, at)).style(Style::new().fg(state.color(cx)))
        })
        .collect()
}

#[derive(Default)]
struct Host {
    background: Option<[u8; 3]>,
    images: Vec<(Rect, ImageSource)>,
}

impl ImageSink for Host {
    fn supports_pixels(&self) -> bool {
        true
    }

    fn pixel_target(&self, area: Rect) -> Option<PixelTarget> {
        let size = PixelSize::new(
            u32::from(area.width) * CELL.0,
            u32::from(area.height) * CELL.1,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Right,
}

fn label(cx: &mut RenderCx<'_>, area: Rect, text: &str, style: Style, size: f32, align: Align) {
    let face = if style.attrs.bold() {
        Face::SemiBold
    } else {
        Face::Regular
    };
    let cells = u16::try_from(lunatui_core::width(text))
        .unwrap_or(u16::MAX)
        .min(area.width);
    let x = match align {
        Align::Left => area.x,
        Align::Right => area.x + area.width - cells,
    };
    cx.put_str(
        x,
        area.y,
        lunatui_core::clip_display_prefix(text, area.width),
        style,
    );
    cx.replace_with_pixels(area, |px| {
        let height = f32::from(px.cell_size().height_px());
        let size = height * size;
        let room = f32::from(u16::try_from(px.width()).unwrap_or(u16::MAX));
        let shown = px.fit(face, size, text, room).into_owned();
        let left = match align {
            Align::Left => 0.0,
            Align::Right => room - px.measure(face, size, &shown),
        };
        let baseline = px.line_metrics(face, size).map_or(height * 0.7, |line| {
            (height + line.ascent - line.descent) / 2.0
        });
        px.text(face, size, (left, baseline), &shown, style.fg);
    });
}

fn rule(cx: &mut RenderCx<'_>, area: Rect, style: Style) {
    for y in area.y..area.y + area.height {
        cx.put_str(area.x, y, "│", style);
    }
    cx.replace_with_pixels(area, |px| {
        let width = usize::try_from(px.width()).unwrap_or(0);
        let height = usize::try_from(px.height()).unwrap_or(0);
        px.fill_rect_with_opacity(
            PixelRect {
                x: width / 2,
                y: 0,
                w: 1,
                h: height,
            },
            0.0,
            style.fg,
            0.6,
        );
    });
}

#[derive(Clone, Copy)]
struct Ink {
    background: Style,
    text: Style,
    muted: Style,
    bold: Attrs,
}

fn draw(cx: &mut RenderCx<'_>, frame: usize) {
    let area = Rect::new(0, 0, COLUMNS, ROWS);
    let background = Style::new().bg(cx.token(Token::Surface0));
    let ink = Ink {
        background,
        text: background.fg(cx.token(Token::TextPrimary)),
        muted: background.fg(cx.token(Token::TextMuted)),
        bold: Attrs::new().set_bold(true),
    };
    cx.fill(area, background, ' ');
    let seats = seats(frame);
    let selected = frame / 4 % 3;
    header(cx, &ink, &seats);
    seat_rows(cx, &ink, &seats, selected, frame);
    rule(cx, Rect::new(SPLIT, 2, 1, ROWS - 4), ink.muted);
    detail(cx, &ink, &seats[selected], selected, frame);
    timeline(cx, &ink, &seats[selected], frame);
    footer(cx, &ink);
}

fn header(cx: &mut RenderCx<'_>, ink: &Ink, seats: &[Seat; 3]) {
    let Ink {
        background,
        text,
        muted,
        bold,
        ..
    } = *ink;
    label(
        cx,
        Rect::new(1, 0, 17, 1),
        "Activity monitor",
        text.with_attrs(bold),
        0.62,
        Align::Left,
    );
    let mut x = 20;
    for state in State::ALL {
        let count = seats.iter().filter(|seat| seat.state == state).count();
        let words = format!("{count} {}", state.name());
        let badge = Badge::new(&words)
            .tone(state.tone())
            .look(Look::Subtle)
            .marker(state.marker());
        let width = badge.measure(Size::new(COLUMNS - x, 1)).width;
        badge.render(Rect::new(x, 0, width, 1), cx);
        x += width + 1;
    }
    let waiting = seats
        .iter()
        .filter(|seat| seat.state != State::Working)
        .count();
    label(
        cx,
        Rect::new(1, 1, 7, 1),
        "sample",
        text.with_attrs(bold),
        0.56,
        Align::Left,
    );
    label(
        cx,
        Rect::new(9, 1, 21, 1),
        "connected · 3 seats ·",
        muted,
        0.5,
        Align::Left,
    );
    let warning = background.fg(cx.token(Token::Warn));
    label(
        cx,
        Rect::new(31, 1, 12, 1),
        &format!("{waiting} need you"),
        warning,
        0.5,
        Align::Left,
    );
}

fn seat_rows(cx: &mut RenderCx<'_>, ink: &Ink, seats: &[Seat; 3], selected: usize, frame: usize) {
    let Ink {
        background,
        text,
        bold,
        ..
    } = *ink;
    let names: Vec<Vec<String>> = seats
        .iter()
        .map(|seat| vec![seat.name.to_owned()])
        .collect();
    let icons = seats
        .iter()
        .map(|seat| {
            (
                seat.state.marker_glyph(),
                Style::new().fg(seat.state.color(cx)),
            )
        })
        .collect();
    let highlight = Style::new()
        .bg(cx.token(Token::Surface1))
        .fg(cx.token(Token::TextPrimary))
        .with_attrs(bold);
    List::new(VecSource::from_rows(names))
        .style(text.with_attrs(bold))
        .highlight_style(highlight)
        .highlight_symbol("▌", Style::new().fg(cx.token(Token::Accent)))
        .icons(icons)
        .selected(Some(selected))
        .render(Rect::new(0, 3, SPLIT - 1, 3), cx);
    for (index, seat) in seats.iter().enumerate() {
        let y = 3 + u16::try_from(index).unwrap_or(0);
        let row = if index == selected {
            highlight
        } else {
            background
        };
        label(
            cx,
            Rect::new(14, y, 16, 1),
            seat.activity,
            row.fg(cx.token(Token::TextMuted)),
            0.5,
            Align::Left,
        );
        label(
            cx,
            Rect::new(29, y, 4, 1),
            &format!("{}s", seat.age),
            row.fg(cx.token(Token::TextPrimary)),
            0.5,
            Align::Right,
        );
        Badge::new("stale")
            .look(Look::Subtle)
            .render(Rect::new(34, y, 7, 1), cx);
        let bars = lane(cx, index, LANE_BINS.min(6), frame, seat.state);
        Sparkline::new(&bars)
            .max(8)
            .style(Style::new().bg(row.bg))
            .render(Rect::new(41, y, 6, 1), cx);
    }
}

fn detail(cx: &mut RenderCx<'_>, ink: &Ink, focus: &Seat, selected: usize, frame: usize) {
    let Ink {
        background,
        text,
        muted,
        bold,
        ..
    } = *ink;
    let right = SPLIT + 2;
    label(
        cx,
        Rect::new(right, 3, 9, 1),
        focus.name,
        text.with_attrs(bold),
        0.62,
        Align::Left,
    );
    label(
        cx,
        Rect::new(right + 9, 3, 9, 1),
        "on sample",
        muted,
        0.5,
        Align::Left,
    );
    let status = format!("{} for {}s", focus.state.name(), focus.age + 4);
    let badge = Badge::new(&status)
        .tone(focus.state.tone())
        .look(Look::Subtle)
        .marker(focus.state.marker());
    let width = badge.measure(Size::new(30, 1)).width;
    badge.render(Rect::new(right + 19, 3, width, 1), cx);
    Badge::new("stale")
        .look(Look::Subtle)
        .render(Rect::new(right + 20 + width, 3, 7, 1), cx);
    label(
        cx,
        Rect::new(right, 4, 44, 1),
        &format!(
            "{} events · 1 task · {} failures · last {}s ago",
            80 + frame,
            17 + frame / 4,
            focus.age
        ),
        muted,
        0.5,
        Align::Left,
    );
    let detail = lane(cx, selected, usize::from(DETAIL_BINS), frame, focus.state);
    Sparkline::new(&detail)
        .max(8)
        .style(Style::new().bg(background.bg))
        .render(Rect::new(right, 6, DETAIL_BINS, 3), cx);
    label(
        cx,
        Rect::new(right, 9, 6, 1),
        "-10m",
        muted,
        0.46,
        Align::Left,
    );
    label(
        cx,
        Rect::new(right + 20, 9, 6, 1),
        "-5m",
        muted,
        0.46,
        Align::Left,
    );
    label(
        cx,
        Rect::new(right + 40, 9, 6, 1),
        "now",
        muted,
        0.46,
        Align::Right,
    );
}

fn timeline(cx: &mut RenderCx<'_>, ink: &Ink, focus: &Seat, frame: usize) {
    let Ink {
        background,
        text,
        muted,
        ..
    } = *ink;
    let right = SPLIT + 2;
    for row in 0..12_u16 {
        let y = 11 + row;
        let (kind, state, glyph) = KINDS[(usize::from(row) + frame) % KINDS.len()];
        let age = if row < 2 {
            format!("{}s", focus.age + u32::from(row) * 4)
        } else {
            format!("{}m", 1 + row / 10)
        };
        label(cx, Rect::new(right, y, 5, 1), &age, text, 0.5, Align::Right);
        label(
            cx,
            Rect::new(right + 6, y, 6, 1),
            "+3.6s",
            muted,
            0.46,
            Align::Left,
        );
        let ink = background.fg(state.color(cx));
        label(
            cx,
            Rect::new(right + 12, y, 2, 1),
            &glyph.to_string(),
            ink,
            0.5,
            Align::Left,
        );
        let kind_style = if state == State::Failed { ink } else { text };
        label(
            cx,
            Rect::new(right + 15, y, 18, 1),
            kind,
            kind_style,
            0.5,
            Align::Left,
        );
        label(
            cx,
            Rect::new(right + 36, y, 8, 1),
            "hooks",
            muted,
            0.5,
            Align::Left,
        );
    }
}

fn footer(cx: &mut RenderCx<'_>, ink: &Ink) {
    let Ink { muted, .. } = *ink;
    let mut x = 1;
    for (keys, action) in HINTS {
        let kbd = Kbd::new(keys);
        let width = kbd.measure(Size::new(COLUMNS - x, 1)).width;
        kbd.render(Rect::new(x, ROWS - 1, width, 1), cx);
        x += width + 1;
        let cells = u16::try_from(action.len()).unwrap_or(0);
        label(
            cx,
            Rect::new(x, ROWS - 1, cells, 1),
            action,
            muted,
            0.5,
            Align::Left,
        );
        x += cells + 1;
    }
}

trait Glyph {
    fn marker_glyph(self) -> String;
}

impl Glyph for State {
    fn marker_glyph(self) -> String {
        self.marker().glyph().to_string()
    }
}

fn rgb(color: Color) -> Result<[u8; 3], Failure> {
    match color {
        Color::Rgb(red, green, blue) => Ok([red, green, blue]),
        other => Err(format!("{other:?} is not an RGB color").into()),
    }
}

fn covers(area: Rect, column: u16, row: u16) -> bool {
    (area.x..area.x.saturating_add(area.width)).contains(&column)
        && (area.y..area.y.saturating_add(area.height)).contains(&row)
}

fn composite(grid: &Grid, clusters: &ClusterTable, host: &Host) -> Result<Vec<u8>, Failure> {
    let (width, height) = (u32::from(COLUMNS) * CELL.0, u32::from(ROWS) * CELL.1);
    let stride = usize::try_from(width)? * 4;
    let mut frame = vec![0_u8; stride * usize::try_from(height)?];
    let cell_bytes = usize::try_from(CELL.0)? * 4;
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let cell = grid.get(column, row).ok_or("cell outside the grid")?;
            let covered = host
                .images
                .iter()
                .any(|(area, _)| covers(*area, column, row));
            if !covered && cell.glyph().decode() != lunatui_core::DecodedGlyph::Inline(' ') {
                return Err(format!(
                    "cell text left outside an image at {column},{row}: {:?}",
                    grid.row_display(row, clusters)
                )
                .into());
            }
            let [red, green, blue] = rgb(cell.style().bg)?;
            for y in 0..usize::try_from(CELL.1)? {
                let top = usize::from(row) * usize::try_from(CELL.1)? + y;
                let start = top * stride + usize::from(column) * cell_bytes;
                for pixel in frame[start..start + cell_bytes].as_chunks_mut::<4>().0 {
                    *pixel = [red, green, blue, 255];
                }
            }
        }
    }
    for (area, image) in &host.images {
        let line_bytes = usize::try_from(image.size().width())? * 4;
        for (y, line) in image.rgba8_pixels().chunks_exact(line_bytes).enumerate() {
            let top = usize::from(area.y) * usize::try_from(CELL.1)? + y;
            let start = top * stride + usize::from(area.x) * cell_bytes;
            frame[start..start + line_bytes].copy_from_slice(line);
        }
    }
    Ok(frame)
}

fn graphite(out: &Path, theme: &Theme) -> Result<(), Failure> {
    for index in 0..FRAMES {
        let mut grid = Grid::new(COLUMNS, ROWS);
        let mut clusters = ClusterTable::new();
        let mut host = Host {
            background: Some(rgb(theme.resolve(Token::Surface0))?),
            images: Vec::new(),
        };
        {
            let mut cx = RenderCx::new(&mut grid, &mut clusters, theme)
                .image_sink(&mut host)
                .render_mode(RenderMode::Graphite);
            draw(&mut cx, index);
        }
        let frame = composite(&grid, &clusters, &host)?;
        let path = out.join(format!("graphite-{index:02}.png"));
        image::save_buffer(
            &path,
            &frame,
            u32::from(COLUMNS) * CELL.0,
            u32::from(ROWS) * CELL.1,
            image::ExtendedColorType::Rgba8,
        )?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn cells(out: &Path, theme: &Theme) -> Result<(), Failure> {
    let mut encoder = Encoder::new();
    let mut cast = format!(
        "{{\"version\": 2, \"width\": {COLUMNS}, \"height\": {ROWS}, \"timestamp\": 0, \"env\": {{\"TERM\": \"xterm-256color\"}}}}\n"
    );
    for index in 0..FRAMES {
        let mut grid = Grid::new(COLUMNS, ROWS);
        let mut clusters = ClusterTable::new();
        draw(&mut RenderCx::new(&mut grid, &mut clusters, theme), index);
        let bytes = encoder.encode_diff(
            &Grid::new(COLUMNS, ROWS),
            &grid,
            &clusters,
            ColorMode::TrueColor,
        );
        let text = format!(
            "\u{1b}[0m\u{1b}[2J\u{1b}[H{}",
            String::from_utf8_lossy(bytes)
        );
        let event = serde_json::json!([u32::try_from(index)?, "o", text]);
        cast.push_str(&event.to_string());
        cast.push('\n');
    }
    let path = out.join("cells.cast");
    fs::write(&path, cast)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> Result<(), Failure> {
    let out = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or("usage: activity_monitor OUTPUT_DIR")?,
    );
    fs::create_dir_all(&out)?;
    let theme = Theme::builtin(Variant::Dark);
    cells(&out, &theme)?;
    graphite(&out, &theme)
}
