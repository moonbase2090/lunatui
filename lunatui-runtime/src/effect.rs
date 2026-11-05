// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! Composable cell effects: fade, dissolve, sweep, slide and HSL shift.
//!
//! An [`Effect`] is a pure function of elapsed time. Render the widgets into a
//! [`Grid`] as usual, then call [`Effect::render`] with the time since the effect
//! started. The same elapsed time always produces the same cells, so frames are
//! reproducible and a golden diff is exact. Combinators run effects in
//! [`Effect::sequence`] or [`Effect::parallel`].
//!
//! Every leaf has a strength that runs from 0 (the grid untouched) to 1 (fully
//! applied) along its delay, duration and [`Ease`]. [`Effect::reversed`] flips the
//! strength, which turns a fade-out into a fade-in. Colors blend in `OKLab`. A
//! cell is rewritten only while the strength is above zero, so theme tokens and
//! the focus and disabled colors a widget chose survive until an effect needs
//! their RGB value.

use std::time::Duration;

use lunatui_core::{Attrs, Color, Grid, Rect, Style, oklab};

use crate::anim::{Ease, MotionPolicy};

/// Resolves the colors an effect needs into RGB.
///
/// Cells can hold a default color, a palette index or a theme token, none of which
/// carry RGB. `fg` and `bg` stand in for the default colors and `bg` is also the
/// backdrop that hidden cells collapse into. Build them from the active theme so
/// effects never hard-code colors.
#[derive(Clone, Copy)]
pub struct Palette<'a> {
    fg: [u8; 3],
    bg: [u8; 3],
    resolve: Option<&'a dyn Fn(Color) -> Option<[u8; 3]>>,
}

impl<'a> Palette<'a> {
    /// A palette with the given default foreground and background.
    #[must_use]
    pub const fn new(fg: [u8; 3], bg: [u8; 3]) -> Self {
        Self {
            fg,
            bg,
            resolve: None,
        }
    }

    /// Adds a resolver for palette indexes and theme tokens.
    #[must_use]
    pub const fn with_resolver(self, resolve: &'a dyn Fn(Color) -> Option<[u8; 3]>) -> Self {
        Self {
            resolve: Some(resolve),
            ..self
        }
    }

    fn rgb(&self, color: Color, foreground: bool) -> [u8; 3] {
        let fallback = if foreground { self.fg } else { self.bg };
        match color {
            Color::Rgb(red, green, blue) => [red, green, blue],
            Color::Default => fallback,
            other => self
                .resolve
                .and_then(|resolve| resolve(other))
                .unwrap_or(fallback),
        }
    }
}

/// Direction a sweep travels or a slide moves its content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Toward the left edge.
    Left,
    /// Toward the right edge.
    Right,
    /// Toward the top edge.
    Up,
    /// Toward the bottom edge.
    Down,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Fade(Color),
    Dissolve(u32),
    Sweep(Direction, f32),
    Slide(Direction),
    HslShift([f32; 3]),
    Group(Vec<Effect>),
}

/// One effect, or a group of effects, with its own timing.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    kind: Kind,
    delay: Duration,
    duration: Duration,
    ease: Ease,
    reversed: bool,
}

impl Effect {
    fn leaf(kind: Kind, duration: Duration, ease: Ease) -> Self {
        Self {
            kind,
            delay: Duration::ZERO,
            duration,
            ease,
            reversed: false,
        }
    }

    /// Blends foreground and background toward `color`; at full strength every cell is `color`.
    #[must_use]
    pub fn fade(color: Color, duration: Duration, ease: Ease) -> Self {
        Self::leaf(Kind::Fade(color), duration, ease)
    }

    /// Hides cells one by one in a scatter fixed by `seed`; at full strength all are hidden.
    #[must_use]
    pub fn dissolve(seed: u32, duration: Duration, ease: Ease) -> Self {
        Self::leaf(Kind::Dissolve(seed), duration, ease)
    }

    /// Hides cells with a wipe that moves toward `direction`, with a soft edge of
    /// `softness` (a fraction of the area, 0 for a hard edge).
    #[must_use]
    pub fn sweep(direction: Direction, softness: f32, duration: Duration, ease: Ease) -> Self {
        let softness = if softness.is_nan() {
            0.0
        } else {
            softness.clamp(0.0, 1.0)
        };
        Self::leaf(Kind::Sweep(direction, softness), duration, ease)
    }

    /// Moves the area's content toward `direction` until it has left the area.
    #[must_use]
    pub fn slide(direction: Direction, duration: Duration, ease: Ease) -> Self {
        Self::leaf(Kind::Slide(direction), duration, ease)
    }

