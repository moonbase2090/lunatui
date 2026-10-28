// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style, clip_display_prefix};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Placement, Popup, PopupState, RenderCx, ScrollView, ScrollViewState, Widget,
    WidgetState,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position as RatPosition, Rect as RatRect, Size as RatSize};
use ratatui::style::{Color as RatColor, Modifier, Style as RatStyle};
use ratatui::text::{Line as RatLine, Span, Text as RatText};
use ratatui::widgets::{
    Block as RatBlock, BorderType as RatBorderType, Paragraph as RatParagraph,
    StatefulWidget as RatStatefulWidget, Widget as RatWidget,
};
use tui_popup::{KnownSizeWrapper, Popup as RatPopup};
use tui_scrollview::{ScrollView as RatScrollView, ScrollViewState as RatScrollViewState};

const BASE_LINES: u16 = 120;
const DEPLOY_LINES: u16 = 18;
const LOG_WIDTH: u16 = 156;
const SCROLL_END: u32 = 42;
const PAN_END: u32 = 60;
const DEPLOY_OPEN: u32 = 78;
const FADE: u32 = 4;
const TOGGLE: u32 = 112;
const DRAG_START: u32 = 135;
const DRAG_END: u32 = 190;
const CONFIRM: u32 = 204;
const ALERTS_OPEN: u32 = 228;
const ALERTS_SCROLL: u32 = 244;
const ALERTS_CLOSE: u32 = 288;
const SPIN: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

const COMPONENTS: [&str; 6] = ["api", "web", "worker", "edge", "db", "queue"];
const MESSAGES: [&str; 9] = [
    "health check passed in 4 ms",
    "compiled 212 crates, 0 warnings",
    "cache hit for target/release, restored 1.4 GB",
    "p99 latency 41 ms over the last 60 s",
    "retrying upstream connect (attempt 2 of 5)",
    "image pushed: registry/app:4812",
    "rotated 3 worker processes",
    "slow query: 812 ms on orders_by_customer",
    "canary at 10 percent, error rate 0.02 percent",
];
const TAILS: [&str; 3] = [
    "  path=releases/4812/assets/manifest.json sha=9f1c22a0e4b7 size=18.2 MB region=us-east",
    "  trace=7d0c41f2a9 span=checkout.submit user_agent=lunatui-console/0.1 status=200",
    "  hint: raise pool_size from 16 to 24 or add an index on (customer_id, created_at)",
];
const DEPLOY_STEPS: [&str; 6] = [
    "deploy 4812 started by release console",
    "draining api pods 1 of 3",
    "api 4812 healthy, shifting 33 percent",
    "web assets uploaded, cache warmed",
    "worker queue drained, restarting",
    "all services on 4812",
];

struct Service {
    name: &'static str,
    from: &'static str,
    note: &'static str,
}

const SERVICES: [Service; 6] = [
    Service {
        name: "api",
        from: "4811",
        note: "3 pods",
    },
    Service {
        name: "web",
        from: "4811",
        note: "cdn",
    },
    Service {
        name: "worker",
        from: "4810",
        note: "queue 0",
    },
    Service {
        name: "edge",
        from: "4812",
        note: "12 pops",
    },
    Service {
        name: "db",
        from: "pg16",
        note: "primary",
    },
    Service {
        name: "queue",
        from: "4811",
        note: "lag 0 s",
    },
];

const ALERTS: [(&str, &str, &str); 12] = [
    ("FAIL", "db", "slow query on orders_by_customer"),
    ("WARN", "api", "p99 above 40 ms for 5 min"),
    ("WARN", "edge", "cache hit rate 81 percent"),
    ("FAIL", "worker", "3 jobs retried twice"),
    ("WARN", "queue", "consumer lag 4 s at peak"),
    ("INFO", "web", "bundle grew 12 KB"),
    ("WARN", "api", "upstream connect retries"),
    ("INFO", "db", "vacuum finished in 41 s"),
    ("WARN", "edge", "TLS renewal in 9 days"),
    ("INFO", "worker", "autoscaled to 6 replicas"),
    ("FAIL", "api", "2 pods restarted"),
    ("INFO", "queue", "new partition added"),
];

