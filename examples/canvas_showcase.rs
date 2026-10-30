// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::cell::RefCell;
use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;
use std::sync::OnceLock;

use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::canvas::{Circle, Context, Line, Points};
use lunatui_widgets::{Block, BorderType, Canvas, RenderCx, Tabs, Widget};

pub(super) const FPS: u32 = 60;
const BOID_RATE: f64 = 30.0;
pub(super) const SCENES: [(&str, &str, u32); 6] = [
    ("Scope", " XY oscilloscope ", 51),
    ("Globe", " Flight paths ", 57),
    ("System", " System load ", 51),
    ("Clock", " Clock and tree ", 69),
    ("Flock", " Flocking ", 42),
    ("Pings", " Network pings ", 51),
];

pub(super) const LOOP_TICKS: u32 = {
    let mut total = 0;
    let mut index = 0;
    while index < SCENES.len() {
        total += SCENES[index].2;
        index += 1;
    }
    total
};

fn scene_at(tick: u32) -> (usize, u32) {
    let mut local = tick % LOOP_TICKS;
    for (index, &(_, _, ticks)) in SCENES.iter().enumerate() {
        if local < ticks {
            return (index, local);
        }
        local -= ticks;
    }
    (0, 0)
}

const CELL_ASPECT: f64 = 0.6;
const STEPS: u8 = 8;
const COASTLINE: &str = include_str!("canvas_showcase/coastline.txt");

#[derive(Clone, Copy)]
struct Ink {
    bg: Color,
    text: Color,
    line: Color,
    warm: Color,
    rose: Color,
    leaf: Color,
}

impl Ink {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            bg: cx.token(Token::Surface0),
            text: cx.token(Token::TextPrimary),
            line: cx.token(Token::FocusRing),
            warm: cx.token(Token::Warn),
            rose: cx.token(Token::Fail),
            leaf: cx.token(Token::Ok),
        }
    }

    fn shade(self, color: Color, level: f64) -> Style {
        Style::new().fg(mix(self.bg, color, level))
    }

    fn panel(self) -> Style {
        Style::new().fg(self.text).bg(self.bg)
    }
}

fn step(level: f64) -> u8 {
    let scaled = level.clamp(0.0, 1.0) * f64::from(STEPS);
    (0..=STEPS)
        .rev()
        .find(|&k| f64::from(k) <= scaled + 0.5)
        .unwrap_or(0)
}

fn mix(from: Color, to: Color, level: f64) -> Color {
    let (Color::Rgb(r0, g0, b0), Color::Rgb(r1, g1, b1)) = (from, to) else {
        return to;
    };
    let k = i32::from(step(level));
    let total = i32::from(STEPS);
    let channel = |a: u8, b: u8| {
        let (a, b) = (i32::from(a), i32::from(b));
        u8::try_from((a + (b - a) * k / total).clamp(0, 255)).unwrap_or(0)
    };
    Color::Rgb(channel(r0, r1), channel(g0, g1), channel(b0, b1))
}

