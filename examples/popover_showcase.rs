// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Size, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Constraint, Paragraph, Popover, PopoverClick, PopoverState, RenderCx,
    StatusBar, Table, VecSource, Widget,
};

const BUTTONS: [&str; 4] = ["Filter", "Columns", "Export", "Sort"];
const STATUSES: [&str; 3] = ["running", "failed", "queued"];
const SORTS: [&str; 3] = ["Newest first", "Oldest first", "By service"];
const COLUMNS: [&str; 3] = ["Env", "Version", "Age"];

const EVENTS: [(&str, &str); 12] = [
    ("12:04:51", "api v4.12.0 rolled out to 6 of 6 pods in prod"),
    ("12:04:12", "billing v2.8.4 waiting for a deploy slot"),
    ("12:03:40", "search v1.30.2 passed smoke tests in staging"),
    (
        "12:02:58",
        "web v9.1.0 failed health checks: 3 of 8 pods crash looping",
    ),
    ("12:02:05", "auth v3.4.1 rolled out to 4 of 4 pods in prod"),
    ("12:01:33", "worker v4.12.0 waiting for a deploy slot"),
    (
        "12:00:49",
        "notify v1.7.3 rolled out to 2 of 2 pods in prod",
    ),
    ("11:59:20", "export v0.9.8 failed: migration 0042 timed out"),
    (
        "11:58:02",
        "gateway v5.0.2 rolled out to 12 of 12 pods in prod",
    ),
    ("11:57:11", "reports v2.2.0 waiting for a deploy slot"),
    ("11:55:47", "ingest v3.9.5 passed smoke tests in staging"),
    ("11:54:30", "metrics v6.3.0 failed: image pull backoff"),
];

const DEPLOYS: [(&str, &str, &str, &str, u32); 14] = [
    ("api", "prod", "v4.12.0", "running", 3),
    ("billing", "prod", "v2.8.4", "queued", 5),
    ("search", "staging", "v1.30.2", "running", 8),
    ("web", "prod", "v9.1.0", "failed", 12),
    ("auth", "prod", "v3.4.1", "running", 15),
    ("worker", "staging", "v4.12.0", "queued", 19),
    ("notify", "prod", "v1.7.3", "running", 24),
    ("export", "staging", "v0.9.8", "failed", 31),
    ("gateway", "prod", "v5.0.2", "running", 38),
    ("reports", "prod", "v2.2.0", "queued", 44),
    ("ingest", "staging", "v3.9.5", "running", 52),
    ("audit", "prod", "v1.1.4", "running", 61),
    ("metrics", "prod", "v6.3.0", "failed", 70),
    ("cache", "staging", "v2.0.1", "running", 84),
];

#[derive(Clone, Copy)]
enum Step {
    Point(u32, u16, u16),
    Click(u32),
    Escape(u32),
}

const STEPS: [Step; 15] = [
    Step::Point(18, 4, 1),
    Step::Click(20),
    Step::Point(40, 5, 5),
    Step::Click(46),
    Step::Point(64, 40, 16),
    Step::Click(70),
    Step::Point(94, 114, 1),
    Step::Click(100),
    Step::Point(122, 104, 5),
    Step::Click(128),
    Step::Point(152, 14, 1),
    Step::Click(158),
    Step::Point(180, 15, 4),
    Step::Click(186),
    Step::Escape(206),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Filter,
    Columns,
    Sort,
}

struct App {
    shown: [bool; 3],
    sort: usize,
    columns: [bool; 3],
    open: Option<Panel>,
    state: PopoverState,
    action: &'static str,
}

impl App {
    const fn new() -> Self {
        Self {
            shown: [true; 3],
            sort: 0,
            columns: [true; 3],
            open: None,
            state: PopoverState::new(),
            action: "Click a toolbar button",
        }
    }

    fn popover(&self, area: Rect, which: Panel) -> Popover<Paragraph> {
        let (title, lines, anchor) = match which {
            Panel::Filter => (" Status ", self.filter_lines(), button(area, 0)),
            Panel::Columns => (" Columns ", self.column_lines(), button(area, 1)),
            Panel::Sort => (" Sort ", self.sort_lines(), button(area, 3)),
        };
        Popover::new(Paragraph::new(lines), anchor)
            .title(title)
            .size(Size::new(16, 3))
    }

    fn filter_lines(&self) -> String {
        lines(
            STATUSES
                .iter()
                .zip(self.shown)
                .map(|(name, on)| (*name, on)),
        )
    }

