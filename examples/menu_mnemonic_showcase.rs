// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, Menu, MenuEvent, MenuItem, MenuState, Paragraph, RenderCx, StatusBar, Widget,
};

const DRAFT: [&str; 9] = [
    "# Release notes 2.4",
    "",
    "## Highlights",
    "",
    "- Passkey sign-in on web and mobile",
    "- Search ranks recent visits first",
    "",
    "## Fixes",
    "- Cache misses no longer storm after deploys",
];

const NOTES: [&str; 12] = [
    "# Team notes, week 44",
    "",
    "## Decisions",
    "- Ship 2.4 on Thursday after the e2e run",
    "- Keep the old cache layer behind a flag for a week",
    "",
    "## Follow-ups",
    "- Kofi: rotate the staging certificates",
    "- Mia: rewrite the first-day checklist",
    "- Lee: profile the sign-in screen on slow phones",
    "",
    "## Parking lot",
];

const PASTED: &str = "- Iris: write the 2.4 migration guide";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    New,
    Open,
    Recent,
    Copy,
    Cut,
    Paste,
    Wrap,
    Numbers,
    About,
}

#[derive(Clone, Copy)]
enum Step {
    Mnemonic(u32, &'static str, char),
    Click(u32, usize),
}

const STEPS: [Step; 8] = [
    Step::Mnemonic(20, "Alt+F", 'f'),
    Step::Mnemonic(45, "R", 'r'),
    Step::Mnemonic(70, "N", 'n'),
    Step::Mnemonic(95, "Alt+E", 'e'),
    Step::Mnemonic(115, "C", 'c'),
    Step::Mnemonic(135, "C", 'c'),
    Step::Mnemonic(155, "P", 'p'),
    Step::Click(182, 2),
];

const WRAP_AT: u32 = 228;

fn menus() -> MenuState<Action> {
    MenuState::new(vec![
        MenuItem::group(
            "File",
            vec![
                MenuItem::item("New", Action::New).mnemonic('n'),
                MenuItem::item("Open…", Action::Open).mnemonic('o'),
                MenuItem::group(
                    "Recent",
                    vec![
                        MenuItem::item("notes.md", Action::Recent).mnemonic('n'),
                        MenuItem::item("release.md", Action::Open).mnemonic('r'),
                    ],
                )
                .mnemonic('r'),
                MenuItem::item("Print", Action::Open)
                    .mnemonic('p')
                    .disabled(true),
            ],
        )
        .mnemonic('f'),
        MenuItem::group(
            "Edit",
            vec![
                MenuItem::item("Copy", Action::Copy).mnemonic('c'),
                MenuItem::item("Cut", Action::Cut).mnemonic('c'),
                MenuItem::item("Paste", Action::Paste).mnemonic('p'),
            ],
        )
        .mnemonic('e'),
        MenuItem::group(
            "View",
            vec![
                MenuItem::item("Word wrap", Action::Wrap).mnemonic('w'),
                MenuItem::item("Line numbers", Action::Numbers).mnemonic('l'),
            ],
        )
        .mnemonic('v'),
        MenuItem::group(
            "Help",
            vec![MenuItem::item("About", Action::About).mnemonic('a')],
        )
        .mnemonic('h'),
    ])
}

struct App {
    menu: MenuState<Action>,
    file: &'static str,
    pasted: bool,
    wrap: bool,
    key: &'static str,
    note: &'static str,
    pointer: Option<(usize, u32)>,
}

impl App {
    fn handle(&mut self) {
        let events: Vec<_> = self.menu.drain_events().collect();
        for MenuEvent::Selected(action) in events {
            self.menu.reset();
            match action {
                Action::Recent => {
                    self.file = "notes.md";
                    self.note = "Opened notes.md from File > Recent";
                }
                Action::Paste => {
                    self.pasted = true;
                    self.note = "Pasted one line";
                }
                Action::Wrap => {
                    self.wrap = true;
                    self.note = "Word wrap on";
                }
                _ => {}
            }
        }
    }
}

fn replay(tick: u32) -> App {
    let mut app = App {
        menu: menus(),
        file: "release.md",
        pasted: false,
        wrap: false,
        key: "",
        note: "Alt plus an underlined letter opens a menu",
        pointer: None,
    };
    for step in STEPS {
        match step {
            Step::Mnemonic(at, key, letter) if tick >= at => {
                app.key = key;
                app.menu.press_mnemonic(letter);
                if app.menu.is_active() {
                    app.note = match letter {
                        'c' => "C is on Copy and Cut, so each press moves between them",
                        'f' | 'e' => "The underlined letters choose an item",
                        _ => app.note,
                    };
                }
                app.handle();
            }
            Step::Click(at, index) if tick + 25 >= at => {
                app.pointer = Some((index, at));
                if tick >= at {
                    app.key = "click";
                    app.menu.open(index);
                    app.note = "A click on the bar opens View";
                }
            }
            _ => {}
        }
    }
    if tick >= WRAP_AT {
        app.key = "W";
        app.menu.press_mnemonic('w');
        app.handle();
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
    let raised = Style::new()
        .fg(text)
        .bg(theme.resolve(Token::SurfaceRaised));
    let accent = theme.resolve(Token::Accent);
    *grid = Grid::new(grid.width(), grid.height());
    let mut cx = RenderCx::new(grid, clusters, theme);
    Block::new().style(fill).render(area, &mut cx);
    if area.width < 100 || area.height < 30 {
        return;
    }
    let app = replay(tick);
    Block::new()
        .style(raised)
        .render(Rect::new(area.x, area.y, area.width, 1), &mut cx);
    let dim = fill.with_attrs(Attrs::new().set_dim(true));
    let body = Rect::new(area.x + 1, area.y + 2, area.width - 2, area.height - 4);
    let outline = Rect::new(body.x + body.width - 30, body.y, 30, body.height);
    let editor = Rect::new(body.x, body.y, body.width - 31, body.height);
    editor_pane(&mut cx, &app, editor, fill, dim, theme);
    outline_pane(&mut cx, &app, outline, fill, dim, theme);
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
            (format!(" {} ", app.note), raised.fg(accent)),
        ])
        .right([(
            format!(
                " {}  wrap {} ",
                app.file,
                if app.wrap { "on" } else { "off" }
            ),
            raised.with_attrs(Attrs::new().set_dim(true)),
        )])
        .render(
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            &mut cx,
        );
    let menu = Menu::new(&app.menu).dropdown_width(22);
    menu.render(
        Rect::new(area.x, area.y, area.width, area.height - 1),
        &mut cx,
    );
    if let Some((index, arrive)) = app.pointer.filter(|_| tick < WRAP_AT + 10) {
        let target = (area.x + 15, area.y);
        let start = (area.x + 40, area.y + 14);
        let spent = (tick + 25).saturating_sub(arrive).min(25);
        let along = |from: u16, to: u16| {
            let from = i32::from(from);
            let to = i32::from(to);
            u16::try_from(from + (to - from) * i32::try_from(spent).unwrap_or(25) / 25).unwrap_or(0)
        };
        let at = (along(start.0, target.0), along(start.1, target.1));
        let over = if spent >= 25 && menu.bar_item_at(area, at.0, at.1) == Some(index) {
            theme.resolve(Token::SurfaceRaised)
        } else {
            fill.bg
        };
        Paragraph::new(if tick >= arrive && tick < arrive + 4 {
            "●"
        } else {
            "↖"
        })
        .style(
            Style::new()
                .fg(text)
                .bg(over)
                .with_attrs(Attrs::new().set_bold(true)),
        )
        .render(Rect::new(at.0, at.1, 1, 1), &mut cx);
    }
}

