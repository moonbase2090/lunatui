// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, List, Menu, MenuEvent, MenuItem, MenuState, Paragraph, RenderCx, StatusBar,
    Tabs, TextInput, VecSource, Widget, WidgetState,
};

const MAIN: &[&str] = &[
    "use relay::{Config, Metrics, Server};",
    "use std::error::Error;",
    "",
    "#[tokio::main]",
    "async fn main() -> Result<(), Box<dyn Error>> {",
    "    let config = Config::from_env()?;",
    "    let metrics = Metrics::default();",
    "",
    "    let server = Server::bind(config.listen).await?;",
    "    let router = routes::build(config)",
    "        .with_tracing()",
    "        .with_metrics(metrics);",
    "",
    "    server.serve(router).await?;",
    "    Ok(())",
    "}",
    "",
    "mod routes;",
    "mod metrics;",
    "mod config;",
];
const METRICS: &[&str] = &[
    "use std::time::Instant;",
    "use crate::telemetry::{Counter, Histogram};",
    "",
    "pub struct RequestMetrics {",
    "    requests: Counter,",
    "    latency: Histogram,",
    "}",
    "",
    "impl RequestMetrics {",
    "    pub fn observe(&self, started: Instant) {",
    "        let elapsed = started.elapsed();",
    "        self.requests.increment(1);",
    "",
    "        self.latency.record(",
    "    }",
    "}",
    "",
    "// Durations use seconds throughout the gateway.",
    "// One sample is recorded for each request.",
];
const INSERT: &str = "elapsed.as_secs_f64());";
const FILES: &[&str] = &[
    "relay-gateway",
    "  src",
    "    main.rs",
    "    metrics.rs",
    "    config.rs",
    "    routes",
    "      health.rs",
    "      search.rs",
    "  tests",
    "    requests.rs",
    "  Cargo.toml",
    "  README.md",
    "  .gitignore",
];

#[derive(Clone, Copy)]
enum Action {
    New,
    Open,
    Save,
    Metrics,
    Config,
    Copy,
    Split,
    Shortcuts,
}

#[derive(Clone, Copy)]
enum Key {
    Activate,
    Down,
    Right,
    Up,
    Enter,
}

fn menu(clean: bool) -> MenuState<Action> {
    MenuState::new(vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New file", Action::New),
                MenuItem::item("Open file", Action::Open),
                MenuItem::item(
                    if clean { "Save (unchanged)" } else { "Save" },
                    Action::Save,
                )
                .disabled(clean),
                MenuItem::group(
                    "Recent",
                    vec![
                        MenuItem::item("metrics.rs", Action::Metrics),
                        MenuItem::item("config.rs", Action::Config),
                    ],
                ),
            ],
        ),
        MenuItem::group("Edit", vec![MenuItem::item("Copy", Action::Copy)]),
        MenuItem::group("View", vec![MenuItem::item("Split editor", Action::Split)]),
        MenuItem::group(
            "Help",
            vec![MenuItem::item("Keyboard shortcuts", Action::Shortcuts)],
        ),
    ])
}

struct Frame {
    menu: MenuState<Action>,
    metrics: bool,
    dirty: bool,
    inserted: String,
    activity: Vec<String>,
    status: String,
    tick: u32,
}

impl Frame {
    fn at(tick: u32) -> Self {
        let mut frame = Self {
            menu: menu(true),
            metrics: false,
            dirty: false,
            inserted: String::new(),
            activity: vec![
                "Workspace opened: relay-gateway".into(),
                "Active buffer: src/main.rs".into(),
            ],
            status: "src/main.rs  |  all changes saved".into(),
            tick,
        };
        for (at, key) in [
            (24, Key::Activate),
            (42, Key::Down),
            (60, Key::Down),
            (78, Key::Down),
            (102, Key::Right),
            (123, Key::Down),
            (144, Key::Up),
            (162, Key::Enter),
        ] {
            if tick >= at {
                frame.press(key);
            }
        }
        if frame.metrics && tick > 174 {
            let count = usize::try_from((tick - 174) / 3)
                .unwrap_or(0)
                .min(INSERT.len());
            frame.inserted = INSERT.chars().take(count).collect();
            frame.dirty = count > 0;
            if frame.dirty {
                frame.status = "Editing src/metrics.rs  |  unsaved changes".into();
                frame
                    .activity
                    .push(format!("Inserted {} of {} characters", count, INSERT.len()));
                frame.menu = menu(false);
            }
        }
        for (at, key) in [
            (270, Key::Activate),
            (276, Key::Down),
            (282, Key::Down),
            (288, Key::Down),
            (296, Key::Enter),
        ] {
            if tick >= at {
                frame.press(key);
            }
        }
        frame
    }