#[derive(Clone, Copy)]
struct Ink {
    text: Color,
    accent: Color,
    surface: Color,
    raised: Color,
    ok: Color,
    warn: Color,
    fail: Color,
    focus: Color,
}

impl Ink {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            accent: cx.token(Token::Accent),
            surface: cx.token(Token::Surface0),
            raised: cx.token(Token::Surface1),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
            fail: cx.token(Token::Fail),
            focus: cx.token(Token::FocusRing),
        }
    }

    fn level(self, level: &str) -> Color {
        match level {
            "FAIL" => self.fail,
            "WARN" => self.warn,
            " OK " => self.ok,
            _ => self.accent,
        }
    }
}

fn bold() -> Attrs {
    Attrs::new().set_bold(true)
}

fn dim() -> Attrs {
    Attrs::new().set_dim(true)
}

fn ease(from: u32, to: u32, tick: u32) -> f32 {
    if tick <= from {
        return 0.0;
    }
    if tick >= to {
        return 1.0;
    }
    let span = f32::from(u16::try_from(to - from).unwrap_or(1));
    let p = f32::from(u16::try_from(tick - from).unwrap_or(0)) / span;
    p * p * (3.0 - 2.0 * p)
}

fn lerp(from: u16, to: u16, amount: f32) -> u16 {
    let value = f32::from(from) + (f32::from(to) - f32::from(from)) * amount;
    let rounded = value.round().clamp(0.0, f32::from(u16::MAX));
    let mut out = 0_u16;
    while f32::from(out) < rounded {
        out += 1;
    }
    out
}

fn appended(tick: u32) -> u16 {
    if tick < CONFIRM {
        return 0;
    }
    u16::try_from((tick - CONFIRM) / 3)
        .unwrap_or(u16::MAX)
        .min(DEPLOY_LINES)
}

fn log_line(row: u16) -> (String, &'static str, &'static str, String) {
    let seconds = 3 + u32::from(row) * 7 / 10;
    let millis = (u32::from(row) * 137) % 1000;
    let stamp = format!("12:{:02}:{:02}.{millis:03}", seconds / 60 + 3, seconds % 60);
    if row >= BASE_LINES {
        let step = usize::from(row - BASE_LINES);
        let message = DEPLOY_STEPS[step.min(DEPLOY_STEPS.len() - 1)];
        let level = if step + 1 >= DEPLOY_STEPS.len() {
            " OK "
        } else {
            "INFO"
        };
        let text = if step < DEPLOY_STEPS.len() {
            message.to_owned()
        } else {
            format!(
                "health check {} passed, all green",
                step - DEPLOY_STEPS.len() + 1
            )
        };
        return (stamp, level, "deploy", text);
    }
    let index = usize::from(row);
    let level = match index % 11 {
        4 => "WARN",
        7 => "FAIL",
        2 | 9 => " OK ",
        _ => "INFO",
    };
    let mut text = MESSAGES[index % MESSAGES.len()].to_owned();
    if index % 4 == 1 {
        text.push_str(TAILS[index % TAILS.len()]);
    }
    (stamp, level, COMPONENTS[index % COMPONENTS.len()], text)
}

struct LogPage {
    ink: Ink,
    lines: u16,
    shown: u16,
}

