// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use core::time::Duration;

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Paragraph, RenderCx, StatusBar, Tabs, Tooltip, TooltipSide, TooltipState,
    Widget,
};

const FPS: u32 = 30;
const MOVE_TICKS: u32 = 12;
const REST_TICKS: u32 = 33;
const DELAY: Duration = Duration::from_millis(400);
const SIDEBAR: u16 = 26;
const GUTTER_LINE: u16 = 13;

const TOOLS: [(&str, &str); 6] = [
    ("New", "New file  Ctrl+N"),
    ("Open", "Open a file or folder  Ctrl+O"),
    ("Save", "Save every changed file  Ctrl+S"),
    ("Find", "Search the whole workspace  Ctrl+Shift+F"),
    ("Run", "Run the release checks  F5"),
    ("Debug", "Start the debugger on this target"),
];
const SIDE_TOOLS: [(&str, &str); 2] = [
    ("Share", "Copy a link to the current line"),
    ("Settings", "Editor, theme and key bindings"),
];
const STATUS_LEFT: [(&str, &str); 2] = [
    (" main ", "Branch main, 2 commits ahead of origin"),
    (" 3 problems ", "2 warnings and 1 error in src/release.rs"),
];
const STATUS_RIGHT: [(&str, &str); 2] = [
    (" Ln 42, Col 7 ", "Go to line  Ctrl+G"),
    (" UTF-8 ", "File encoding"),
];
const WARNING: &str =
    "unused variable `tag`\nPrefix it with an underscore to silence this warning.";
const PATH: [usize; 6] = [2, 4, 7, 8, 10, 11];

const FILES: [&str; 16] = [
    "▾ release-tools",
    "  ▾ src",
    "    main.rs",
    "    release.rs",
    "    notes.rs",
    "    checks.rs",
    "  ▾ tests",
    "    release.rs",
    "    notes.rs",
    "  ▸ docs",
    "  ▸ scripts",
    "  Cargo.toml",
    "  Cargo.lock",
    "  CHANGELOG.md",
    "  README.md",
    "  LICENSE",
];

const CODE: [&str; 28] = [
    "use std::process::Command;",
    "",
    "use crate::checks::{Check, Outcome};",
    "use crate::notes::Notes;",
    "",
    "pub struct Release {",
    "    version: String,",
    "    checks: Vec<Check>,",
    "}",
    "",
    "impl Release {",
    "    pub fn tag(&self) -> Result<String, Error> {",
    "        let tag = format!(\"v{}\", self.version);",
    "        let name = self.version.clone();",
    "        let status = Command::new(\"git\")",
    "            .args([\"tag\", \"-s\", &name, \"-m\", &name])",
    "            .status()?;",
    "        if !status.success() {",
    "            return Err(Error::Tag(name));",
    "        }",
    "        Ok(name)",
    "    }",
    "",
    "    pub fn run_checks(&self) -> Outcome {",
    "        self.checks.iter().map(Check::run).collect()",
    "    }",
    "",
    "    pub fn notes(&self) -> Notes {",
];

#[derive(Clone, Copy)]
struct Target {
    anchor: Rect,
    text: &'static str,
    side: TooltipSide,
    max_width: u16,
    point: (u16, u16),
}

struct Ink {
    fill: Style,
    raised: Style,
    dim: Style,
    accent: Color,
    warn: Color,
    text: Color,
    focus: Color,
}

impl Ink {
    fn new(theme: &Theme) -> Self {
        let text = theme.resolve(Token::TextPrimary);
        let fill = Style::new().fg(text).bg(theme.resolve(Token::Surface0));
        Self {
            fill,
            raised: Style::new().fg(text).bg(theme.resolve(Token::Surface1)),
            dim: fill.with_attrs(Attrs::new().set_dim(true)),
            accent: theme.resolve(Token::Accent),
            warn: theme.resolve(Token::Warn),
            text,
            focus: theme.resolve(Token::FocusRing),
        }
    }
}

fn cells(text: &str) -> u16 {
    u16::try_from(text.chars().count()).unwrap_or(0)
}

