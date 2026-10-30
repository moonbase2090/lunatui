// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::ops::Range;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, CodeMark, CodeView, CodeViewState, LineGauge, Paragraph, RenderCx,
    StatusBar, Widget, WidgetState,
};

const SOURCE: [&str; 64] = [
    "//! A small time-bounded cache for rendered pages.",
    "",
    "use std::collections::HashMap;",
    "use std::time::{Duration, Instant};",
    "",
    "/// How long a page stays fresh before it is rendered again.",
    "pub const DEFAULT_TTL: Duration = Duration::from_secs(30);",
    "",
    "pub struct Entry {",
    "    body: String,",
    "    stored: Instant,",
    "    hits: u32,",
    "}",
    "",
    "pub struct PageCache {",
    "    entries: HashMap<String, Entry>,",
    "    ttl: Duration,",
    "    capacity: usize,",
    "}",
    "",
    "impl PageCache {",
    "    pub fn new(capacity: usize) -> Self {",
    "        Self { entries: HashMap::new(), ttl: DEFAULT_TTL, capacity }",
    "    }",
    "",
    "    pub fn with_ttl(mut self, ttl: Duration) -> Self {",
    "        self.ttl = ttl;",
    "        self",
    "    }",
    "",
    "    /// Returns the body for `path` while it is fresh.",
    "    pub fn get(&mut self, path: &str, now: Instant) -> Option<&str> {",
    "        let entry = self.entries.get_mut(path)?;",
    "        if now.duration_since(entry.stored) > self.ttl {",
    "            return None;",
    "        }",
    "        entry.hits += 1;",
    "        Some(&entry.body)",
    "    }",
    "",
    "    pub fn insert(&mut self, path: &str, body: String, now: Instant) {",
    "        if self.entries.len() >= self.capacity { self.evict_coldest(now); } // keep within the configured capacity before inserting",
    "        self.entries.insert(path.to_string(), Entry { body, stored: now, hits: 0 });",
    "    }",
    "",
    "    fn evict_coldest(&mut self, now: Instant) {",
    "        let coldest = self",
    "            .entries",
    "            .iter()",
    "            .min_by_key(|(_, entry)| (entry.hits, now - entry.stored))",
    "            .map(|(path, _)| path.clone());",
    "        if let Some(path) = coldest {",
    "            self.entries.remove(&path);",
    "        }",
    "    }",
    "",
    "    pub fn purge_stale(&mut self, now: Instant) -> usize {",
    "        let before = self.entries.len();",
    "        let ttl = self.ttl;",
    "        self.entries.retain(|_, entry| now.duration_since(entry.stored) <= ttl);",
    "        before - self.entries.len()",
    "    }",
    "}",
    "",
];

const MARKS: [(usize, CodeMark); 8] = [
    (16, CodeMark::Changed),
    (17, CodeMark::Added),
    (33, CodeMark::Breakpoint),
    (41, CodeMark::Error),
    (50, CodeMark::Warning),
    (57, CodeMark::Added),
    (58, CodeMark::Added),
    (59, CodeMark::Added),
];

const FILES: [&str; 9] = [
    "src/",
    "  lib.rs",
    "  cache.rs",
    "  render.rs",
    "  routes.rs",
    "tests/",
    "  cache.rs",
    "Cargo.toml",
    "README.md",
];

const PROBLEMS: [(&str, &str, &str); 3] = [
    (
        "error",
        "42:58",
        "E0308 mismatched types: expected `Instant`, found `&Instant`",
    ),
    (
        "warn",
        "51:42",
        "this `clone` may be avoided by returning the key by reference",
    ),
    ("warn", "17:5", "field `ttl` is never read outside `get`"),
];

const KEYWORDS: [&str; 18] = [
    "use", "pub", "const", "struct", "impl", "fn", "let", "mut", "self", "Self", "if", "return",
    "Some", "None", "Option", "for", "in", "usize",
];

#[derive(Clone, Copy)]
enum Key {
    Down,
    NextError,
    Right,
    Home,
    PageDown,
    NextChange,
}

