// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

use graphite_core::{DARK, DEFAULT_ACCENT, PixelBuffer, Rect, Rgba, accent};
use lunatui_core::Rect as CellRect;
use lunatui_term::{PixelImage, PixelLayer, PixelMode};

use crate::{CountingWriter, WriteStats};

/// Width of one cell in pixels at 1x.
pub const CELL_WIDTH: usize = 8;
/// Height of one cell in pixels at 1x.
pub const CELL_HEIGHT: usize = 16;

const WINDOW_IMAGE: u32 = 1;
const CHANGED_IMAGE: u32 = 2;

/// What a frame repaints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repaint {
    /// The whole window, every frame.
    Full,
    /// Only the changed rectangle; the window image is reused.
    Changed,
}

/// One Graphite case: a repaint at a scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphiteCase {
    /// Name used by the instruction worker and reports.
    pub name: &'static str,
    /// What each frame repaints.
    pub repaint: Repaint,
    /// Device pixels per logical pixel: 1 or 2.
    pub scale: usize,
}

/// Every Graphite case, in report order.
pub const GRAPHITE_CASES: [GraphiteCase; 4] = [
    GraphiteCase {
        name: "graphite-changed-1x",
        repaint: Repaint::Changed,
        scale: 1,
    },
    GraphiteCase {
        name: "graphite-changed-2x",
        repaint: Repaint::Changed,
        scale: 2,
    },
    GraphiteCase {
        name: "graphite-full-1x",
        repaint: Repaint::Full,
        scale: 1,
    },
    GraphiteCase {
        name: "graphite-full-2x",
        repaint: Repaint::Full,
        scale: 2,
    },
];

/// Looks up a case by name.
#[must_use]
pub fn graphite_case(name: &str) -> Option<&'static GraphiteCase> {
    GRAPHITE_CASES.iter().find(|case| case.name == name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    cols: usize,
    rows: usize,
    rows_per_seat: usize,
    changed_cols: usize,
}

const SEATS: usize = 3;
const LANE_START: usize = 12;

impl Layout {
    fn new(cols: u16, rows: u16) -> Result<Self, String> {
        let (cols, rows) = (usize::from(cols), usize::from(rows));
        if cols < 2 * LANE_START || rows < 2 + SEATS + 2 {
            return Err(format!("{cols}x{rows} cells is too small for the scene"));
        }
        Ok(Self {
            cols,
            rows,
            rows_per_seat: (rows / 12).max(1),
            changed_cols: (cols * 13 / 25).max(LANE_START + 2),
        })
    }

    fn seat_rows(self) -> usize {
        SEATS * self.rows_per_seat
    }

    fn changed(self) -> CellRect {
        cell_rect(0, 1, self.changed_cols, self.seat_rows())
    }
}

fn cell_rect(x: usize, y: usize, width: usize, height: usize) -> CellRect {
    let narrow = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
    CellRect::new(narrow(x), narrow(y), narrow(width), narrow(height))
}

/// The cell rectangle that changes between ticks for a `cols`×`rows` window.
///
/// # Errors
///
/// Returns an error when the window is too small for the scene.
pub fn changed_cells(cols: u16, rows: u16) -> Result<CellRect, String> {
    Layout::new(cols, rows).map(Layout::changed)
}

struct Canvas<'a> {
    buffer: &'a mut PixelBuffer,
    origin: (usize, usize),
    scale: usize,
}

impl Canvas<'_> {
    fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, color: Rgba) {
        let (x, y, w, h) = (
            x * self.scale,
            y * self.scale,
            w * self.scale,
            h * self.scale,
        );
        let (ox, oy) = self.origin;
        let left = x.max(ox);
        let top = y.max(oy);
        let right = (x + w).min(ox + self.buffer.width());
        let bottom = (y + h).min(oy + self.buffer.height());
        if left < right && top < bottom {
            self.buffer.fill_rect(
                Rect::new(left - ox, top - oy, right - left, bottom - top),
                color,
            );
        }
    }

    fn stroke(&mut self, x: usize, y: usize, w: usize, h: usize, color: Rgba) {
        self.fill(x, y, w, 1, color);
        self.fill(x, y + h - 1, w, 1, color);
        self.fill(x, y, 1, h, color);
        self.fill(x + w - 1, y, 1, h, color);
    }
}

fn solid(color: [u8; 3]) -> Rgba {
    Rgba::new(color[0], color[1], color[2], 255)
}

fn tinted(color: [u8; 3], alpha: u8) -> Rgba {
    Rgba::new(color[0], color[1], color[2], alpha)
}

fn bin_height(seat: usize, bin: usize, tick: u32, max: usize) -> usize {
    let mut x = (seat as u64) << 32 ^ (bin as u64).wrapping_add(u64::from(tick));
    x = x.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^= x >> 29;
    usize::try_from(x % (max as u64 + 1)).unwrap_or(0)
}

