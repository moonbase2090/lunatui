// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Color, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Dialog, DialogFocus, DialogKind, DialogOutcome, DialogState, LineGauge,
    Paragraph, RenderCx, StatusBar, Widget,
};

const BRANCH_LIST: u32 = 1;

const BRANCHES: [(&str, &str, u16, u16, &str); 7] = [
    ("main", "Bump the cache layer to v3", 0, 0, "Ana Ruiz"),
    (
        "release/2.4",
        "Freeze strings for the 2.4 build",
        2,
        0,
        "Kofi Mensah",
    ),
    (
        "feature/login",
        "Draft the passkey sign-in screen",
        3,
        14,
        "Lee Park",
    ),
    (
        "feature/search",
        "Rank results by recent visits",
        5,
        2,
        "Iris Costa",
    ),
    (
        "fix/cache-headers",
        "Send Vary on Accept-Encoding",
        1,
        0,
        "Sam Okafor",
    ),
    (
        "docs/onboarding",
        "Rewrite the first-day checklist",
        4,
        6,
        "Mia Chen",
    ),
    (
        "chore/deps",
        "Update the image pipeline crates",
        2,
        1,
        "Raj Patel",
    ),
];

const ACTIVITY: [&str; 12] = [
    "ci   main            build 1482 passed in 4m 12s",
    "push feature/search  2 commits from Iris Costa",
    "ci   release/2.4     tests 2,318 of 2,318 passed",
    "pr   #812 fix/cache-headers ready for review",
    "ci   chore/deps      audit found no new advisories",
    "push docs/onboarding 1 commit from Mia Chen",
    "ci   feature/search  lint passed, tests running",
    "pr   #809 release/2.4 approved by 2 reviewers",
    "ci   fix/cache-headers build 1483 passed in 3m 58s",
    "push main            merge of #806 by Ana Ruiz",
    "ci   docs/onboarding link check passed",
    "pr   #814 chore/deps opened by Raj Patel",
];

#[derive(Clone, Copy)]
enum Key {
    Down,
    Up,
    Delete,
    Rename,
    Tab,
    Enter,
    Char(char),
}

const SCRIPT: [(u32, Key); 13] = [
    (6, Key::Down),
    (12, Key::Down),
    (20, Key::Delete),
    (60, Key::Tab),
    (100, Key::Enter),
    (110, Key::Rename),
    (128, Key::Char('-')),
    (138, Key::Char('v')),
    (148, Key::Char('2')),
    (190, Key::Enter),
    (198, Key::Up),
    (204, Key::Up),
    (212, Key::Delete),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    Delete,
    Rename,
    Protected,
}

struct App {
    branches: Vec<(String, &'static str, u16, u16, &'static str)>,
    selected: usize,
    dialog: DialogState,
    ask: Option<Ask>,
    text: String,
    focus: Option<u32>,
    key: &'static str,
    note: String,
}

impl App {
    fn new() -> Self {
        Self {
            branches: BRANCHES
                .iter()
                .map(|(name, commit, ahead, behind, author)| {
                    ((*name).to_string(), *commit, *ahead, *behind, *author)
                })
                .collect(),
            selected: 0,
            dialog: DialogState::new(),
            ask: None,
            text: String::new(),
            focus: Some(BRANCH_LIST),
            key: "",
            note: String::from("Branches has focus"),
        }
    }

    fn press(&mut self, key: Key) {
        self.key = match key {
            Key::Down => "↓",
            Key::Up => "↑",
            Key::Delete => "d",
            Key::Rename => "r",
            Key::Tab => "Tab",
            Key::Enter => "Enter",
            Key::Char(_) => "type",
        };
        if self.dialog.is_open() {
            match key {
                Key::Tab => self.dialog.next(),
                Key::Enter => {
                    if let Some(closed) = self.dialog.activate() {
                        self.answer(closed.outcome(), closed.restore());
                    }
                }
                Key::Char(ch) => self.text.push(ch),
                _ => {}
            }
            return;
        }
        match key {
            Key::Down => self.selected = (self.selected + 1).min(self.branches.len() - 1),
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Delete if self.selected == 0 => self.ask(Ask::Protected, DialogKind::Alert),
            Key::Delete => {
                self.ask(Ask::Delete, DialogKind::Confirm);
                self.dialog.set_focus(DialogFocus::Cancel);
            }
            Key::Rename => {
                self.text = self.branches[self.selected].0.clone();
                self.ask(Ask::Rename, DialogKind::Prompt);
            }
            _ => {}
        }
    }