const SCRIPT: [(u32, Key); 25] = [
    (6, Key::Down),
    (9, Key::Down),
    (12, Key::Down),
    (15, Key::Down),
    (18, Key::Down),
    (21, Key::Down),
    (24, Key::Down),
    (27, Key::Down),
    (30, Key::Down),
    (33, Key::Down),
    (36, Key::Down),
    (39, Key::Down),
    (42, Key::Down),
    (45, Key::Down),
    (48, Key::Down),
    (51, Key::Down),
    (54, Key::Down),
    (80, Key::NextError),
    (110, Key::Right),
    (116, Key::Right),
    (122, Key::Right),
    (128, Key::Right),
    (160, Key::Home),
    (175, Key::PageDown),
    (205, Key::NextChange),
];

struct App {
    view: CodeViewState,
    key: &'static str,
    problem: Option<usize>,
}

fn replay(area: Rect, tick: u32) -> App {
    let mut app = App {
        view: CodeViewState::new(),
        key: "",
        problem: None,
    };
    CodeView::new(&SOURCE).sync(area, &mut app.view);
    app.view.set_cursor(Some(0));
    for (at, key) in SCRIPT {
        if tick < at {
            break;
        }
        match key {
            Key::Down => {
                app.key = "j";
                app.view.down(1);
            }
            Key::NextError => {
                app.key = "]e";
                app.view.set_cursor(Some(41));
                app.problem = Some(0);
            }
            Key::Right => {
                app.key = "l";
                app.view.scroll_right(12);
            }
            Key::Home => {
                app.key = "0";
                app.view.scroll_left(u16::MAX);
            }
            Key::PageDown => {
                app.key = "PgDn";
                app.view.page_down();
                app.problem = None;
            }
            Key::NextChange => {
                app.key = "]c";
                app.view.set_cursor(Some(57));
            }
        }
    }
    app
}

fn highlight(theme: &Theme) -> impl Fn(usize, &str, &mut dyn FnMut(Range<usize>, Style)) + '_ {
    move |_, text, paint| {
        let keyword = Style::new().fg(theme.resolve(Token::Accent));
        let string = Style::new().fg(theme.resolve(Token::Ok));
        let number = Style::new().fg(theme.resolve(Token::Warn));
        let comment = Style::new()
            .fg(theme.resolve(Token::TextPrimary))
            .with_attrs(Attrs::new().set_dim(true).set_italic(true));
        let bytes = text.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            let byte = bytes[at];
            if text[at..].starts_with("//") {
                paint(at..text.len(), comment);
                return;
            }
            if byte == b'"' {
                let end = text[at + 1..]
                    .find('"')
                    .map_or(text.len(), |end| at + end + 2);
                paint(at..end, string);
                at = end;
            } else if byte.is_ascii_digit() {
                let end = at + text[at..].bytes().take_while(u8::is_ascii_digit).count();
                paint(at..end, number);
                at = end;
            } else if byte.is_ascii_alphabetic() || byte == b'_' {
                let end = at
                    + text[at..]
                        .bytes()
                        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                        .count();
                if KEYWORDS.contains(&&text[at..end]) {
                    paint(at..end, keyword);
                }
                at = end;
            } else {
                at += 1;
            }
        }
    }
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let text = theme.resolve(Token::TextPrimary);
    let fill = Style::new().fg(text).bg(theme.resolve(Token::Surface0));
    let raised = Style::new().fg(text).bg(theme.resolve(Token::Surface1));
    let accent = theme.resolve(Token::Accent);
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(fill).render(area, &mut cx);
    if area.width < 100 || area.height < 30 {
        return;
    }
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let seconds = 7 + tick / 30;
    Paragraph::new(format!(
        " acme/web · src/cache.rs · review #812{:>width$}",
        format!("14:22:{seconds:02} "),
        width = usize::from(area.width) - 38
    ))
    .style(raised.fg(accent).with_attrs(Attrs::new().set_bold(true)))
    .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);

    let files = Rect::new(area.x + 1, area.y + 2, 22, area.height - 4);
    files_pane(&mut cx, files, fill, dim, accent, theme);

    let editor = Rect::new(
        files.x + files.width + 1,
        area.y + 2,
        area.width - files.width - 3,
        area.height - 13,
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" src/cache.rs ")
        .style(fill)
        .border_style(fill.fg(theme.resolve(Token::FocusRing)))
        .title_style(
            fill.fg(theme.resolve(Token::FocusRing))
                .with_attrs(Attrs::new().set_bold(true)),
        );
    block.render(editor, &mut cx);
    let inner = block.inner(editor);
    let app = replay(inner, tick);
    let paint = highlight(theme);
    CodeView::new(&SOURCE)
        .view(&app.view)
        .marks(&MARKS)
        .highlight(&paint)
        .state(WidgetState::Focused)
        .render(inner, &mut cx);

    let problems = Rect::new(editor.x, editor.y + editor.height + 1, editor.width, 9);
    problems_pane(&mut cx, problems, &app, fill, dim, theme, tick);

    let cursor = app.view.cursor().unwrap_or(0) + 1;
    StatusBar::new("")
        .style(raised)
        .left([
            (
                if app.key.is_empty() {
                    String::new()
                } else {
                    format!(" key {} ", app.key)
                },
                raised.fg(theme.resolve(Token::FocusRing)),
            ),
            (
                format!(" Ln {cursor}, Col {} ", app.view.left() + 1),
                raised.fg(accent),
            ),
        ])
        .right([(
            String::from(" j down  ]e next error  ]c next change  l right  0 home  PgDn "),
            raised.with_attrs(Attrs::new().set_dim(true)),
        )])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
}