fn editor_pane(
    cx: &mut RenderCx<'_>,
    app: &App,
    pane: Rect,
    fill: Style,
    dim: Style,
    theme: &Theme,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", app.file))
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let lines: &[&str] = if app.file == "notes.md" {
        &NOTES
    } else {
        &DRAFT
    };
    let pasted = app.pasted.then_some(PASTED);
    for (row, line) in (0_u16..).zip(lines.iter().copied().chain(pasted)) {
        if row >= inner.height {
            break;
        }
        let style = if Some(line) == pasted {
            fill.fg(theme.resolve(Token::Ok))
        } else if line.starts_with('#') {
            fill.fg(theme.resolve(Token::Accent))
                .with_attrs(Attrs::new().set_bold(true))
        } else {
            fill
        };
        Paragraph::new(format!("{:>3}", row + 1))
            .style(dim)
            .render(Rect::new(inner.x, inner.y + row, 3, 1), cx);
        Paragraph::new(line).style(style).render(
            Rect::new(inner.x + 5, inner.y + row, inner.width - 6, 1),
            cx,
        );
    }
}

fn outline_pane(
    cx: &mut RenderCx<'_>,
    app: &App,
    pane: Rect,
    fill: Style,
    dim: Style,
    theme: &Theme,
) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Outline ")
        .style(fill)
        .border_style(dim)
        .title_style(fill.fg(theme.resolve(Token::Accent)));
    block.render(pane, cx);
    let inner = block.inner(pane);
    let lines: &[&str] = if app.file == "notes.md" {
        &NOTES
    } else {
        &DRAFT
    };
    for (row, heading) in (0_u16..).zip(lines.iter().filter(|line| line.starts_with('#'))) {
        let depth = heading.chars().take_while(|ch| *ch == '#').count();
        Paragraph::new(format!(
            "{}{}",
            "  ".repeat(depth.saturating_sub(1)),
            heading.trim_start_matches('#').trim()
        ))
        .style(if depth == 1 { fill } else { dim })
        .render(
            Rect::new(inner.x + 1, inner.y + row, inner.width - 2, 1),
            cx,
        );
    }
}