impl Widget for LogPage {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let ink = self.ink;
        for row in 0..self.lines.min(area.height).min(BASE_LINES + self.shown) {
            let (stamp, level, component, text) = log_line(row);
            let y = area.y + row;
            let fresh = row >= BASE_LINES;
            cx.put_str(
                area.x,
                y,
                &stamp,
                Style::new().fg(ink.text).with_attrs(dim()),
            );
            cx.put_str(
                area.x + 13,
                y,
                level,
                Style::new().fg(ink.level(level)).with_attrs(bold()),
            );
            cx.put_str(
                area.x + 18,
                y,
                &format!("{component:<6}"),
                Style::new().fg(if fresh { ink.ok } else { ink.accent }),
            );
            let style = if fresh {
                Style::new().fg(ink.text).with_attrs(bold())
            } else {
                Style::new().fg(ink.text)
            };
            cx.put_str(area.x + 25, y, &text, style);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Button {
    Cancel,
    Deploy,
}

struct DeployDialog {
    ink: Ink,
    selected: Button,
    seconds: u32,
}

impl Widget for DeployDialog {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let ink = self.ink;
        let panel = Style::new().fg(ink.text).bg(ink.raised);
        let rows: [(&str, &str); 3] = [
            ("services", "api, web, worker restart"),
            ("migrations", "none"),
            ("rollback", "30 min window"),
        ];
        put(
            cx,
            area,
            0,
            1,
            "Ship build 4812 to production?",
            panel.with_attrs(bold()),
        );
        for (index, (key, value)) in (2_u16..).zip(rows) {
            put(cx, area, index, 3, key, panel.with_attrs(dim()));
            put(cx, area, index, 16, value, panel);
        }
        let hold = format!("auto-cancel in {:>2}s", self.seconds);
        put(cx, area, 6, 1, &hold, panel.with_attrs(dim()));
        let chip = |label: &'static str, on: bool| {
            if on {
                (
                    label,
                    Style::new()
                        .fg(ink.surface)
                        .bg(ink.accent)
                        .with_attrs(bold()),
                )
            } else {
                (label, panel)
            }
        };
        let cancel = chip(" Cancel ", self.selected == Button::Cancel);
        let deploy = chip(" Deploy ", self.selected == Button::Deploy);
        let right = area.width.saturating_sub(20);
        put(cx, area, 6, right, cancel.0, cancel.1);
        put(cx, area, 6, right + 10, deploy.0, deploy.1);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(44, 7).clamp(avail)
    }
}

struct AlertList {
    ink: Ink,
}

impl Widget for AlertList {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let ink = self.ink;
        for (row, (level, service, text)) in (0_u16..).zip(ALERTS).take(usize::from(area.height)) {
            let y = area.y + row;
            cx.put_str(area.x + 1, y, "●", Style::new().fg(ink.level(level)));
            cx.put_str(
                area.x + 3,
                y,
                level,
                Style::new().fg(ink.level(level)).with_attrs(bold()),
            );
            cx.put_str(area.x + 8, y, service, Style::new().fg(ink.accent));
            cx.put_str(area.x + 15, y, text, Style::new().fg(ink.text));
        }
    }

    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn put(cx: &mut RenderCx<'_>, area: Rect, row: u16, col: u16, text: &str, style: Style) {
    if row >= area.height || col >= area.width {
        return;
    }
    let room = area.width - col;
    cx.put_str(
        area.x + col,
        area.y + row,
        clip_display_prefix(text, room),
        style,
    );
}

fn key_for(tick: u32) -> Option<&'static str> {
    match tick {
        0..SCROLL_END => Some("j"),
        SCROLL_END..PAN_END => Some("l"),
        DEPLOY_OPEN..90 => Some("d"),
        TOGGLE..118 => Some("tab"),
        DRAG_START..DRAG_END => Some("drag"),
        198..CONFIRM => Some("enter"),
        ALERTS_OPEN..ALERTS_SCROLL => Some("a"),
        ALERTS_SCROLL..282 => Some("j"),
        282..ALERTS_CLOSE => Some("esc"),
        _ => None,
    }
}

fn draw_header(cx: &mut RenderCx<'_>, ink: Ink, area: Rect, tick: u32) {
    let bar = Style::new().fg(ink.text).bg(ink.raised);
    cx.set_style(Rect::new(area.x, area.y, area.width, 1), bar);
    let shipping = tick >= CONFIRM;
    let (badge, color) = if shipping && appended(tick) < DEPLOY_LINES {
        (" rolling ", ink.warn)
    } else if shipping {
        (" live ", ink.ok)
    } else {
        (" staged ", ink.accent)
    };
    cx.put_str(
        area.x + 1,
        area.y,
        badge,
        Style::new().fg(ink.surface).bg(color).with_attrs(bold()),
    );
    let after = area.x + 2 + u16::try_from(badge.len()).unwrap_or(0);
    cx.put_str(after, area.y, "release console", bar.with_attrs(bold()));
    cx.put_str(
        after + 16,
        area.y,
        "build 4812 / prod",
        bar.with_attrs(dim()),
    );
    let seconds = 7 + tick / 30;
    let clock = format!("12:04:{seconds:02}");
    let x = area.x + area.width.saturating_sub(10);
    if x > after + 40 {
        cx.put_str(
            x,
            area.y,
            &clock,
            Style::new().fg(ink.accent).bg(ink.raised),
        );
    }
}