    fn press(&mut self, key: Key) {
        let previous = self.menu.selected().map(|item| item.label().to_owned());
        match key {
            Key::Activate => {
                self.menu.activate();
            }
            Key::Down => {
                self.menu.down();
            }
            Key::Right => {
                self.menu.right();
            }
            Key::Up => {
                self.menu.up();
            }
            Key::Enter => self.menu.select(),
        }
        if let Some(item) = self.menu.selected() {
            let selected = item.label().to_owned();
            self.status = format!("Selected {selected}  |  Enter to choose, Esc to close");
            self.activity.push(format!("Menu selection: {selected}"));
            if previous.as_deref() == Some("Open file") && selected == "Recent" {
                self.status = "Save skipped: the active buffer has no changes".into();
                self.activity.push(self.status.clone());
            }
        }
        for event in self.menu.drain_events().collect::<Vec<_>>() {
            match event {
                MenuEvent::Selected(Action::Metrics) => {
                    self.metrics = true;
                    self.status = "Opened src/metrics.rs from Recent".into();
                }
                MenuEvent::Selected(Action::Save) => {
                    self.dirty = false;
                    self.status = "Saved src/metrics.rs".into();
                }
                MenuEvent::Selected(_) => continue,
            }
            self.activity.push(self.status.clone());
            self.menu.reset();
        }
    }

    fn path(&self) -> &'static str {
        if self.metrics {
            "src/metrics.rs"
        } else {
            "src/main.rs"
        }
    }
}

#[derive(Clone, Copy)]
struct Ink {
    text: Color,
    muted: Color,
    accent: Color,
    surface: Color,
    raised: Color,
    focus: Color,
    ok: Color,
    warn: Color,
}

impl Ink {
    fn new(cx: &RenderCx<'_>) -> Self {
        Self {
            text: cx.token(Token::TextPrimary),
            muted: cx.token(Token::TextMuted),
            accent: cx.token(Token::Accent),
            surface: cx.token(Token::Surface0),
            raised: cx.token(Token::SurfaceRaised),
            focus: cx.token(Token::FocusFill),
            ok: cx.token(Token::Ok),
            warn: cx.token(Token::Warn),
        }
    }

    fn text(self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    fn panel(self, title: &str, area: Rect, cx: &mut RenderCx<'_>) -> Rect {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(format!(" {title} "))
            .style(self.text())
            .border_style(self.text().fg(self.muted))
            .title_style(self.text().fg(self.accent));
        let inner = block.inner(area);
        block.render(area, cx);
        inner
    }
}

pub(super) fn draw(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    *grid = Grid::new(grid.width(), grid.height());
    let frame = Frame::at(tick);
    let mut cx = RenderCx::new(grid, clusters, theme);
    let ink = Ink::new(&cx);
    let body = Rect::new(
        area.x,
        area.y.saturating_add(2),
        area.width,
        area.height.saturating_sub(4),
    );
    let sidebar = 26.min(body.width / 3);
    explorer(
        &frame,
        Rect::new(body.x, body.y, sidebar, body.height),
        ink,
        &mut cx,
    );
    let work = Rect::new(body.x + sidebar, body.y, body.width - sidebar, body.height);
    let console_h = 8.min(work.height / 3);
    editor(
        &frame,
        Rect::new(work.x, work.y, work.width, work.height - console_h),
        ink,
        &mut cx,
    );
    activity(
        &frame,
        Rect::new(
            work.x,
            work.y + work.height - console_h,
            work.width,
            console_h,
        ),
        ink,
        &mut cx,
    );
    StatusBar::new("  RELAY  /  gateway")
        .style(ink.text().bg(ink.raised))
        .right([(
            "main  |  Rust  |  UTF-8  ",
            ink.text().fg(ink.muted).bg(ink.raised),
        )])
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    StatusBar::new(format!("  {}", frame.status))
        .style(
            ink.text()
                .fg(if frame.dirty { ink.warn } else { ink.ok })
                .bg(ink.raised),
        )
        .right([(
            format!(
                "Ln 14, Col {}  ",
                if frame.metrics {
                    METRICS[13].len() + frame.inserted.len() + 1
                } else {
                    MAIN[13].len() + 1
                }
            ),
            ink.text().bg(ink.raised),
        )])
        .render(
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(2),
                area.width,
                1,
            ),
            &mut cx,
        );
    StatusBar::new(
        "  F10 Menu     Up/Down Navigate     Right Submenu     Enter Select     Esc Close",
    )
    .style(ink.text().fg(ink.muted))
    .render(
        Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        ),
        &mut cx,
    );
    let menu_area = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(3),
    );
    Block::new().style(ink.text().bg(ink.raised)).render(
        Rect::new(menu_area.x, menu_area.y, menu_area.width, 1),
        &mut cx,
    );
    Menu::new(&frame.menu)
        .state(WidgetState::Focused)
        .render(menu_area, &mut cx);
}