    fn column_lines(&self) -> String {
        lines(
            COLUMNS
                .iter()
                .zip(self.columns)
                .map(|(name, on)| (*name, on)),
        )
    }

    fn sort_lines(&self) -> String {
        SORTS
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mark = if index == self.sort { "◉" } else { "○" };
                format!(" {mark} {name}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn panel_area(&self, area: Rect) -> Option<Rect> {
        let which = self.open?;
        self.popover(area, which).area(area)
    }

    fn click(&mut self, area: Rect, (column, row): (u16, u16)) {
        for (index, which) in [(0, Panel::Filter), (1, Panel::Columns), (3, Panel::Sort)] {
            if self.open.is_some_and(|open| open != which) {
                continue;
            }
            let anchor = button(area, index);
            let panel = self.panel_area(area);
            match self.state.click(column, row, anchor, panel) {
                PopoverClick::Anchor => {
                    self.open = self.state.is_open().then_some(which);
                    self.action = if self.open.is_some() {
                        "Opened"
                    } else {
                        "Closed"
                    };
                    return;
                }
                PopoverClick::Inside => {
                    if let Some(panel) = panel {
                        self.choose(which, usize::from(row.saturating_sub(panel.y + 1)));
                    }
                    return;
                }
                _ => {}
            }
        }
        self.open = None;
        self.state.close();
        self.action = "Closed by clicking outside";
    }

    fn choose(&mut self, which: Panel, option: usize) {
        match which {
            Panel::Filter if option < 3 => {
                self.shown[option] = !self.shown[option];
                self.action = "Filter changed";
            }
            Panel::Columns if option < 3 => {
                self.columns[option] = !self.columns[option];
                self.action = "Columns changed";
            }
            Panel::Sort if option < 3 => {
                self.sort = option;
                self.open = None;
                self.state.close();
                self.action = "Sorted";
            }
            _ => {}
        }
    }
}

fn lines(items: impl Iterator<Item = (&'static str, bool)>) -> String {
    items
        .map(|(name, on)| format!(" {} {name}", if on { "☑" } else { "☐" }))
        .collect::<Vec<_>>()
        .join("\n")
}

fn button(area: Rect, index: usize) -> Rect {
    let width = |label: &str| u16::try_from(label.len()).unwrap_or(0) + 2;
    if index == 3 {
        let sort = width(BUTTONS[3]);
        return Rect::new(area.x + area.width - sort - 1, area.y + 1, sort, 1);
    }
    let mut x = area.x + 1;
    for label in &BUTTONS[..index] {
        x += width(label) + 1;
    }
    Rect::new(x, area.y + 1, width(BUTTONS[index]), 1)
}

fn eased(start: u16, end: u16, spent: u32, span: u32) -> u16 {
    let span = i64::from(span.max(1));
    let spent = i64::from(spent).min(span);
    let curve = spent * spent * (3 * span - 2 * spent);
    let (start, end) = (i64::from(start), i64::from(end));
    u16::try_from(start + (end - start) * curve / (span * span * span)).unwrap_or(0)
}

fn replay(area: Rect, tick: u32) -> (App, (u16, u16), bool) {
    let mut app = App::new();
    let mut from = (area.x + area.width / 2, area.y + area.height / 2);
    let mut start = 0_u32;
    let mut at = from;
    let mut pressed = false;
    for step in STEPS {
        match step {
            Step::Point(arrive, x, y) => {
                let to = (area.x + x, area.y + y);
                if tick < arrive {
                    let span = arrive - start;
                    let spent = tick.saturating_sub(start);
                    return (
                        app,
                        (
                            eased(from.0, to.0, spent, span),
                            eased(from.1, to.1, spent, span),
                        ),
                        false,
                    );
                }
                at = to;
            }
            Step::Click(when) => {
                if tick < when {
                    return (app, at, false);
                }
                app.click(area, at);
                pressed = tick < when + 3;
                from = at;
                start = when;
            }
            Step::Escape(when) => {
                if tick < when {
                    return (app, at, false);
                }
                app.open = None;
                app.state.close();
                app.action = "Closed with Escape";
            }
        }
    }
    (app, at, pressed)
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
    let focus = theme.resolve(Token::FocusRing);
    grid.fill(area, fill, ' ', clusters);
    if area.width < 60 || area.height < 20 {
        return;
    }
    let mut cx = RenderCx::new(grid, clusters, theme);
    let (app, pointer, pressed) = replay(area, tick);
    Paragraph::new(" Deploys")
        .style(raised.fg(accent).with_attrs(Attrs::new().set_bold(true)))
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    for (index, label) in BUTTONS.iter().enumerate() {
        let open = match index {
            0 => app.open == Some(Panel::Filter),
            1 => app.open == Some(Panel::Columns),
            3 => app.open == Some(Panel::Sort),
            _ => false,
        };
        let style = if open {
            fill.fg(focus).with_attrs(Attrs::new().set_bold(true))
        } else {
            raised
        };
        Paragraph::new(format!(" {label} "))
            .style(style)
            .render(button(area, index), &mut cx);
    }
    deploys(&mut cx, &app, area, fill, theme);
    activity(&mut cx, area, fill, accent, &app);
    StatusBar::new("")
        .style(raised)
        .left([(format!(" {} ", app.action), raised.fg(accent))])
        .right([(
            String::from(" Esc closes  Click outside closes "),
            raised.with_attrs(Attrs::new().set_dim(true)),
        )])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
    if let Some(which) = app.open {
        app.popover(area, which).render(area, &mut cx);
    }
    let glyph = if pressed { "●" } else { "↖" };
    let under = if pointer.1 == area.y + 1 {
        raised
    } else {
        fill
    };
    Paragraph::new(glyph)
        .style(under.fg(text).with_attrs(Attrs::new().set_bold(true)))
        .render(Rect::new(pointer.0, pointer.1, 1, 1), &mut cx);
}

fn deploys(cx: &mut RenderCx<'_>, app: &App, area: Rect, fill: Style, theme: &Theme) {
    let mut rows: Vec<_> = DEPLOYS
        .iter()
        .filter(|deploy| {
            STATUSES
                .iter()
                .position(|status| *status == deploy.3)
                .is_none_or(|index| app.shown[index])
        })
        .collect();
    match app.sort {
        1 => rows.reverse(),
        2 => rows.sort_by_key(|deploy| deploy.0),
        _ => {}
    }
    let status_color = |status: &str| -> Color {
        match status {
            "running" => theme.resolve(Token::Ok),
            "failed" => theme.resolve(Token::Fail),
            _ => theme.resolve(Token::Warn),
        }
    };
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let mut cells = Vec::new();
    let mut styles = Vec::new();
    let mut columns = vec![Constraint::Length(16)];
    let mut header = vec![("Service", dim)];
    for (shown, (name, width)) in
        app.columns
            .iter()
            .zip([("Env", 12), ("Version", 14), ("Age", 10)])
    {
        if *shown {
            columns.push(Constraint::Length(width));
            header.push((name, dim));
        }
    }
    columns.push(Constraint::Fill(1));
    header.push(("Status", dim));
    for (service, env, version, status, age) in rows {
        let mut row = vec![(*service).to_string()];
        let mut style = vec![fill];
        for (shown, value) in app.columns.iter().zip([
            (*env).to_string(),
            (*version).to_string(),
            format!("{age} min"),
        ]) {
            if *shown {
                row.push(value);
                style.push(dim);
            }
        }
        row.push(format!("● {status}"));
        style.push(fill.fg(status_color(status)));
        cells.push(row);
        styles.push(style);
    }
    Table::new(VecSource::from_rows(cells), columns)
        .header(header)
        .style(fill)
        .cell_styles(styles)
        .render(
            Rect::new(area.x + 2, area.y + 3, area.width - 4, area.height - 5),
            cx,
        );
}

fn activity(cx: &mut RenderCx<'_>, area: Rect, fill: Style, accent: Color, app: &App) {
    let pane = Rect::new(area.x + 1, area.y + 19, area.width - 2, area.height - 20);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Activity ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let shown = |line: &str| {
        STATUSES
            .iter()
            .zip(app.shown)
            .all(|(status, on)| on || !line.contains(status_word(status)))
    };
    for (row, (time, line)) in (0_u16..).zip(EVENTS.iter().filter(|(_, line)| shown(line))) {
        if row >= inner.height {
            break;
        }
        let y = inner.y + row;
        Paragraph::new(*time)
            .style(dim)
            .render(Rect::new(inner.x + 1, y, 9, 1), cx);
        Paragraph::new(*line)
            .style(fill)
            .render(Rect::new(inner.x + 11, y, inner.width - 12, 1), cx);
    }
}

fn status_word(status: &str) -> &'static str {
    match status {
        "failed" => "failed",
        "queued" => "waiting",
        _ => "rolled out",
    }
}