fn targets(area: Rect) -> Vec<Target> {
    let mut out = Vec::new();
    let mut x = area.x + 1;
    for (label, text) in TOOLS {
        let anchor = Rect::new(x, area.y, cells(label) + 2, 1);
        out.push(toolbar_target(anchor, text));
        x += anchor.width + 1;
    }
    let side_width: u16 = SIDE_TOOLS.iter().map(|(label, _)| cells(label) + 3).sum();
    let mut x = (area.x + area.width).saturating_sub(side_width);
    for (label, text) in SIDE_TOOLS {
        let anchor = Rect::new(x, area.y, cells(label) + 2, 1);
        out.push(toolbar_target(anchor, text));
        x += anchor.width + 1;
    }
    let editor = editor_area(area);
    let gutter = Rect::new(editor.x + 1, editor.y + GUTTER_LINE, 6, 1);
    out.push(Target {
        anchor: gutter,
        text: WARNING,
        side: TooltipSide::Right,
        max_width: 44,
        point: (gutter.x, gutter.y),
    });
    let bottom = area.y + area.height - 1;
    let mut x = area.x;
    for (label, text) in STATUS_LEFT {
        out.push(status_target(Rect::new(x, bottom, cells(label), 1), text));
        x += cells(label);
    }
    let right_width: u16 = STATUS_RIGHT.iter().map(|(label, _)| cells(label)).sum();
    let mut x = (area.x + area.width).saturating_sub(right_width);
    for (label, text) in STATUS_RIGHT {
        out.push(status_target(Rect::new(x, bottom, cells(label), 1), text));
        x += cells(label);
    }
    out
}

fn toolbar_target(anchor: Rect, text: &'static str) -> Target {
    Target {
        anchor,
        text,
        side: TooltipSide::Below,
        max_width: 40,
        point: (anchor.x + anchor.width - 1, anchor.y),
    }
}

fn status_target(anchor: Rect, text: &'static str) -> Target {
    Target {
        anchor,
        text,
        side: TooltipSide::Below,
        max_width: 44,
        point: (anchor.x + anchor.width - 1, anchor.y),
    }
}

fn editor_area(area: Rect) -> Rect {
    Rect::new(
        area.x + SIDEBAR,
        area.y + 2,
        area.width - SIDEBAR,
        area.height - 3,
    )
}

fn eased(start: u16, end: u16, spent: u32, span: u32) -> u16 {
    let span = i64::from(span.max(1));
    let spent = i64::from(spent).min(span);
    let curve = spent * spent * (3 * span - 2 * spent);
    let (start, end) = (i64::from(start), i64::from(end));
    u16::try_from(start + (end - start) * curve / (span * span * span)).unwrap_or(0)
}

fn pointer(targets: &[Target], area: Rect, tick: u32) -> (u16, u16) {
    let leg = tick / (MOVE_TICKS + REST_TICKS);
    let within = tick % (MOVE_TICKS + REST_TICKS);
    let index = usize::try_from(leg).unwrap_or(0).min(PATH.len() - 1);
    let to = targets[PATH[index]].point;
    let from = match index {
        0 => (area.x + area.width / 2, area.y + area.height / 2),
        _ => targets[PATH[index - 1]].point,
    };
    if leg >= u32::try_from(PATH.len()).unwrap_or(0) {
        return to;
    }
    (
        eased(from.0, to.0, within, MOVE_TICKS),
        eased(from.1, to.1, within, MOVE_TICKS),
    )
}

fn at(tick: u32) -> Duration {
    Duration::from_millis(u64::from(tick) * 1000 / u64::from(FPS))
}

fn contains(area: Rect, (x, y): (u16, u16)) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

fn hovered(targets: &[Target], area: Rect, tick: u32) -> (Option<usize>, bool) {
    let mut state = TooltipState::with_delay(DELAY);
    let mut over = None;
    for now in 0..=tick {
        let point = pointer(targets, area, now);
        let hit = targets
            .iter()
            .position(|target| contains(target.anchor, point));
        if hit != over {
            state.hover(false, at(now));
            over = hit;
        }
        state.hover(hit.is_some(), at(now));
    }
    (over, state.is_shown(at(tick)))
}