    /// Shifts hue by `hue` degrees, and saturation and lightness by `saturation` and
    /// `lightness` (each -1 to 1), at full strength.
    #[must_use]
    pub fn hsl_shift(
        hue: f32,
        saturation: f32,
        lightness: f32,
        duration: Duration,
        ease: Ease,
    ) -> Self {
        Self::leaf(Kind::HslShift([hue, saturation, lightness]), duration, ease)
    }

    /// Runs `effects` one after another; each starts when the previous one ends.
    /// Finished effects keep their final strength.
    #[must_use]
    pub fn sequence(effects: Vec<Self>) -> Self {
        let mut offset = Duration::ZERO;
        let staged = effects
            .into_iter()
            .map(|effect| {
                let start = offset;
                offset += effect.total();
                effect.delayed(start)
            })
            .collect();
        Self::group(staged)
    }

    /// Runs `effects` at the same time.
    #[must_use]
    pub fn parallel(effects: Vec<Self>) -> Self {
        Self::group(effects)
    }

    fn group(children: Vec<Self>) -> Self {
        let duration = children
            .iter()
            .map(Self::total)
            .max()
            .unwrap_or(Duration::ZERO);
        Self {
            kind: Kind::Group(children),
            delay: Duration::ZERO,
            duration,
            ease: Ease::Linear,
            reversed: false,
        }
    }

    /// Waits `delay` more before starting.
    #[must_use]
    pub fn delayed(self, delay: Duration) -> Self {
        Self {
            delay: self.delay + delay,
            ..self
        }
    }

    /// Flips the strength so the effect runs backward: a fade-out becomes a fade-in.
    /// On a group this flips each member.
    #[must_use]
    pub fn reversed(self) -> Self {
        let kind = match self.kind {
            Kind::Group(children) => {
                Kind::Group(children.into_iter().map(Self::reversed).collect())
            }
            other => other,
        };
        Self {
            kind,
            reversed: !self.reversed,
            ..self
        }
    }

    /// Collapses every duration and delay when `policy` asks for reduced motion, so
    /// the effect jumps to its end state.
    #[must_use]
    pub fn with_policy(self, policy: MotionPolicy) -> Self {
        if !policy.is_reduced() {
            return self;
        }
        let kind = match self.kind {
            Kind::Group(children) => Kind::Group(
                children
                    .into_iter()
                    .map(|child| child.with_policy(policy))
                    .collect(),
            ),
            other => other,
        };
        Self {
            kind,
            delay: Duration::ZERO,
            duration: Duration::ZERO,
            ..self
        }
    }

    /// Time from the start until the effect is finished.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.delay + self.duration
    }

    /// Whether `elapsed` is past the end of the effect.
    #[must_use]
    pub fn is_done(&self, elapsed: Duration) -> bool {
        elapsed >= self.total()
    }

    /// The strength, 0 to 1, of a leaf at `elapsed`, after easing and reversal.
    #[must_use]
    pub fn strength(&self, elapsed: Duration) -> f32 {
        let local = elapsed.saturating_sub(self.delay);
        let progress = if self.duration.is_zero() {
            if elapsed >= self.delay { 1.0 } else { 0.0 }
        } else {
            (local.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
        };
        let eased = self.ease.apply(progress);
        if self.reversed { 1.0 - eased } else { eased }
    }

    /// Applies the effect at `elapsed` to the cells of `grid` inside `area`.
    ///
    /// The grid must hold the frame as the widgets drew it. Rendering the same frame
    /// at the same time is deterministic.
    pub fn render(&self, elapsed: Duration, grid: &mut Grid, area: Rect, palette: &Palette<'_>) {
        let area = clip(area, grid);
        if area.width == 0 || area.height == 0 {
            return;
        }
        if let Kind::Group(children) = &self.kind {
            let local = elapsed.saturating_sub(self.delay);
            for child in children {
                child.render(local, grid, area, palette);
            }
            return;
        }
        let strength = self.strength(elapsed);
        if strength <= 0.0 {
            return;
        }
        match &self.kind {
            Kind::Slide(direction) => slide(grid, area, *direction, strength, palette),
            Kind::Group(_) => {}
            kind => restyle(grid, area, kind, strength, palette),
        }
    }
}

fn clip(area: Rect, grid: &Grid) -> Rect {
    let width = grid.width().saturating_sub(area.x).min(area.width);
    let height = grid.height().saturating_sub(area.y).min(area.height);
    Rect::new(area.x, area.y, width, height)
}

/// Remembers the last few style conversions of one render call. Neighbouring
/// cells mostly share a style, so the color math runs once per distinct style.
struct Memo {
    entries: [(Style, u32, Style); 16],
    filled: usize,
    next: usize,
}

impl Memo {
    fn new() -> Self {
        Self {
            entries: [(Style::new(), 0, Style::new()); 16],
            filled: 0,
            next: 0,
        }
    }