    fn ask(&mut self, ask: Ask, kind: DialogKind) {
        self.ask = Some(ask);
        self.dialog.open(kind, self.focus.take());
        self.note = String::from("Dialog has focus; Tab stays inside");
    }

    fn answer(&mut self, outcome: DialogOutcome, restore: Option<u32>) {
        let name = self.branches[self.selected].0.clone();
        self.note = match (self.ask.take(), outcome) {
            (Some(Ask::Delete), DialogOutcome::Confirmed) => {
                self.branches.remove(self.selected);
                self.selected = self.selected.min(self.branches.len() - 1);
                format!("Deleted {name}")
            }
            (Some(Ask::Rename), DialogOutcome::Confirmed) => {
                let renamed = self.text.clone();
                self.branches[self.selected].0.clone_from(&renamed);
                format!("Renamed to {renamed}")
            }
            (Some(Ask::Protected), _) => format!("Kept {name}"),
            _ => String::from("Cancelled"),
        };
        self.focus = restore;
        if self.focus == Some(BRANCH_LIST) {
            self.note.push_str(", focus back on Branches");
        }
    }

    fn dialog(&self) -> Option<Dialog<'_>> {
        let name = &self.branches[self.selected].0;
        let dialog = match self.ask? {
            Ask::Delete => Dialog::confirm(
                " Delete branch ",
                format!("Delete {name}? Its 3 commits that are not on main will be lost. This cannot be undone."),
            )
            .confirm_label("Delete branch")
            .destructive(true),
            Ask::Rename => Dialog::prompt(
                " Rename branch ",
                format!("New name for {name}. Open pull requests follow the rename."),
                self.text.as_str(),
                self.text.len(),
            )
            .placeholder("branch name")
            .confirm_label("Rename"),
            Ask::Protected => Dialog::alert(
                " Protected branch ",
                "main is protected and cannot be deleted. Change its rules under Settings > Branches first.",
            )
            .confirm_label("Got it"),
        };
        Some(dialog.width(60).state(&self.dialog))
    }
}

fn replay(tick: u32) -> App {
    let mut app = App::new();
    for (at, key) in SCRIPT {
        if tick >= at {
            app.press(key);
        }
    }
    app
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
    let app = replay(tick);
    let seconds = 51 + tick / 30;
    Paragraph::new(format!(
        " acme/web · branches{:>width$}",
        format!("12:{:02}:{:02} ", 4 + seconds / 60, seconds % 60),
        width = usize::from(area.width) - 20
    ))
    .style(raised.fg(accent).with_attrs(Attrs::new().set_bold(true)))
    .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let left = Rect::new(area.x + 1, area.y + 2, 54, area.height - 4);
    let right_x = left.x + left.width + 1;
    let right_width = area.x + area.width - right_x - 1;
    branches(&mut cx, &app, left, fill, theme);
    details(
        &mut cx,
        &app,
        Rect::new(right_x, left.y, right_width, 13),
        fill,
        theme,
        tick,
    );
    activity(
        &mut cx,
        Rect::new(right_x, left.y + 14, right_width, left.height - 14),
        fill,
        accent,
        tick,
    );
    let dim = raised.with_attrs(Attrs::new().set_dim(true));
    let key = if app.key.is_empty() {
        String::new()
    } else {
        format!(" key {} ", app.key)
    };
    StatusBar::new("")
        .style(raised)
        .left([
            (key, raised.fg(theme.resolve(Token::FocusRing))),
            (format!(" {} ", app.note), raised.fg(accent)),
        ])
        .right([(
            String::from(" d delete  r rename  Tab next  Enter choose  Esc cancel "),
            dim,
        )])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
    if let Some(dialog) = app.dialog() {
        dialog.render(area, &mut cx);
    }
}