fn chip_rect(area: Rect) -> Rect {
    let label = 12;
    Rect::new(
        area.x + area.width.saturating_sub(label + 1),
        area.y + area.height - 1,
        label,
        1,
    )
}

fn draw_keys(cx: &mut RenderCx<'_>, ink: Ink, area: Rect, tick: u32) {
    let y = area.y + area.height - 1;
    let bar = Style::new().fg(ink.text).bg(ink.raised);
    cx.set_style(Rect::new(area.x, y, area.width, 1), bar);
    let keys = [
        ("j", "scroll"),
        ("l", "pan"),
        ("d", "deploy"),
        ("tab", "choose"),
        ("drag", "move"),
        ("enter", "confirm"),
        ("a", "alerts"),
        ("esc", "close"),
    ];
    let pressed = key_for(tick);
    let chip = chip_rect(area);
    let mut x = area.x + 1;
    for (key, label) in keys {
        let used = u16::try_from(key.len() + label.len() + 3).unwrap_or(u16::MAX);
        if x + used >= chip.x {
            break;
        }
        let style = if pressed == Some(key) {
            Style::new()
                .fg(ink.surface)
                .bg(ink.accent)
                .with_attrs(bold())
        } else {
            Style::new()
                .fg(ink.accent)
                .bg(ink.raised)
                .with_attrs(bold())
        };
        cx.put_str(x, y, &format!(" {key} "), style);
        let at = x + u16::try_from(key.len() + 2).unwrap_or(0);
        cx.put_str(at, y, label, bar.with_attrs(dim()));
        x += used + 1;
    }
    let open = (ALERTS_OPEN..ALERTS_CLOSE).contains(&tick);
    let style = if open {
        Style::new().fg(ink.surface).bg(ink.warn).with_attrs(bold())
    } else {
        Style::new().fg(ink.warn).bg(ink.raised).with_attrs(bold())
    };
    cx.put_str(chip.x, chip.y, " ▲ 12 alerts", style);
}

fn draw_services(cx: &mut RenderCx<'_>, ink: Ink, area: Rect, tick: u32) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Services ")
        .style(Style::new().fg(ink.text).bg(ink.surface))
        .border_style(Style::new().fg(ink.text).bg(ink.surface).with_attrs(dim()))
        .title_style(Style::new().fg(ink.text).bg(ink.surface).with_attrs(bold()));
    block.render(area, cx);
    let inner = block.inner(area);
    let done = appended(tick);
    for (index, service) in (0_u16..).zip(SERVICES.iter()) {
        let row = index * 2 + 1;
        if row >= inner.height {
            break;
        }
        let ready_at = 3 + index * 3;
        let (glyph, color, version) = if tick < CONFIRM || service.from == "4812" {
            (
                "●",
                if index == 4 { ink.accent } else { ink.ok },
                service.from,
            )
        } else if done >= ready_at {
            ("●", ink.ok, "4812")
        } else {
            let frame = usize::try_from(tick / 2).unwrap_or(0) % SPIN.len();
            (SPIN[frame], ink.warn, "4812")
        };
        let base = Style::new().bg(ink.surface);
        put(cx, inner, row, 1, glyph, base.fg(color));
        put(
            cx,
            inner,
            row,
            3,
            service.name,
            base.fg(ink.text).with_attrs(bold()),
        );
        put(cx, inner, row, 11, version, base.fg(ink.accent));
        put(
            cx,
            inner,
            row,
            17,
            service.note,
            base.fg(ink.text).with_attrs(dim()),
        );
    }
}