fn hash(n: u32) -> f64 {
    let mut x = u64::from(n).wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    let top = u32::try_from(x >> 40).unwrap_or(0);
    f64::from(top) / f64::from(1_u32 << 24)
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn noise(n: u32, period: u32, seed: u32) -> f64 {
    let knot = n / period;
    let within = f64::from(n % period) / f64::from(period);
    let at = |k: u32| {
        hash(
            k.wrapping_mul(7919)
                .wrapping_add(seed.wrapping_mul(104_729)),
        )
    };
    let (a, b) = (at(knot), at(knot + 1));
    a + (b - a) * smooth(within)
}

fn stroke(ctx: &mut Context<'_>, points: &[(f64, f64)], style: Style) {
    for pair in points.windows(2) {
        ctx.draw(&Line {
            x1: pair[0].0,
            y1: pair[0].1,
            x2: pair[1].0,
            y2: pair[1].1,
            style,
        });
    }
}

fn segment(ctx: &mut Context<'_>, from: (f64, f64), to: (f64, f64), style: Style) {
    ctx.draw(&Line {
        x1: from.0,
        y1: from.1,
        x2: to.0,
        y2: to.1,
        style,
    });
}

#[derive(Clone, Copy)]
struct Stage {
    ink: Ink,
    hw: f64,
    hh: f64,
    tick: u32,
    t: f64,
    pixels: bool,
}

impl Stage {
    fn new(ink: Ink, area: Rect, tick: u32, pixels: bool) -> Self {
        Self {
            ink,
            hw: f64::from(area.width) * CELL_ASPECT / 2.0,
            hh: f64::from(area.height) / 2.0,
            tick,
            t: f64::from(tick) / f64::from(FPS),
            pixels,
        }
    }

    fn canvas<F: Fn(&mut Context<'_>)>(self, area: Rect, cx: &mut RenderCx<'_>, paint: F) {
        Canvas::new(paint)
            .pixels(self.pixels)
            .x_bounds([-self.hw, self.hw])
            .y_bounds([-self.hh, self.hh])
            .background(self.ink.panel())
            .render(area, cx);
    }
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let mut cx = RenderCx::new(grid, clusters, theme);
    let (index, local) = scene_at(tick);
    render_scene(&mut cx, area, index, local, false);
}

pub(super) fn render_scene(
    cx: &mut RenderCx<'_>,
    area: Rect,
    index: usize,
    tick: u32,
    pixels: bool,
) {
    let ink = Ink::new(cx);
    Block::default().style(ink.panel()).render(area, cx);
    if area.width < 4 || area.height < 4 {
        return;
    }
    let index = index % SCENES.len();
    Tabs::new(SCENES.iter().map(|(name, _, _)| format!(" {name} ")))
        .select(index)
        .style(ink.shade(ink.text, 0.55).bg(ink.bg))
        .highlight_style(Style::new().fg(ink.bg).bg(ink.line))
        .indent(1)
        .render(Rect::new(area.x, area.y, area.width, 1), cx);
    let frame = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(SCENES[index].1)
        .style(ink.panel())
        .border_style(ink.shade(ink.text, 0.22).bg(ink.bg))
        .title_style(ink.shade(ink.text, 0.8).bg(ink.bg));
    block.render(frame, cx);
    let inner = block.inner(frame);
    let stage = Stage::new(ink, inner, tick, pixels);
    match index {
        0 => scope(stage, inner, cx),
        1 => globe(stage, inner, cx),
        2 => system(stage, inner, cx),
        3 => clock_and_tree(stage, inner, cx),
        4 => flock(stage, inner, cx),
        _ => pings(stage, inner, cx),
    }
}

fn scope_ratio(t: f64) -> (f64, f64) {
    const KEYS: [(f64, f64, f64); 6] = [
        (0.0, 3.0, 2.0),
        (0.06, 3.0, 2.0),
        (0.27, 5.0, 4.0),
        (0.34, 5.0, 4.0),
        (0.56, 3.0, 4.0),
        (0.62, 3.0, 4.0),
    ];
    const END: (f64, f64, f64) = (0.85, 1.0, 2.0);
    let mut previous = KEYS[0];
    for &key in KEYS.iter().skip(1).chain(core::iter::once(&END)) {
        if t < key.0 {
            let k = smooth((t - previous.0) / (key.0 - previous.0));
            return (
                previous.1 + (key.1 - previous.1) * k,
                previous.2 + (key.2 - previous.2) * k,
            );
        }
        previous = key;
    }
    (END.1, END.2)
}

fn scope(stage: Stage, area: Rect, cx: &mut RenderCx<'_>) {
    let Stage { ink, hh, t, .. } = stage;
    let div = hh * 1.8 / 8.0;
    let (a, b) = scope_ratio(t);
    let phase = PI / 4.0 + 0.55 * t;
    let head = t * TAU * 0.7;
    let (ax, ay) = (div * 4.2, div * 3.3);
    stage.canvas(area, cx, move |ctx| {
        let grid = ink.shade(ink.text, 0.12);
        for i in -5_i32..=5 {
            let x = f64::from(i) * div;
            let style = if i == 0 {
                ink.shade(ink.text, 0.2)
            } else {
                grid
            };
            segment(ctx, (x, -4.0 * div), (x, 4.0 * div), style);
        }
        for j in -4_i32..=4 {
            let y = f64::from(j) * div;
            let style = if j == 0 {
                ink.shade(ink.text, 0.2)
            } else {
                grid
            };
            segment(ctx, (-5.0 * div, y), (5.0 * div, y), style);
        }
        ctx.layer();
        let samples = 520_u32;
        let span = TAU * 0.45;
        let beam = |u: f64| {
            let s = head - u;
            (ax * (a * s + phase).sin(), ay * (b * s).sin())
        };
        let mut older = beam(span);
        for k in (0..samples).rev() {
            let u = span * f64::from(k) / f64::from(samples);
            let point = beam(u);
            let fresh = 1.0 - u / span;
            segment(
                ctx,
                older,
                point,
                ink.shade(ink.line, 0.22 + 0.78 * ((fresh * 4.0).ceil() / 4.0).powi(2)),
            );
            older = point;
        }
        ctx.draw(&Points {
            coords: &[beam(0.0)],
            style: Style::new().fg(ink.text),
        });
        let label = ink.shade(ink.text, 0.6);
        ctx.print(
            -5.0 * div,
            4.0 * div + 0.9,
            format!("X  sin({a:.2} t + φ)"),
            label,
        );
        ctx.print(
            -1.4 * div,
            4.0 * div + 0.9,
            format!("Y  sin({b:.2} t)"),
            label,
        );
        ctx.print(3.0 * div, 4.0 * div + 0.9, "1 V/div  2 ms/div", label);
        ctx.print(
            -5.0 * div,
            -4.0 * div - 0.6,
            format!("ratio {a:.2} : {b:.2}"),
            ink.shade(ink.line, 0.9),
        );
    });
}

fn sphere(lat: f64, lon: f64) -> [f64; 3] {
    let (lat, lon) = (lat.to_radians(), lon.to_radians());
    [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
}

#[derive(Clone, Copy)]
struct View {
    spin: (f64, f64),
    tilt: (f64, f64),
}

impl View {
    fn new(lon: f64, lat: f64) -> Self {
        Self {
            spin: lon.to_radians().sin_cos(),
            tilt: lat.to_radians().sin_cos(),
        }
    }

    fn turn(self, [east, north, up]: [f64; 3]) -> [f64; 3] {
        let (spin_sin, spin_cos) = self.spin;
        let front = east * spin_cos + north * spin_sin;
        let side = north * spin_cos - east * spin_sin;
        let (tilt_sin, tilt_cos) = self.tilt;
        [
            side,
            tilt_cos * up - tilt_sin * front,
            tilt_sin * up + tilt_cos * front,
        ]
    }
}

fn slerp(a: [f64; 3], b: [f64; 3], s: f64) -> [f64; 3] {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let angle = dot.acos();
    if angle < 1e-6 {
        return a;
    }
    let (wa, wb) = (
        ((1.0 - s) * angle).sin() / angle.sin(),
        (s * angle).sin() / angle.sin(),
    );
    [
        wa * a[0] + wb * b[0],
        wa * a[1] + wb * b[1],
        wa * a[2] + wb * b[2],
    ]
}

fn coastlines() -> &'static [Vec<(f64, f64)>] {
    static LINES: OnceLock<Vec<Vec<(f64, f64)>>> = OnceLock::new();
    LINES.get_or_init(|| {
        COASTLINE
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .map(|line| {
                let points: Vec<(f64, f64)> = line
                    .split_whitespace()
                    .filter_map(|pair| {
                        let (lon, lat) = pair.split_once(',')?;
                        Some((lon.parse().ok()?, lat.parse().ok()?))
                    })
                    .collect();
                thin(&points)
            })
            .filter(|line| !small_polar(line))
            .collect()
    })
}

fn thin(line: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(line.len());
    for (index, &point) in line.iter().enumerate() {
        let keep = out
            .last()
            .is_none_or(|last| (point.0 - last.0).abs().max((point.1 - last.1).abs()) >= 2.0);
        if keep || index + 1 == line.len() {
            out.push(point);
        }
    }
    out
}

fn small_polar(line: &[(f64, f64)]) -> bool {
    let (mut west, mut east) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut south = f64::INFINITY;
    for &(lon, lat) in line {
        west = west.min(lon);
        east = east.max(lon);
        south = south.min(lat);
    }
    south > 60.0 && east - west < 40.0
}

fn dense(line: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(line.len() * 2);
    for pair in line.windows(2) {
        let ((lon0, lat0), (lon1, lat1)) = (pair[0], pair[1]);
        let span = (lon1 - lon0).abs().max((lat1 - lat0).abs());
        let parts = (1..=24_u32)
            .find(|&n| span / f64::from(n) <= 2.5)
            .unwrap_or(24);
        for n in 0..parts {
            let k = f64::from(n) / f64::from(parts);
            out.push((lon0 + (lon1 - lon0) * k, lat0 + (lat1 - lat0) * k));
        }
    }
    if let Some(&last) = line.last() {
        out.push(last);
    }
    out
}

const CITIES: [(&str, f64, f64); 9] = [
    ("JFK", 40.6, -73.8),
    ("LHR", 51.5, -0.5),
    ("DXB", 25.3, 55.4),
    ("SIN", 1.4, 104.0),
    ("HND", 35.6, 139.8),
    ("SFO", 37.6, -122.4),
    ("GRU", -23.4, -46.5),
    ("JNB", -26.1, 28.2),
    ("SYD", -33.9, 151.2),
];

const FLIGHTS: [(usize, usize, f64); 7] = [
    (0, 1, 0.1),
    (1, 2, 0.55),
    (2, 3, 0.3),
    (3, 8, 0.8),
    (5, 4, 0.0),
    (6, 7, 0.45),
    (0, 6, 0.7),
];

fn flush(ctx: &mut Context<'_>, run: &mut Vec<(f64, f64)>, style: Style) {
    stroke(ctx, run, style);
    run.clear();
}

#[derive(Clone, Copy)]
struct Globe {
    view: View,
    radius: f64,
    center: (f64, f64),
}

impl Globe {
    fn screen(self, point: [f64; 3], lift: f64) -> (f64, f64, bool) {
        let [x, y, depth] = self.view.turn(point);
        let reach = self.radius * (1.0 + lift);
        let shown = depth > 0.0 || (x * x + y * y) * (1.0 + lift).powi(2) > 1.0;
        (self.center.0 + x * reach, self.center.1 + y * reach, shown)
    }

    fn trace(self, ctx: &mut Context<'_>, path: impl Iterator<Item = (f64, f64)>, style: Style) {
        let mut run: Vec<(f64, f64)> = Vec::new();
        for (lat, lon) in path {
            let (x, y, shown) = self.screen(sphere(lat, lon), 0.0);
            if shown {
                run.push((x, y));
            } else {
                flush(ctx, &mut run, style);
            }
        }
        flush(ctx, &mut run, style);
    }

    fn flight(self, ctx: &mut Context<'_>, ink: Ink, (from, to): (usize, usize), progress: f64) {
        let (a, b) = (
            sphere(CITIES[from].1, CITIES[from].2),
            sphere(CITIES[to].1, CITIES[to].2),
        );
        let reach = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
            .clamp(-1.0, 1.0)
            .acos()
            / PI;
        let at = |s: f64| self.screen(slerp(a, b, s), (0.04 + 0.22 * reach) * (PI * s).sin());
        let leg = |ctx: &mut Context<'_>, s0: f64, s1: f64, style: Style| {
            let (p, q) = (at(s0), at(s1));
            if p.2 && q.2 {
                segment(ctx, (p.0, p.1), (q.0, q.1), style);
            }
        };
        let samples = 64_u32;
        for n in 0..samples {
            let (s0, s1) = (
                f64::from(n) / f64::from(samples),
                f64::from(n + 1) / f64::from(samples),
            );
            leg(ctx, s0, s1, ink.shade(ink.warm, 0.42));
        }
        let tail = 0.14;
        for n in 0..12_u32 {
            let (k0, k1) = (f64::from(n) / 12.0, f64::from(n + 1) / 12.0);
            let (s0, s1) = (progress - tail * (1.0 - k0), progress - tail * (1.0 - k1));
            if s0 >= 0.0 && s1 <= 1.0 {
                leg(ctx, s0, s1, ink.shade(ink.warm, 0.45 + 0.55 * k1));
            }
        }
        let tip = at(progress);
        if tip.2 && progress < 1.0 {
            ctx.draw(&Points {
                coords: &[(tip.0, tip.1)],
                style: Style::new().fg(ink.text),
            });
        }
    }
}