fn draw_static(canvas: &mut Canvas<'_>, layout: Layout) {
    let t = &DARK;
    let (cw, ch) = (CELL_WIDTH, CELL_HEIGHT);
    let width = layout.cols * cw;
    canvas.fill(0, 0, width, ch, solid(t.bar));
    canvas.fill(0, ch - 1, width, 1, solid(t.bar_line));
    for chip in 0..4 {
        let x = cw + chip * 12 * cw;
        canvas.fill(x, 3, 10 * cw, ch - 6, solid(t.chip_active));
        canvas.fill(x + cw, 7, 6 * cw, 2, solid(t.text));
    }
    let top = ch * (1 + layout.seat_rows());
    let summary_x = layout.changed_cols * cw;
    canvas.fill(summary_x, ch, width - summary_x, top - ch, solid(t.panel));
    canvas.stroke(
        summary_x,
        ch,
        width - summary_x,
        top - ch,
        solid(t.hairline),
    );
    let timeline_rows = layout.rows - 2 - layout.seat_rows();
    for row in 0..timeline_rows {
        let y = top + row * ch;
        if row % 2 == 1 {
            canvas.fill(0, y, width, ch, solid(t.panel));
        }
        canvas.fill(cw, y + 6, (8 + row % 5 * 4) * cw, 3, solid(t.muted));
        canvas.fill(width / 3, y + 6, (10 + row % 7 * 3) * cw, 3, solid(t.text));
        canvas.fill(0, y + ch - 1, width, 1, solid(t.title_line));
    }
    let key_y = (layout.rows - 1) * ch;
    canvas.fill(0, key_y, width, ch, solid(t.status_bar));
    canvas.fill(0, key_y, width, 1, solid(t.status_line));
    for key in 0..6 {
        let x = cw + key * 9 * cw;
        canvas.fill(x, key_y + 3, 3 * cw, ch - 6, solid(t.key));
        canvas.stroke(x, key_y + 3, 3 * cw, ch - 6, solid(t.key_line));
        canvas.fill(x + 4 * cw, key_y + 7, 4 * cw, 2, solid(t.muted));
    }
}

fn draw_seats(canvas: &mut Canvas<'_>, layout: Layout, tick: u32) {
    let t = &DARK;
    let focus = accent(t, DEFAULT_ACCENT);
    let (cw, ch) = (CELL_WIDTH, CELL_HEIGHT);
    let width = layout.changed_cols * cw;
    let seat_h = layout.rows_per_seat * ch;
    let status = [t.working, t.unseen, t.attention, t.idle];
    for seat in 0..SEATS {
        let y = ch + seat * seat_h;
        let selected = (tick as usize / 8) % SEATS == seat;
        canvas.fill(
            0,
            y,
            width,
            seat_h,
            solid(if selected { t.tab_active } else { t.panel }),
        );
        if selected {
            canvas.fill(0, y, 2, seat_h, solid(focus));
        }
        let state = status[(seat + tick as usize / 4) % status.len()];
        canvas.fill(cw, y + seat_h / 2 - 3, 6, 6, solid(state));
        canvas.fill(2 * cw, y + seat_h / 2 - 2, 7 * cw, 4, solid(t.text));
        let lane_x = LANE_START * cw;
        let lane_w = width - lane_x - cw;
        canvas.fill(lane_x, y + 2, lane_w, seat_h - 4, solid(t.field));
        canvas.stroke(lane_x, y + 2, lane_w, seat_h - 4, solid(t.field_line));
        let max = seat_h - 8;
        for bin in 0..lane_w / 4 {
            let h = bin_height(seat, bin, tick, max);
            canvas.fill(
                lane_x + bin * 4 + 1,
                y + seat_h - 4 - h,
                3,
                h,
                tinted(state, 200),
            );
        }
        canvas.fill(0, y + seat_h - 1, width, 1, solid(t.hairline));
    }
}

fn buffer(width: usize, height: usize) -> Result<PixelBuffer, String> {
    PixelBuffer::new(width, height, solid(DARK.ground)).map_err(|error| format!("{error:?}"))
}

/// Rasterizes the whole window at `tick`.
///
/// # Errors
///
/// Returns an error when the window is too small or the buffer cannot be allocated.
pub fn render_full(cols: u16, rows: u16, scale: usize, tick: u32) -> Result<PixelBuffer, String> {
    let layout = Layout::new(cols, rows)?;
    let mut pixels = buffer(
        layout.cols * CELL_WIDTH * scale,
        layout.rows * CELL_HEIGHT * scale,
    )?;
    repaint_full(&mut pixels, layout, scale, tick);
    Ok(pixels)
}