fn log_offset(tick: u32, area: Rect, view: &ScrollView<LogPage>) -> ScrollViewState {
    let mut scroll = ScrollViewState::new();
    view.sync(area, &mut scroll);
    if tick >= CONFIRM {
        scroll.scroll_to_bottom();
        let (_, bottom) = scroll.offset();
        scroll.set_offset(0, bottom);
        return scroll;
    }
    let down = lerp(0, 8, ease(6, SCROLL_END, tick));
    let pan = lerp(0, 4, ease(SCROLL_END + 4, PAN_END, tick)) * 3;
    scroll.set_offset(pan, down);
    scroll
}

fn draw_log(cx: &mut RenderCx<'_>, ink: Ink, area: Rect, tick: u32, focused: bool) {
    let line = if focused {
        Style::new().fg(ink.focus).bg(ink.surface)
    } else {
        Style::new().fg(ink.text).bg(ink.surface).with_attrs(dim())
    };
    let lines = BASE_LINES + DEPLOY_LINES;
    let shown = appended(tick);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Build log ")
        .title_inset(1)
        .style(Style::new().fg(ink.text).bg(ink.surface))
        .border_style(line)
        .title_style(line.with_attrs(bold()));
    block.render(area, cx);
    let inner = block.inner(area);
    let view = ScrollView::new(LogPage { ink, lines, shown }, Size::new(LOG_WIDTH, lines)).state(
        if focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        },
    );
    let scroll = log_offset(tick, inner, &view);
    let (x, y) = scroll.offset();
    let total = BASE_LINES + shown;
    let last = (y + inner.height).min(total);
    let position = format!(" {}-{last} of {total}  col {x} ", y + 1);
    let at = area.x
        + area
            .width
            .saturating_sub(u16::try_from(position.len()).unwrap_or(0) + 2);
    if at > area.x + 16 {
        cx.put_str(at, area.y, &position, line);
    }
    view.scroll(&scroll).render(inner, cx);
}

fn deploy_popup(ink: Ink, tick: u32) -> Popup<DeployDialog> {
    let selected = if tick >= TOGGLE {
        Button::Deploy
    } else {
        Button::Cancel
    };
    let seconds = 30 - (tick.saturating_sub(DEPLOY_OPEN) / 30).min(29);
    let dragging = (DRAG_START..DRAG_END).contains(&tick);
    let title = if dragging {
        " Deploy - moving "
    } else {
        " Deploy "
    };
    Popup::new(DeployDialog {
        ink,
        selected,
        seconds,
    })
    .title(title)
    .size(Size::new(44, 7))
    .state(WidgetState::Focused)
}

fn deploy_position(popup: &Popup<DeployDialog>, area: Rect, tick: u32) -> PopupState {
    let mut state = PopupState::new();
    popup.sync(area, &mut state);
    if tick < DRAG_START {
        return state;
    }
    let Some(start) = state.area() else {
        return state;
    };
    let grab = (start.x + 4, start.y);
    state.press(grab.0, grab.1);
    let amount = ease(DRAG_START, DRAG_END, tick);
    let dx = lerp(0, area.width / 5, amount);
    let middle = DRAG_START.midpoint(DRAG_END);
    let lift = lerp(0, 4, ease(DRAG_START, middle, tick));
    let drop = lerp(0, 2, ease(middle, DRAG_END, tick));
    state.drag(grab.0 + dx, (grab.1 + drop).saturating_sub(lift));
    if tick >= DRAG_END {
        state.release();
    }
    state
}

fn alerts_popup(ink: Ink, area: Rect, tick: u32) -> Popup<ScrollView<AlertList>> {
    let rows = u16::try_from(ALERTS.len()).unwrap_or(0);
    let page = 6_u16.min(area.height.saturating_sub(6)).max(1);
    let down = lerp(0, rows - page, ease(ALERTS_SCROLL, 280, tick));
    let view = ScrollView::new(AlertList { ink }, Size::new(52, rows))
        .offset(0, down)
        .state(WidgetState::Focused);
    Popup::new(view)
        .title(" Alerts: 12 open ")
        .size(Size::new(53, page))
        .placement(Placement::Above(chip_rect(area)))
        .state(WidgetState::Focused)
}