fn branches(cx: &mut RenderCx<'_>, app: &App, pane: Rect, fill: Style, theme: &Theme) {
    let focused = app.focus == Some(BRANCH_LIST);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let border = if focused {
        fill.fg(theme.resolve(Token::FocusRing))
    } else {
        dim
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Branches ")
        .style(fill)
        .border_style(border)
        .title_style(border.with_attrs(Attrs::new().set_bold(focused)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    Paragraph::new(format!("  {:<22}{:>8}{:>8}", "Name", "ahead", "behind"))
        .style(dim)
        .render(Rect::new(inner.x, inner.y, inner.width, 1), cx);
    for (row, (name, _, ahead, behind, _)) in (0_u16..).zip(&app.branches) {
        let y = inner.y + 2 + row * 2;
        if y >= inner.y + inner.height {
            break;
        }
        let selected = usize::from(row) == app.selected;
        let style = if selected {
            Style::new()
                .fg(theme.resolve(Token::Surface0))
                .bg(theme.resolve(Token::FocusRing))
                .with_attrs(Attrs::new().set_bold(true))
        } else {
            fill
        };
        Paragraph::new(format!(
            "{} {name:<22}{:>8}{:>8}",
            if selected { "▸" } else { " " },
            format!("+{ahead}"),
            format!("-{behind}")
        ))
        .style(style)
        .render(Rect::new(inner.x, y, inner.width, 1), cx);
    }
}

fn details(cx: &mut RenderCx<'_>, app: &App, pane: Rect, fill: Style, theme: &Theme, tick: u32) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let (name, commit, ahead, _, author) = &app.branches[app.selected];
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {name} "))
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let line = |cx: &mut RenderCx<'_>, row: u16, text: String, style: Style| {
        Paragraph::new(text).style(style).render(
            Rect::new(inner.x + 1, inner.y + row, inner.width - 2, 1),
            cx,
        );
    };
    line(cx, 1, format!("Last commit   {commit}"), fill);
    line(cx, 2, format!("Author        {author}"), dim);
    line(cx, 3, format!("Ahead of main {ahead} commits"), dim);
    let checks = [
        ("build", 0_u32, 120_u32),
        ("unit tests", 30, 140),
        ("e2e tests", 70, 170),
    ];
    for (row, (check, start, span)) in (5_u16..).zip(checks) {
        let done = tick.saturating_sub(start).min(span);
        let percent = done * 100 / span;
        let finished = done == span;
        let tone: Color = if finished {
            theme.resolve(Token::Ok)
        } else {
            theme.resolve(Token::Warn)
        };
        line(cx, row, format!("{check:<14}"), fill);
        LineGauge::new(f32::from(u8::try_from(percent).unwrap_or(100)) / 100.0)
            .filled_style(fill.fg(tone))
            .unfilled_style(fill.with_attrs(Attrs::new().set_dim(true)))
            .render(Rect::new(inner.x + 15, inner.y + row, 30, 1), cx);
        Paragraph::new(if finished {
            String::from("passed")
        } else {
            format!("{percent:>3}%")
        })
        .style(fill.fg(tone))
        .render(Rect::new(inner.x + 47, inner.y + row, 8, 1), cx);
    }
}

fn activity(cx: &mut RenderCx<'_>, pane: Rect, fill: Style, accent: Color, tick: u32) {
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Activity ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(accent));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let arrived = usize::try_from(3 + tick / 28).unwrap_or(ACTIVITY.len());
    let shown = arrived.min(ACTIVITY.len());
    let rows = usize::from(inner.height);
    for (row, (index, entry)) in (0_u16..).zip(
        ACTIVITY
            .iter()
            .enumerate()
            .take(shown)
            .skip(shown.saturating_sub(rows)),
    ) {
        let seconds = 180 + index * 9;
        let style = if index + 1 == shown {
            fill.fg(accent)
        } else {
            fill
        };
        Paragraph::new(format!("12:{:02}:{:02}", seconds / 60, seconds % 60))
            .style(dim)
            .render(Rect::new(inner.x + 1, inner.y + row, 9, 1), cx);
        Paragraph::new(*entry).style(style).render(
            Rect::new(inner.x + 11, inner.y + row, inner.width - 12, 1),
            cx,
        );
    }
}