fn repaint_full(pixels: &mut PixelBuffer, layout: Layout, scale: usize, tick: u32) {
    pixels.clear(solid(DARK.ground));
    let mut canvas = Canvas {
        buffer: pixels,
        origin: (0, 0),
        scale,
    };
    draw_static(&mut canvas, layout);
    draw_seats(&mut canvas, layout, tick);
}

/// Rasterizes only the changed rectangle at `tick`, as its own buffer.
///
/// # Errors
///
/// Returns an error when the window is too small or the buffer cannot be allocated.
pub fn render_changed(
    cols: u16,
    rows: u16,
    scale: usize,
    tick: u32,
) -> Result<PixelBuffer, String> {
    let layout = Layout::new(cols, rows)?;
    let changed = layout.changed();
    let mut pixels = buffer(
        usize::from(changed.width) * CELL_WIDTH * scale,
        usize::from(changed.height) * CELL_HEIGHT * scale,
    )?;
    repaint_changed(&mut pixels, layout, scale, tick);
    Ok(pixels)
}

fn repaint_changed(pixels: &mut PixelBuffer, layout: Layout, scale: usize, tick: u32) {
    let changed = layout.changed();
    pixels.clear(solid(DARK.ground));
    let mut canvas = Canvas {
        buffer: pixels,
        origin: (
            usize::from(changed.x) * CELL_WIDTH * scale,
            usize::from(changed.y) * CELL_HEIGHT * scale,
        ),
        scale,
    };
    draw_seats(&mut canvas, layout, tick);
}

fn image(pixels: &PixelBuffer) -> io::Result<PixelImage<'_>> {
    let size = |value: usize| u32::try_from(value).map_err(io::Error::other);
    PixelImage::new(
        size(pixels.width())?,
        size(pixels.height())?,
        pixels.as_rgba(),
    )
    .ok_or_else(|| io::Error::other("the pixel buffer is not a valid image"))
}

struct Frames {
    case: &'static GraphiteCase,
    layout: Layout,
    layer: PixelLayer,
    window: PixelBuffer,
    changed: PixelBuffer,
    tick: u32,
}

impl Frames {
    fn frame_into<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let tick = self.tick;
        let generation = u64::from(tick) + 2;
        let window = cell_rect(0, 0, self.layout.cols, self.layout.rows);
        self.layer.begin_frame();
        match self.case.repaint {
            Repaint::Full => {
                repaint_full(&mut self.window, self.layout, self.case.scale, tick);
                self.layer
                    .image(WINDOW_IMAGE, generation, &image(&self.window)?, window, 0);
            }
            Repaint::Changed => {
                repaint_changed(&mut self.changed, self.layout, self.case.scale, tick);
                self.layer
                    .image(WINDOW_IMAGE, 1, &image(&self.window)?, window, 0);
                self.layer.image(
                    CHANGED_IMAGE,
                    generation,
                    &image(&self.changed)?,
                    self.layout.changed(),
                    1,
                );
            }
        }
        out.write_all(self.layer.finish_frame())?;
        self.tick = tick.wrapping_add(1);
        Ok(())
    }
}

/// One Graphite case at one window size, reusing its buffers across frames.
pub struct GraphiteRun {
    frames: Frames,
    sink: CountingWriter<io::Sink>,
}

impl GraphiteRun {
    /// Sets up `case` for a window of `cols`×`rows` cells.
    ///
    /// # Errors
    ///
    /// Returns an error when the window is too small or a buffer cannot be allocated.
    pub fn new(case: &'static GraphiteCase, cols: u16, rows: u16) -> Result<Self, String> {
        Ok(Self {
            frames: Frames {
                case,
                layout: Layout::new(cols, rows)?,
                layer: PixelLayer::new(PixelMode::Kitty),
                window: render_full(cols, rows, case.scale, 0)?,
                changed: render_changed(cols, rows, case.scale, 0)?,
                tick: 0,
            },
            sink: CountingWriter::new(io::sink()),
        })
    }

    /// Frames drawn so far; frame `n` draws the scene at tick `n`.
    #[must_use]
    pub fn tick(&self) -> u32 {
        self.frames.tick
    }

    /// Bytes and `write()` calls sent to the counting sink so far.
    #[must_use]
    pub fn stats(&self) -> WriteStats {
        self.sink.stats()
    }

    /// Rasterizes the next tick, encodes what changed as terminal images and writes it to the
    /// counting sink.
    ///
    /// # Errors
    ///
    /// Returns an error if the image is invalid or writing fails.
    pub fn frame(&mut self) -> io::Result<()> {
        self.frames.frame_into(&mut self.sink)
    }

    /// Like [`frame`](Self::frame), but writes the frame's bytes to `out` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the image is invalid or writing fails.
    pub fn frame_into<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        self.frames.frame_into(out)
    }
}