fn files_pane(
    cx: &mut RenderCx<'_>,
    pane: Rect,
    fill: Style,
    dim: Style,
    accent: Color,
    theme: &Theme,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Files ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    for (row, name) in (0_u16..).zip(FILES) {
        let style = if row == 2 {
            Style::new()
                .fg(theme.resolve(Token::Surface0))
                .bg(theme.resolve(Token::FocusRing))
                .with_attrs(Attrs::new().set_bold(true))
        } else if name.ends_with('/') {
            dim
        } else {
            fill
        };
        Paragraph::new(format!(" {name:<19}"))
            .style(style)
            .render(Rect::new(inner.x, inner.y + row, inner.width, 1), cx);
    }
}

fn problems_pane(
    cx: &mut RenderCx<'_>,
    pane: Rect,
    app: &App,
    fill: Style,
    dim: Style,
    theme: &Theme,
    tick: u32,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Problems · cargo check ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    for (row, (level, at, message)) in (0_u16..).zip(PROBLEMS) {
        let tone = if level == "error" {
            theme.resolve(Token::Fail)
        } else {
            theme.resolve(Token::Warn)
        };
        let selected = app.problem == Some(usize::from(row));
        let base = if selected {
            fill.bg(theme.resolve(Token::Surface1))
        } else {
            fill
        };
        let y = inner.y + row;
        Paragraph::new(" ●")
            .style(base.fg(tone))
            .render(Rect::new(inner.x, y, 3, 1), cx);
        Paragraph::new(format!("{level:<6}{at:<7}{message}"))
            .style(base.with_attrs(Attrs::new().set_bold(selected)))
            .render(Rect::new(inner.x + 3, y, inner.width - 3, 1), cx);
    }
    let run = (tick % 120) * 100 / 119;
    Paragraph::new(format!(" checking acme-web (lib) · {run:>3}%"))
        .style(dim)
        .render(Rect::new(inner.x, inner.y + 4, 40, 1), cx);
    LineGauge::new(f32::from(u8::try_from(run).unwrap_or(100)) / 100.0)
        .filled_style(fill.fg(theme.resolve(Token::Accent)))
        .unfilled_style(dim)
        .render(Rect::new(inner.x + 1, inner.y + 5, inner.width - 2, 1), cx);
}