fn globe(stage: Stage, area: Rect, cx: &mut RenderCx<'_>) {
    let Stage { ink, hw, hh, t, .. } = stage;
    let globe = Globe {
        view: View::new(-30.0 + 9.0 * t, 24.0),
        radius: hh * 0.76,
        center: (-hw * 0.3, 0.0),
    };
    let progress = FLIGHTS.map(|(_, _, offset)| ((t * 0.75 + offset) % 1.2).min(1.0));
    stage.canvas(area, cx, move |ctx| {
        ctx.draw(&Circle {
            x: globe.center.0,
            y: globe.center.1,
            radius: globe.radius,
            style: ink.shade(ink.line, 0.62),
        });
        let guide = ink.shade(ink.text, 0.16);
        for meridian in (0..12_i32).map(|m| f64::from(m) * 30.0) {
            let path = (0..=60_i32).map(|step| (f64::from(step) * 3.0 - 90.0, meridian));
            globe.trace(ctx, path, guide);
        }
        for parallel in [-60.0, -30.0, 0.0, 30.0, 60.0] {
            let path = (0..=120_i32).map(|step| (parallel, f64::from(step) * 3.0));
            globe.trace(ctx, path, guide);
        }
        ctx.layer();
        let coast = ink.shade(ink.text, 0.75);
        for line in coastlines() {
            globe.trace(
                ctx,
                dense(line).into_iter().map(|(lon, lat)| (lat, lon)),
                coast,
            );
        }
        ctx.layer();
        for (&(from, to, _), &s) in FLIGHTS.iter().zip(&progress) {
            globe.flight(ctx, ink, (from, to), s);
        }
        for &(code, lat, lon) in &CITIES {
            let (x, y, _) = globe.screen(sphere(lat, lon), 0.0);
            if globe.view.turn(sphere(lat, lon))[2] > 0.0 {
                ctx.draw(&Points {
                    coords: &[(x, y)],
                    style: Style::new().fg(ink.text),
                });
                ctx.print(x + 0.8, y, code, ink.shade(ink.text, 0.7));
            }
        }
        let left = hw * 0.36;
        ctx.print(left, hh * 0.62, "Flights", ink.shade(ink.text, 0.85));
        for (row, (&(from, to, _), &s)) in (1_u32..).zip(FLIGHTS.iter().zip(&progress)) {
            let status = if s >= 1.0 {
                String::from("landed")
            } else {
                format!("{:>3.0}%", s * 100.0)
            };
            ctx.print(
                left,
                hh * 0.62 - 2.0 * f64::from(row),
                format!("{}  {}  {status}", CITIES[from].0, CITIES[to].0),
                ink.shade(ink.text, 0.6),
            );
        }
    });
}