fn explorer(frame: &Frame, area: Rect, ink: Ink, cx: &mut RenderCx<'_>) {
    let inner = ink.panel("EXPLORER", area, cx);
    let files = VecSource::from_rows(FILES.iter().map(|file| vec![(*file).into()]).collect());
    List::new(files)
        .style(ink.text())
        .selected(Some(if frame.metrics { 3 } else { 2 }))
        .highlight_style(ink.text().bg(ink.focus).fg(ink.accent))
        .render(
            Rect::new(
                inner.x,
                inner.y + 1,
                inner.width,
                14.min(inner.height.saturating_sub(1)),
            ),
            cx,
        );
    for (offset, text) in [
        (16, "OUTLINE"),
        (
            18,
            if frame.metrics {
                "  RequestMetrics"
            } else {
                "  main()"
            },
        ),
        (
            19,
            if frame.metrics {
                "    observe(started)"
            } else {
                "  routes::build()"
            },
        ),
        (22, "SOURCE CONTROL"),
        (24, "  main"),
        (25, "  1 modified buffer"),
        (27, "WORKSPACE"),
        (28, "  Rust / Cargo"),
    ] {
        if offset < inner.height {
            Paragraph::new(if offset == 25 && !frame.dirty {
                "  Working tree clean"
            } else {
                text
            })
            .style(
                ink.text()
                    .fg(if offset == 16 || offset == 22 || offset == 27 {
                        ink.accent
                    } else {
                        ink.muted
                    }),
            )
            .render(
                Rect::new(
                    inner.x + 1,
                    inner.y + offset,
                    inner.width.saturating_sub(2),
                    1,
                ),
                cx,
            );
        }
    }
}

fn editor(frame: &Frame, area: Rect, ink: Ink, cx: &mut RenderCx<'_>) {
    let inner = ink.panel("EDITOR", area, cx);
    let tab = format!("metrics.rs{}", if frame.dirty { " *" } else { "" });
    Tabs::new(["main.rs", tab.as_str(), "config.rs"])
        .select(usize::from(frame.metrics))
        .style(ink.text().fg(ink.muted))
        .highlight_style(
            ink.text()
                .fg(ink.accent)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .render(
            Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1),
            cx,
        );
    let lines = if frame.metrics { METRICS } else { MAIN };
    for (index, line) in lines.iter().enumerate() {
        let row = u16::try_from(index).unwrap_or(u16::MAX).saturating_add(2);
        if row >= inner.height.saturating_sub(1) {
            break;
        }
        let y = inner.y + row;
        Paragraph::new(format!("{:>3}", index + 1))
            .style(ink.text().fg(ink.muted))
            .render(Rect::new(inner.x + 1, y, 4.min(inner.width), 1), cx);
        let value = if frame.metrics && index == 13 {
            format!("{line}{}", frame.inserted)
        } else {
            (*line).into()
        };
        let text = Rect::new(inner.x + 6, y, inner.width.saturating_sub(7), 1);
        if index == 13 {
            TextInput::new(&value, value.len())
                .style(ink.text().bg(ink.raised))
                .field_style(ink.text().bg(ink.raised))
                .cursor_style(ink.text().fg(ink.accent).bg(ink.raised))
                .show_cursor(!frame.menu.is_active() && (frame.tick / 15).is_multiple_of(2))
                .render(text, cx);
        } else {
            let color =
                if line.starts_with("use ") || line.contains("pub ") || line.starts_with("impl ") {
                    ink.accent
                } else if line.trim_start().starts_with("//") {
                    ink.muted
                } else {
                    ink.text
                };
            Paragraph::new(value)
                .style(ink.text().fg(color))
                .render(text, cx);
        }
    }
    Paragraph::new(frame.path())
        .style(ink.text().fg(ink.muted))
        .render(
            Rect::new(
                inner.x + 2,
                inner.y + inner.height.saturating_sub(1),
                inner.width.saturating_sub(3),
                1,
            ),
            cx,
        );
}

fn activity(frame: &Frame, area: Rect, ink: Ink, cx: &mut RenderCx<'_>) {
    let inner = ink.panel("ACTIVITY", area, cx);
    let skip = frame
        .activity
        .len()
        .saturating_sub(usize::from(inner.height));
    for (row, event) in (0_u16..).zip(frame.activity.iter().skip(skip)) {
        Paragraph::new(format!("  {event}"))
            .style(ink.text().fg(
                if event.starts_with("Saved") || event.starts_with("Opened") {
                    ink.ok
                } else {
                    ink.muted
                },
            ))
            .render(Rect::new(inner.x, inner.y + row, inner.width, 1), cx);
    }
}