    fn get(&mut self, style: Style, amount: f32, compute: impl FnOnce() -> Style) -> Style {
        let key = amount.to_bits();
        if let Some(&(_, _, found)) = self.entries[..self.filled]
            .iter()
            .find(|(from, bits, _)| *from == style && *bits == key)
        {
            return found;
        }
        let made = compute();
        self.entries[self.next] = (style, key, made);
        self.next = (self.next + 1) % self.entries.len();
        self.filled = (self.filled + 1).min(self.entries.len());
        made
    }
}

fn restyle(grid: &mut Grid, area: Rect, kind: &Kind, strength: f32, palette: &Palette<'_>) {
    let mut memo = Memo::new();
    for row in 0..area.height {
        let y = area.y + row;
        // Cells that end up with the same style are written as one run.
        let mut run: Option<(u16, u16, Style)> = None;
        for column in 0..area.width {
            let x = area.x + column;
            let styled = grid.get(x, y).and_then(|cell| {
                let style = cell.style();
                let styled = match kind {
                    Kind::Fade(color) => {
                        memo.get(style, strength, || fade(style, *color, strength, palette))
                    }
                    Kind::Dissolve(seed) => {
                        if scatter(x, y, *seed) < strength {
                            memo.get(style, 1.0, || hide(style, 1.0, palette))
                        } else {
                            return None;
                        }
                    }
                    Kind::Sweep(direction, softness) => {
                        let along = position(*direction, column, row, area);
                        let hidden = sweep_cover(along, strength, *softness);
                        if hidden <= 0.0 {
                            return None;
                        }
                        memo.get(style, hidden, || hide(style, hidden, palette))
                    }
                    Kind::HslShift(shift) => memo.get(style, strength, || {
                        hsl_shift(style, *shift, strength, palette)
                    }),
                    Kind::Slide(_) | Kind::Group(_) => return None,
                };
                (styled != style).then_some(styled)
            });
            run = match (run, styled) {
                (Some((start, len, held)), Some(next)) if held == next => {
                    Some((start, len + 1, held))
                }
                (open, next) => {
                    if let Some((start, len, held)) = open {
                        grid.set_style(Rect::new(start, y, len, 1), held);
                    }
                    next.map(|style| (x, 1, style))
                }
            };
        }
        if let Some((start, len, held)) = run {
            grid.set_style(Rect::new(start, y, len, 1), held);
        }
    }
}

fn mix(from: [u8; 3], to: [u8; 3], amount: f32) -> Color {
    let rgb = oklab::lerp_rgb(from, to, amount);
    Color::Rgb(rgb[0], rgb[1], rgb[2])
}

fn fade(style: Style, target: Color, strength: f32, palette: &Palette<'_>) -> Style {
    let fg = palette.rgb(style.fg, true);
    let bg = palette.rgb(style.bg, false);
    Style {
        fg: mix(fg, palette.rgb(target, true), strength),
        bg: mix(bg, palette.rgb(target, false), strength),
        attrs: style.attrs,
    }
}

fn hide(style: Style, amount: f32, palette: &Palette<'_>) -> Style {
    let surface = oklab::lerp_rgb(palette.rgb(style.bg, false), palette.bg, amount);
    let ink = oklab::lerp_rgb(palette.rgb(style.fg, true), surface, amount);
    Style {
        fg: Color::Rgb(ink[0], ink[1], ink[2]),
        bg: Color::Rgb(surface[0], surface[1], surface[2]),
        attrs: if amount >= 1.0 {
            Attrs::default()
        } else {
            style.attrs
        },
    }
}

fn scatter(x: u16, y: u16, seed: u32) -> f32 {
    let mut state =
        seed ^ u32::from(x).wrapping_mul(0x9E37_79B1) ^ u32::from(y).wrapping_mul(0x85EB_CA77);
    state ^= state >> 15;
    state = state.wrapping_mul(0x2C1B_3C6D);
    state ^= state >> 12;
    state = state.wrapping_mul(0x297A_2D39);
    state ^= state >> 15;
    f32::from(u16::try_from(state >> 16).unwrap_or(u16::MAX)) / 65_536.0
}

/// Position of a cell along the sweep, 0 where the sweep starts and 1 where it ends.
fn position(direction: Direction, column: u16, row: u16, area: Rect) -> f32 {
    let (index, span) = match direction {
        Direction::Left => (area.width - 1 - column, area.width),
        Direction::Right => (column, area.width),
        Direction::Up => (area.height - 1 - row, area.height),
        Direction::Down => (row, area.height),
    };
    if span <= 1 {
        0.0
    } else {
        f32::from(index) / f32::from(span - 1)
    }
}