/// Draws the release console at `tick` (30 per second) into `area`.
pub fn draw(grid: &mut Grid, clusters: &mut ClusterTable, theme: &Theme, area: Rect, tick: u32) {
    let surface = theme.resolve(Token::Surface0);
    let text = theme.resolve(Token::TextPrimary);
    grid.fill(area, Style::new().fg(text).bg(surface), ' ', clusters);
    if area.width < 40 || area.height < 12 {
        return;
    }
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ink = Ink::new(&cx);
    draw_header(&mut cx, ink, area, tick);
    let body = Rect::new(area.x, area.y + 1, area.width, area.height - 2);
    let side = if area.width >= 100 { 34 } else { 0 };
    let deploy_open = (DEPLOY_OPEN..CONFIRM).contains(&tick);
    let alerts_open = (ALERTS_OPEN..ALERTS_CLOSE).contains(&tick);
    let log = Rect::new(body.x, body.y, body.width - side, body.height);
    draw_log(&mut cx, ink, log, tick, !deploy_open && !alerts_open);
    if side > 0 {
        let services = Rect::new(body.x + body.width - side, body.y, side, body.height);
        draw_services(&mut cx, ink, services, tick);
    }
    draw_keys(&mut cx, ink, area, tick);
    if deploy_open {
        let fade = ease(DEPLOY_OPEN, DEPLOY_OPEN + FADE, tick);
        let popup = deploy_popup(ink, tick);
        let state = deploy_position(&popup, area, tick);
        if fade > 0.05 {
            popup
                .position(&state)
                .backdrop(0.55 * fade)
                .render(area, &mut cx);
        }
    }
    if alerts_open {
        alerts_popup(ink, area, tick)
            .backdrop(0.0)
            .render(area, &mut cx);
    }
}

fn rat(color: Color) -> RatColor {
    match color {
        Color::Rgb(red, green, blue) => RatColor::Rgb(red, green, blue),
        Color::Indexed(index) => RatColor::Indexed(index),
        _ => RatColor::Reset,
    }
}

fn rat_style(style: Style) -> RatStyle {
    let mut out = RatStyle::new().fg(rat(style.fg)).bg(rat(style.bg));
    if style.attrs.bold() {
        out = out.add_modifier(Modifier::BOLD);
    }
    if style.attrs.dim() {
        out = out.add_modifier(Modifier::DIM);
    }
    out
}

fn log_spans(ink: Ink, row: u16) -> RatLine<'static> {
    let (stamp, level, component, text) = log_line(row);
    let fresh = row >= BASE_LINES;
    let body = if fresh {
        Style::new().fg(ink.text).with_attrs(bold())
    } else {
        Style::new().fg(ink.text)
    };
    RatLine::from(vec![
        Span::styled(
            stamp,
            rat_style(Style::new().fg(ink.text).with_attrs(dim())),
        ),
        Span::raw(" "),
        Span::styled(
            level,
            rat_style(Style::new().fg(ink.level(level)).with_attrs(bold())),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{component:<6}"),
            rat_style(Style::new().fg(if fresh { ink.ok } else { ink.accent })),
        ),
        Span::raw(" "),
        Span::styled(text, rat_style(body)),
    ])
}

struct RatAlerts {
    ink: Ink,
    offset: u16,
}

impl RatWidget for &RatAlerts {
    fn render(self, area: RatRect, buf: &mut Buffer) {
        let ink = self.ink;
        let rows = u16::try_from(ALERTS.len()).unwrap_or(0);
        let mut view = RatScrollView::new(RatSize::new(52, rows));
        let lines: Vec<RatLine<'static>> = ALERTS
            .iter()
            .map(|(level, service, text)| {
                let tone = rat_style(Style::new().fg(ink.level(level)));
                RatLine::from(vec![
                    Span::styled(" ● ", tone),
                    Span::styled(*level, tone.add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{service:<6}"),
                        rat_style(Style::new().fg(ink.accent)),
                    ),
                    Span::raw(" "),
                    Span::styled(*text, rat_style(Style::new().fg(ink.text))),
                ])
            })
            .collect();
        let page = view.area();
        view.render_widget(RatParagraph::new(lines), page);
        let mut state = RatScrollViewState::with_offset(RatPosition::new(0, self.offset));
        RatStatefulWidget::render(view, area, buf, &mut state);
    }
}