pub(super) fn render(
    grid: &mut Grid,
    clusters: &mut ClusterTable,
    theme: &Theme,
    area: Rect,
    tick: u32,
) {
    let ink = Ink::new(theme);
    grid.fill(area, ink.fill, ' ', clusters);
    if area.width < 60 || area.height < 20 {
        return;
    }
    let mut cx = RenderCx::new(grid, clusters, theme);
    let targets = targets(area);
    let (over, shown) = hovered(&targets, area, tick);
    toolbar(&mut cx, &ink, area, over);
    Tabs::new([" release.rs ", " notes.rs ", " Cargo.toml "])
        .select(0)
        .style(ink.dim)
        .highlight_style(
            ink.fill
                .fg(ink.focus)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .indent(SIDEBAR)
        .render(Rect::new(area.x, area.y + 1, area.width, 1), &mut cx);
    sidebar(&mut cx, &ink, area);
    editor(&mut cx, &ink, area);
    status(&mut cx, &ink, area, over);
    if let (Some(index), true) = (over, shown) {
        let target = targets[index];
        Tooltip::new(target.text, target.anchor)
            .side(target.side)
            .max_width(target.max_width)
            .render(area, &mut cx);
    }
    let (x, y) = pointer(&targets, area, tick);
    let under = if y == area.y || y + 1 == area.y + area.height {
        ink.raised
    } else {
        ink.fill
    };
    Paragraph::new("↖")
        .style(under.fg(ink.text).with_attrs(Attrs::new().set_bold(true)))
        .render(Rect::new(x, y, 1, 1), &mut cx);
}

fn toolbar(cx: &mut RenderCx<'_>, ink: &Ink, area: Rect, over: Option<usize>) {
    let button = |index: usize, label: &'static str| {
        let style = if over == Some(index) {
            ink.raised
                .fg(ink.focus)
                .with_attrs(Attrs::new().set_bold(true))
        } else {
            ink.raised
        };
        [
            (format!(" {label} "), style),
            (String::from(" "), ink.raised),
        ]
    };
    let left: Vec<(String, Style)> = core::iter::once((String::from(" "), ink.raised))
        .chain(
            TOOLS
                .iter()
                .enumerate()
                .flat_map(|(index, (label, _))| button(index, label)),
        )
        .collect();
    let right: Vec<(String, Style)> = SIDE_TOOLS
        .iter()
        .enumerate()
        .flat_map(|(index, (label, _))| button(index + TOOLS.len(), label))
        .collect();
    StatusBar::new("")
        .style(ink.raised)
        .left(left)
        .right(right)
        .render(Rect::new(area.x, area.y, area.width, 1), cx);
}

fn sidebar(cx: &mut RenderCx<'_>, ink: &Ink, area: Rect) {
    let pane = Rect::new(area.x, area.y + 2, SIDEBAR, area.height - 3);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Files ")
        .style(ink.fill)
        .border_style(ink.dim)
        .title_style(ink.fill.fg(ink.accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    for (row, name) in (0_u16..).zip(FILES) {
        if row >= inner.height {
            break;
        }
        let style = if row == 3 {
            ink.raised.fg(ink.focus)
        } else if name.trim_start().starts_with('▾') || name.trim_start().starts_with('▸') {
            ink.fill.fg(ink.accent)
        } else {
            ink.fill
        };
        Paragraph::new(name).style(style).render(
            Rect::new(inner.x + 1, inner.y + row, inner.width - 1, 1),
            cx,
        );
    }
}

fn editor(cx: &mut RenderCx<'_>, ink: &Ink, area: Rect) {
    let pane = editor_area(area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" src/release.rs ")
        .style(ink.fill)
        .border_style(ink.dim)
        .title_style(ink.fill.fg(ink.accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    for (row, line) in (0_u16..).zip(CODE) {
        if row >= inner.height {
            break;
        }
        let number = row + 1;
        let y = inner.y + row;
        Paragraph::new(format!("{number:>4}"))
            .style(ink.dim)
            .render(Rect::new(inner.x, y, 4, 1), cx);
        if number == GUTTER_LINE {
            Paragraph::new("▲")
                .style(ink.fill.fg(ink.warn))
                .render(Rect::new(inner.x + 5, y, 1, 1), cx);
        }
        let style = if line.trim_start().starts_with("pub fn") || line.starts_with("impl") {
            ink.fill.fg(ink.accent)
        } else {
            ink.fill
        };
        Paragraph::new(line)
            .style(style)
            .render(Rect::new(inner.x + 7, y, inner.width - 7, 1), cx);
    }
}

fn status(cx: &mut RenderCx<'_>, ink: &Ink, area: Rect, over: Option<usize>) {
    let first = TOOLS.len() + SIDE_TOOLS.len() + 1;
    let pick = |index: usize, style: Style| {
        if over == Some(index) {
            style.fg(ink.focus).with_attrs(Attrs::new().set_bold(true))
        } else {
            style
        }
    };
    let left = [
        (STATUS_LEFT[0].0, pick(first, ink.raised)),
        (STATUS_LEFT[1].0, pick(first + 1, ink.raised.fg(ink.warn))),
    ];
    let right = [
        (STATUS_RIGHT[0].0, pick(first + 2, ink.raised)),
        (STATUS_RIGHT[1].0, pick(first + 3, ink.raised)),
    ];
    StatusBar::new("")
        .style(ink.raised)
        .left(left)
        .right(right)
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            cx,
        );
}