/// How hidden a cell at `along` is when the sweep front is at `strength`.
fn sweep_cover(along: f32, strength: f32, softness: f32) -> f32 {
    if strength >= 1.0 {
        return 1.0;
    }
    if softness <= 0.0 {
        return if along < strength { 1.0 } else { 0.0 };
    }
    ((strength * (1.0 + softness) - along) / softness).clamp(0.0, 1.0)
}

fn slide(grid: &mut Grid, area: Rect, direction: Direction, strength: f32, palette: &Palette<'_>) {
    let horizontal = matches!(direction, Direction::Left | Direction::Right);
    let span = if horizontal { area.width } else { area.height };
    let moved = round_to_cells(f32::from(span) * strength).min(span);
    if moved == 0 {
        return;
    }
    let mut snapshot = Grid::new(area.width, area.height);
    snapshot.blit(grid, area, 0, 0);
    let kept = span - moved;
    let (from, to, gap) = match direction {
        Direction::Left => (
            Rect::new(moved, 0, kept, area.height),
            (area.x, area.y),
            Rect::new(area.x + kept, area.y, moved, area.height),
        ),
        Direction::Right => (
            Rect::new(0, 0, kept, area.height),
            (area.x + moved, area.y),
            Rect::new(area.x, area.y, moved, area.height),
        ),
        Direction::Up => (
            Rect::new(0, moved, area.width, kept),
            (area.x, area.y),
            Rect::new(area.x, area.y + kept, area.width, moved),
        ),
        Direction::Down => (
            Rect::new(0, 0, area.width, kept),
            (area.x, area.y + moved),
            Rect::new(area.x, area.y, area.width, moved),
        ),
    };
    if kept > 0 {
        grid.blit(&snapshot, from, to.0, to.1);
    }
    let backdrop = Color::Rgb(palette.bg[0], palette.bg[1], palette.bg[2]);
    let blank = Grid::new(gap.width, gap.height);
    grid.blit(&blank, Rect::new(0, 0, gap.width, gap.height), gap.x, gap.y);
    grid.set_style(gap, Style::new().fg(backdrop).bg(backdrop));
}

fn round_to_cells(value: f32) -> u16 {
    let mut cells = 0_u16;
    while f32::from(cells) + 0.5 <= value && cells < u16::MAX {
        cells += 1;
    }
    cells
}

fn hsl_shift(style: Style, shift: [f32; 3], strength: f32, palette: &Palette<'_>) -> Style {
    let apply = |rgb: [u8; 3]| {
        let [hue, saturation, lightness] = to_hsl(rgb);
        let hue = (hue + shift[0] * strength).rem_euclid(360.0);
        let saturation = (saturation + shift[1] * strength).clamp(0.0, 1.0);
        let lightness = (lightness + shift[2] * strength).clamp(0.0, 1.0);
        let [red, green, blue] = from_hsl([hue, saturation, lightness]);
        Color::Rgb(red, green, blue)
    };
    Style {
        fg: apply(palette.rgb(style.fg, true)),
        bg: apply(palette.rgb(style.bg, false)),
        attrs: style.attrs,
    }
}

fn to_hsl(rgb: [u8; 3]) -> [f32; 3] {
    let [red, green, blue] = rgb.map(|channel| f32::from(channel) / 255.0);
    let high = red.max(green).max(blue);
    let low = red.min(green).min(blue);
    let lightness = f32::midpoint(high, low);
    let spread = high - low;
    if spread <= f32::EPSILON {
        return [0.0, 0.0, lightness];
    }
    let saturation = spread / (1.0 - (2.0 * lightness - 1.0).abs());
    let sector = if (high - red).abs() <= f32::EPSILON {
        ((green - blue) / spread).rem_euclid(6.0)
    } else if (high - green).abs() <= f32::EPSILON {
        (blue - red) / spread + 2.0
    } else {
        (red - green) / spread + 4.0
    };
    [sector * 60.0, saturation, lightness]
}

fn from_hsl(hsl: [f32; 3]) -> [u8; 3] {
    let [hue, saturation, lightness] = hsl;
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match sector {
        s if s < 1.0 => (chroma, second, 0.0),
        s if s < 2.0 => (second, chroma, 0.0),
        s if s < 3.0 => (0.0, chroma, second),
        s if s < 4.0 => (0.0, second, chroma),
        s if s < 5.0 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let offset = lightness - chroma / 2.0;
    [red, green, blue].map(|channel| unit_to_byte(channel + offset))
}

fn unit_to_byte(unit: f32) -> u8 {
    let scaled = unit.clamp(0.0, 1.0) * 255.0;
    let mut byte = 0_u8;
    while byte < 255 && f32::from(byte) + 0.5 <= scaled {
        byte += 1;
    }
    byte
}