fn rat_put(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) {
    let area = buf.area;
    if x < area.width && y < area.height {
        buf.set_stringn(x, y, text, usize::from(area.width - x), rat_style(style));
    }
}

fn rat_services(buffer: &mut Buffer, ink: Ink, services: RatRect, tick: u32) {
    let block = RatBlock::bordered()
        .border_type(RatBorderType::Rounded)
        .title(" Services ")
        .style(rat_style(Style::new().fg(ink.text).bg(ink.surface)))
        .border_style(rat_style(Style::new().fg(ink.text).with_attrs(dim())));
    let inner = block.inner(services);
    block.render(services, buffer);
    for (index, service) in (0_u16..).zip(SERVICES.iter()) {
        let row = inner.y + index * 2 + 1;
        if row >= inner.y + inner.height {
            break;
        }
        let version = if tick >= CONFIRM && appended(tick) >= 3 + index * 3 {
            "4812"
        } else {
            service.from
        };
        rat_put(buffer, inner.x + 1, row, "●", Style::new().fg(ink.ok));
        rat_put(
            buffer,
            inner.x + 3,
            row,
            service.name,
            Style::new().fg(ink.text).with_attrs(bold()),
        );
        rat_put(
            buffer,
            inner.x + 11,
            row,
            version,
            Style::new().fg(ink.accent),
        );
        rat_put(
            buffer,
            inner.x + 17,
            row,
            service.note,
            Style::new().fg(ink.text).with_attrs(dim()),
        );
    }
}

fn rat_deploy(buffer: &mut Buffer, ink: Ink, area: RatRect, tick: u32) {
    let lunatui_area = Rect::new(area.x, area.y, area.width, area.height);
    let popup = deploy_popup(ink, tick);
    let at = deploy_position(&popup, lunatui_area, tick)
        .area()
        .unwrap_or(lunatui_area);
    let choice = if tick >= TOGGLE {
        "[Deploy]"
    } else {
        "[Cancel]"
    };
    let body = RatText::from(vec![
        RatLine::raw(" Ship build 4812 to production?"),
        RatLine::raw(""),
        RatLine::raw("   services     api, web, worker restart"),
        RatLine::raw("   migrations   none"),
        RatLine::raw("   rollback     30 min window"),
        RatLine::raw(""),
        RatLine::raw(format!(" choose: {choice:<35}")),
    ]);
    let mut state = tui_popup::PopupState::default();
    let rat_popup = RatPopup::new(body)
        .title(" Deploy ")
        .style(rat_style(Style::new().fg(ink.text).bg(ink.raised)))
        .border_style(rat_style(Style::new().fg(ink.focus)));
    let mut scratch = Buffer::empty(area);
    RatStatefulWidget::render(&rat_popup, area, &mut scratch, &mut state);
    state.move_to(at.x, at.y);
    RatStatefulWidget::render(&rat_popup, area, buffer, &mut state);
}

fn rat_alerts(buffer: &mut Buffer, ink: Ink, area: RatRect, tick: u32) {
    let lunatui_area = Rect::new(area.x, area.y, area.width, area.height);
    let rows = u16::try_from(ALERTS.len()).unwrap_or(0);
    let page = 6_u16.min(area.height.saturating_sub(6)).max(1);
    let offset = lerp(0, rows - page, ease(ALERTS_SCROLL, 280, tick));
    let at = alerts_popup(ink, lunatui_area, tick).area(lunatui_area);
    let body = KnownSizeWrapper {
        inner: RatAlerts { ink, offset },
        width: 53,
        height: usize::from(page),
    };
    let mut state = tui_popup::PopupState::default();
    let rat_popup = RatPopup::new(body)
        .title(" Alerts: 12 open ")
        .style(rat_style(Style::new().fg(ink.text).bg(ink.raised)))
        .border_style(rat_style(Style::new().fg(ink.focus)));
    let mut scratch = Buffer::empty(area);
    RatStatefulWidget::render(&rat_popup, area, &mut scratch, &mut state);
    state.move_to(at.x, at.y);
    RatStatefulWidget::render(&rat_popup, area, buffer, &mut state);
}