const SAMPLE_DOTS: u32 = 3;
const SCROLL_DOTS: u32 = 1;

fn cpu_user(n: u32) -> f64 {
    let burst = if hash(n / 50 + 11) > 0.6 {
        28.0 * (PI * f64::from(n % 50) / 50.0).sin()
    } else {
        0.0
    };
    (14.0 + 34.0 * noise(n, 23, 1) + 14.0 * noise(n, 5, 2) + burst).clamp(1.0, 99.0)
}

fn cpu_system(n: u32) -> f64 {
    4.0 + 12.0 * noise(n, 17, 3) + 4.0 * noise(n, 4, 4)
}

fn memory(n: u32) -> f64 {
    let gc = f64::from(n % 140) / 140.0 * 9.0;
    56.0 + 14.0 * noise(n, 40, 5) + gc + 3.0 * noise(n, 6, 6)
}

fn net_rx(n: u32) -> f64 {
    let wave = noise(n, 13, 7);
    6.0 + 70.0 * wave * wave + 10.0 * noise(n, 3, 8)
}

fn net_tx(n: u32) -> f64 {
    let wave = noise(n, 19, 9);
    3.0 + 34.0 * wave * wave + 6.0 * noise(n, 4, 10)
}

type Series = (fn(u32) -> f64, Color, &'static str);

fn system(stage: Stage, area: Rect, cx: &mut RenderCx<'_>) {
    let Stage { ink, tick, .. } = stage;
    let now = tick * SCROLL_DOTS + 600;
    let rows = area.height / 3;
    let graphs: [(&str, &str, [Option<Series>; 2]); 3] = [
        (
            "CPU",
            "%",
            [
                Some((cpu_user, ink.line, "user")),
                Some((cpu_system, ink.warm, "sys")),
            ],
        ),
        ("Memory", "%", [Some((memory, ink.leaf, "used")), None]),
        (
            "Network",
            " MB/s",
            [
                Some((net_rx, ink.line, "rx")),
                Some((net_tx, ink.rose, "tx")),
            ],
        ),
    ];
    for (slot, (name, unit, series)) in graphs.into_iter().enumerate() {
        let top = area.y + rows * u16::try_from(slot).unwrap_or(0);
        let height = if slot == 2 {
            area.height - rows * 2
        } else {
            rows
        };
        let plot = Rect::new(
            area.x + 1,
            top + 1,
            area.width - 2,
            height.saturating_sub(2),
        );
        let dots = u32::from(plot.width) * 2;
        let last = dots - 1;
        Canvas::new(move |ctx| {
            for level in [25.0, 50.0, 75.0] {
                segment(
                    ctx,
                    (0.0, level),
                    (f64::from(last), level),
                    ink.shade(ink.text, 0.1),
                );
            }
            segment(
                ctx,
                (0.0, 0.0),
                (f64::from(last), 0.0),
                ink.shade(ink.text, 0.2),
            );
            ctx.layer();
            for (value, color, _) in series.iter().flatten() {
                let first = now.saturating_sub(last) / SAMPLE_DOTS;
                let points: Vec<(f64, f64)> = (first..=now / SAMPLE_DOTS + 1)
                    .map(|n| {
                        let x = f64::from(n * SAMPLE_DOTS) - f64::from(now) + f64::from(last);
                        (x, value(n))
                    })
                    .collect();
                stroke(ctx, &points, Style::new().fg(*color));
            }
            ctx.print(0.0, 57.0, "50%", ink.shade(ink.text, 0.4));
        })
        .pixels(stage.pixels)
        .x_bounds([0.0, f64::from(last)])
        .y_bounds([0.0, 100.0])
        .background(ink.panel())
        .render(plot, cx);
        let mut heading = format!(" {name}");
        let latest = |value: fn(u32) -> f64| {
            let n = now / SAMPLE_DOTS;
            let k = f64::from(now % SAMPLE_DOTS) / f64::from(SAMPLE_DOTS);
            value(n) + (value(n + 1) - value(n)) * k
        };
        for (value, _, label) in series.iter().flatten() {
            let _ = write!(heading, "    {label} {:.1}{unit}", latest(*value));
        }
        lunatui_widgets::Paragraph::new(heading)
            .style(ink.shade(ink.text, 0.75).bg(ink.bg))
            .render(Rect::new(area.x, top, area.width, 1), cx);
        let mut x = area.x + area.width;
        for (_, color, label) in series.iter().flatten().rev() {
            let text = format!("── {label} ");
            let width = u16::try_from(text.chars().count()).unwrap_or(0);
            x = x.saturating_sub(width + 1);
            lunatui_widgets::Paragraph::new(text)
                .style(Style::new().fg(*color).bg(ink.bg))
                .render(Rect::new(x, top, width, 1), cx);
        }
    }
}

const TREE_DEPTH: u32 = 7;

struct Branch {
    at: (f64, f64),
    angle: f64,
    length: f64,
    depth: u32,
    id: u32,
}

fn grow(ctx: &mut Context<'_>, ink: Ink, branch: &Branch, growth: f64, t: f64) {
    let part = (growth - f64::from(branch.depth)).clamp(0.0, 1.0);
    if part <= 0.0 {
        return;
    }
    let depth = f64::from(branch.depth) / f64::from(TREE_DEPTH);
    let angle = branch.angle + 0.035 * depth * (t * 3.2 + f64::from(branch.depth)).sin();
    let reach = branch.length * part;
    let end = (
        branch.at.0 + reach * angle.cos(),
        branch.at.1 + reach * angle.sin(),
    );
    let color = mix(ink.text, ink.leaf, depth);
    segment(ctx, branch.at, end, ink.shade(color, 0.55 + 0.45 * depth));
    if part < 1.0 {
        return;
    }
    if branch.depth + 1 >= TREE_DEPTH {
        return;
    }
    let id = branch.id;
    let spread = [0.24 + 0.2 * hash(id * 3), -(0.22 + 0.2 * hash(id * 3 + 1))];
    for (side, turn) in (0_u32..).zip(spread) {
        let child = id * 2 + side + 1;
        grow(
            ctx,
            ink,
            &Branch {
                at: end,
                angle: angle + turn,
                length: branch.length * (0.72 + 0.1 * hash(child + 500)),
                depth: branch.depth + 1,
                id: child,
            },
            growth,
            t,
        );
    }
}

fn clock_and_tree(stage: Stage, area: Rect, cx: &mut RenderCx<'_>) {
    let Stage { ink, hw, hh, t, .. } = stage;
    let center = (-hw * 0.48, hh * 0.04);
    let radius = hh * 0.76;
    let seconds = 8.0 * 60.0 + 42.0 + 2.0 * t;
    let growth = smooth(t / 0.8) * (f64::from(TREE_DEPTH) + 0.001);
    stage.canvas(area, cx, move |ctx| {
        ctx.draw(&Circle {
            x: center.0,
            y: center.1,
            radius,
            style: ink.shade(ink.line, 0.7),
        });
        let polar = |angle: f64, r: f64| (center.0 + r * angle.cos(), center.1 + r * angle.sin());
        for mark in 0..60_u32 {
            let angle = PI / 2.0 - TAU * f64::from(mark) / 60.0;
            let (inner, style) = if mark % 5 == 0 {
                (0.84, ink.shade(ink.text, 0.8))
            } else {
                (0.93, ink.shade(ink.text, 0.4))
            };
            segment(
                ctx,
                polar(angle, radius * inner),
                polar(angle, radius * 0.97),
                style,
            );
        }
        for (hour, label) in [(0.0, "12"), (3.0, "3"), (6.0, "6"), (9.0, "9")] {
            let angle = PI / 2.0 - TAU * hour / 12.0;
            let (x, y) = polar(angle, radius * 0.7);
            let shift = if label.len() == 2 { 0.6 } else { 0.3 };
            ctx.print(x - shift, y + 0.5, label, ink.shade(ink.text, 0.7));
        }
        ctx.layer();
        let hours = 10.0 + seconds / 3600.0;
        let minutes = seconds / 60.0;
        let hand = |turns: f64| PI / 2.0 - TAU * turns;
        let hour = hand(hours / 12.0);
        segment(
            ctx,
            center,
            polar(hour, radius * 0.5),
            Style::new().fg(ink.text),
        );
        let minute = hand(minutes / 60.0);
        segment(
            ctx,
            center,
            polar(minute, radius * 0.76),
            Style::new().fg(ink.text),
        );
        let second = hand(seconds / 60.0);
        segment(
            ctx,
            polar(second + PI, radius * 0.16),
            polar(second, radius * 0.9),
            Style::new().fg(ink.rose),
        );
        ctx.draw(&Circle {
            x: center.0,
            y: center.1,
            radius: 0.35,
            style: Style::new().fg(ink.rose),
        });
        let whole = seconds.floor();
        let (m, s) = ((whole / 60.0).floor(), whole % 60.0);
        ctx.print(
            center.0 - 2.4,
            -hh * 0.9,
            format!("10:{m:02.0}:{s:02.0}"),
            ink.shade(ink.text, 0.6),
        );
        ctx.layer();
        let ground = -hh * 0.9;
        segment(
            ctx,
            (hw * 0.08, ground),
            (hw * 0.9, ground),
            ink.shade(ink.text, 0.22),
        );
        grow(
            ctx,
            ink,
            &Branch {
                at: (hw * 0.54, ground),
                angle: PI / 2.0,
                length: hh * 0.4,
                depth: 0,
                id: 0,
            },
            growth,
            t,
        );
    });
}

const BOIDS: u32 = 200;
const TRAIL: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Boid {
    p: (f64, f64),
    v: (f64, f64),
}

fn wrap(value: f64, half: f64) -> f64 {
    (value + half).rem_euclid(2.0 * half) - half
}

struct Flock {
    boids: Vec<Boid>,
    trails: Vec<Vec<(f64, f64)>>,
    steps: u32,
    half: (f64, f64),
}

impl Flock {
    fn new(hw: f64, hh: f64) -> Self {
        let boids: Vec<Boid> = (0..BOIDS)
            .map(|n| {
                let angle = TAU * hash(n * 2 + 1);
                Boid {
                    p: (
                        hw * (hash(n * 2) * 1.8 - 0.9),
                        hh * (hash(n * 3 + 7) * 1.8 - 0.9),
                    ),
                    v: (0.4 * angle.cos(), 0.4 * angle.sin()),
                }
            })
            .collect();
        let trails = boids.iter().map(|b| vec![b.p]).collect();
        Self {
            boids,
            trails,
            steps: 0,
            half: (hw, hh),
        }
    }

    fn step(&mut self) {
        let (hw, hh) = self.half;
        let dt = BOID_RATE / f64::from(FPS);
        let before = self.boids.clone();
        for (boid, trail) in self.boids.iter_mut().zip(&mut self.trails) {
            let (mut sep, mut align, mut mass, mut seen) =
                ((0.0, 0.0), (0.0, 0.0), (0.0, 0.0), 0.0);
            for other in &before {
                let (dx, dy) = (
                    wrap(other.p.0 - boid.p.0, hw),
                    wrap(other.p.1 - boid.p.1, hh),
                );
                let d2 = dx * dx + dy * dy;
                if d2 == 0.0 || d2 > 12.0 {
                    continue;
                }
                seen += 1.0;
                align = (align.0 + other.v.0, align.1 + other.v.1);
                mass = (mass.0 + dx, mass.1 + dy);
                if d2 < 5.0 {
                    sep = (sep.0 - dx / d2, sep.1 - dy / d2);
                }
            }
            let mut v = boid.v;
            if seen > 0.0 {
                v.0 += dt * (0.06 * (align.0 / seen - v.0) + 0.003 * mass.0 / seen + 0.1 * sep.0);
                v.1 += dt * (0.06 * (align.1 / seen - v.1) + 0.003 * mass.1 / seen + 0.1 * sep.1);
            }
            let speed = (v.0 * v.0 + v.1 * v.1).sqrt().max(1e-6);
            let clamped = speed.clamp(0.3, 0.55);
            v = (v.0 / speed * clamped, v.1 / speed * clamped);
            boid.v = v;
            let moved = (boid.p.0 + v.0 * dt, boid.p.1 + v.1 * dt);
            boid.p = (wrap(moved.0, hw), wrap(moved.1, hh));
            if (boid.p.0 - moved.0).abs() + (boid.p.1 - moved.1).abs() > 1e-9 {
                trail.clear();
            }
            trail.push(boid.p);
            if trail.len() > TRAIL {
                trail.remove(0);
            }
        }
        self.steps += 1;
    }
}

thread_local! {
    static FLOCK: RefCell<Option<Flock>> = const { RefCell::new(None) };
}

fn flock_after(steps: u32, hw: f64, hh: f64) -> Vec<(Boid, Vec<(f64, f64)>)> {
    FLOCK.with_borrow_mut(|cache| {
        let fresh = cache.as_ref().is_none_or(|flock| {
            flock.steps > steps
                || flock.half.0.to_bits() != hw.to_bits()
                || flock.half.1.to_bits() != hh.to_bits()
        });
        if fresh {
            *cache = Some(Flock::new(hw, hh));
        }
        let Some(flock) = cache.as_mut() else {
            return Vec::new();
        };
        while flock.steps < steps {
            flock.step();
        }
        flock
            .boids
            .iter()
            .copied()
            .zip(flock.trails.iter().cloned())
            .collect()
    })
}

fn flock(stage: Stage, area: Rect, cx: &mut RenderCx<'_>) {
    let Stage {
        ink, hw, hh, tick, ..
    } = stage;
    let birds = flock_after(tick + 120, hw, hh);
    stage.canvas(area, cx, move |ctx| {
        let trace = ink.shade(ink.line, 0.42);
        for (_, trail) in &birds {
            stroke(ctx, trail, trace);
        }
        ctx.layer();
        for (n, (boid, _)) in (0_u32..).zip(&birds) {
            let speed = (boid.v.0 * boid.v.0 + boid.v.1 * boid.v.1).sqrt().max(1e-6);
            let (dx, dy) = (boid.v.0 / speed, boid.v.1 / speed);
            let size = 0.9;
            let tip = (boid.p.0 + dx * size, boid.p.1 + dy * size);
            let back = (boid.p.0 - dx * size * 0.4, boid.p.1 - dy * size * 0.4);
            let wing = (-dy * size * 0.45, dx * size * 0.45);
            let color = if n % 40 == 0 { ink.warm } else { ink.line };
            let style = Style::new().fg(color);
            segment(ctx, (back.0 + wing.0, back.1 + wing.1), tip, style);
            segment(ctx, (back.0 - wing.0, back.1 - wing.1), tip, style);
        }
        ctx.print(
            -hw + 1.0,
            -hh + 1.0,
            format!("{BOIDS} boids"),
            ink.shade(ink.text, 0.5),
        );
    });
}

const PING_TICKS: u32 = 60;

const PINGS: [(&str, f64, f64, u32, f64); 10] = [
    ("FRA", 50.1, 8.7, 0, 14.0),
    ("IAD", 38.9, -77.4, 20, 81.0),
    ("SJC", 37.4, -121.9, 6, 142.0),
    ("GRU", -23.4, -46.5, 42, 188.0),
    ("NRT", 35.8, 140.4, 26, 226.0),
    ("SIN", 1.4, 104.0, 12, 162.0),
    ("SYD", -33.9, 151.2, 56, 274.0),
    ("BOM", 19.1, 72.9, 18, 118.0),
    ("JNB", -26.1, 28.2, 32, 160.0),
    ("ARN", 59.6, 17.9, 36, 27.0),
];

fn pings(stage: Stage, area: Rect, cx: &mut RenderCx<'_>) {
    let Stage { ink, hw, tick, .. } = stage;
    let scale = hw * 0.96 / 180.0;
    let place = move |lat: f64, lon: f64| (lon * scale, (lat - 13.0) * scale);
    stage.canvas(area, cx, move |ctx| {
        let faint = ink.shade(ink.text, 0.1);
        for lon in (-6_i32..=6).map(|k| f64::from(k) * 30.0) {
            segment(ctx, place(-58.0, lon), place(84.0, lon), faint);
        }
        for lat in [-30.0, 30.0, 60.0] {
            segment(ctx, place(lat, -180.0), place(lat, 180.0), faint);
        }
        segment(
            ctx,
            place(0.0, -180.0),
            place(0.0, 180.0),
            ink.shade(ink.text, 0.16),
        );
        ctx.layer();
        let coast = ink.shade(ink.text, 0.7);
        for line in coastlines() {
            if line.iter().all(|&(_, lat)| lat < -56.0) {
                continue;
            }
            for pair in line.windows(2) {
                let ((lon0, lat0), (lon1, lat1)) = (pair[0], pair[1]);
                if (lon1 - lon0).abs() > 180.0 || lat0.min(lat1) < -58.0 {
                    continue;
                }
                segment(ctx, place(lat0, lon0), place(lat1, lon1), coast);
            }
        }
        ctx.layer();
        for &(_, lat, lon, offset, _) in &PINGS {
            let age = f64::from((tick + offset) % PING_TICKS) / f64::from(PING_TICKS);
            let at = place(lat, lon);
            let fade = (1.0 - age).powf(1.6);
            ctx.draw(&Circle {
                x: at.0,
                y: at.1,
                radius: 0.5 + age * 4.2,
                style: ink.shade(ink.warm, fade),
            });
        }
        ctx.layer();
        for &(code, lat, lon, offset, latency) in &PINGS {
            let at = place(lat, lon);
            ctx.draw(&Points {
                coords: &[at],
                style: Style::new().fg(ink.text),
            });
            let cycle = (tick + offset) / PING_TICKS;
            let jitter = hash(offset * 131 + cycle);
            ctx.print(
                at.0 + 0.9,
                at.1 - 0.4,
                format!("{code} {:.0} ms", latency + jitter * 6.0),
                ink.shade(ink.text, 0.62),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use lunatui_theme::Variant;

    #[derive(Debug, PartialEq)]
    struct Snapshot {
        steps: u32,
        half: (f64, f64),
        birds: Vec<Boid>,
        trails: Vec<Vec<(f64, f64)>>,
    }

    fn snapshot() -> Option<Snapshot> {
        FLOCK.with_borrow(|cache| {
            cache.as_ref().map(|flock| Snapshot {
                steps: flock.steps,
                half: flock.half,
                birds: flock.boids.clone(),
                trails: flock.trails.clone(),
            })
        })
    }

    #[test]
    fn held_flock_twins_use_identical_actual_birds_and_trails_before_and_after_resize()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut grid = Grid::new(120, 42);
        let mut clusters = ClusterTable::new();
        let theme = Theme::builtin(Variant::Dark);
        let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
        let mut previous: Option<Snapshot> = None;
        for (width, height) in [(57, 25), (53, 23)] {
            render_scene(&mut cx, Rect::new(1, 7, width, height), 4, 18, false);
            let left = snapshot().ok_or("first flock was not rendered")?;
            render_scene(&mut cx, Rect::new(60, 7, width, height), 4, 18, true);
            let right = snapshot().ok_or("second flock was not rendered")?;
            assert_eq!(left, right);
            assert_eq!(left.steps, 138);
            assert_eq!(left.birds.len(), 200);
            assert_eq!(left.trails.len(), 200);
            let moving = left
                .trails
                .iter()
                .find(|trail| trail.len() > 1)
                .ok_or("no moving trail")?;
            assert_ne!(moving.first(), moving.last());
            if let Some(before) = previous {
                assert_ne!(left.half, before.half);
                assert_ne!(left.birds, before.birds);
                assert_ne!(left.trails, before.trails);
            } else {
                assert_eq!(left.half, (16.5, 11.0));
            }
            previous = Some(left);
        }
        Ok(())
    }
}