/// Draws the same console with ratatui, tui-scrollview and tui-popup for side-by-side runs.
pub fn baseline(buffer: &mut Buffer, area: RatRect, theme: &Theme, tick: u32) {
    let resolve = |token| theme.resolve(token);
    let ink = Ink {
        text: resolve(Token::TextPrimary),
        accent: resolve(Token::Accent),
        surface: resolve(Token::Surface0),
        raised: resolve(Token::Surface1),
        ok: resolve(Token::Ok),
        warn: resolve(Token::Warn),
        fail: resolve(Token::Fail),
        focus: resolve(Token::FocusRing),
    };
    buffer.set_style(area, rat_style(Style::new().fg(ink.text).bg(ink.surface)));
    if area.width < 40 || area.height < 12 {
        return;
    }
    let bar = Style::new().fg(ink.text).bg(ink.raised);
    buffer.set_style(RatRect::new(0, 0, area.width, 1), rat_style(bar));
    rat_put(buffer, 1, 0, " release console ", bar.with_attrs(bold()));
    rat_put(buffer, 19, 0, "build 4812 / prod", bar.with_attrs(dim()));
    let side = if area.width >= 100 { 34 } else { 0 };
    let deploy_open = (DEPLOY_OPEN..CONFIRM).contains(&tick);
    let alerts_open = (ALERTS_OPEN..ALERTS_CLOSE).contains(&tick);
    let focused = !deploy_open && !alerts_open;
    let border = if focused {
        Style::new().fg(ink.focus)
    } else {
        Style::new().fg(ink.text).with_attrs(dim())
    };
    let log_area = RatRect::new(0, 1, area.width - side, area.height - 2);
    let block = RatBlock::bordered()
        .border_type(RatBorderType::Rounded)
        .title(" Build log ")
        .style(rat_style(Style::new().fg(ink.text).bg(ink.surface)))
        .border_style(rat_style(border));
    let inner = block.inner(log_area);
    block.render(log_area, buffer);
    let lines = BASE_LINES + DEPLOY_LINES;
    let shown = appended(tick);
    let mut view = RatScrollView::new(RatSize::new(LOG_WIDTH, lines));
    let page = view.area();
    view.render_widget(
        RatParagraph::new(
            (0..BASE_LINES + shown)
                .map(|row| log_spans(ink, row))
                .collect::<Vec<_>>(),
        ),
        page,
    );
    let lunatui_inner = Rect::new(inner.x, inner.y, inner.width, inner.height);
    let probe = ScrollView::new(LogPage { ink, lines, shown }, Size::new(LOG_WIDTH, lines));
    let (x, y) = log_offset(tick, lunatui_inner, &probe).offset();
    let mut state = RatScrollViewState::with_offset(RatPosition::new(x, y));
    RatStatefulWidget::render(view, inner, buffer, &mut state);
    if side > 0 {
        let services = RatRect::new(area.width - side, 1, side, area.height - 2);
        rat_services(buffer, ink, services, tick);
    }
    let keys_y = area.height - 1;
    buffer.set_style(RatRect::new(0, keys_y, area.width, 1), rat_style(bar));
    rat_put(
        buffer,
        1,
        keys_y,
        " j scroll  l pan  d deploy  tab choose  enter confirm  a alerts",
        bar.with_attrs(dim()),
    );
    let lunatui_area = Rect::new(area.x, area.y, area.width, area.height);
    let chip = chip_rect(lunatui_area);
    rat_put(
        buffer,
        chip.x,
        chip.y,
        " ▲ 12 alerts",
        Style::new().fg(ink.warn).bg(ink.raised),
    );
    if deploy_open {
        rat_deploy(buffer, ink, area, tick);
    }
    if alerts_open {
        rat_alerts(buffer, ink, area, tick);
    }
}
