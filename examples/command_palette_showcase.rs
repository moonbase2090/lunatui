// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_runtime::keymap::{Command, KeyChord};
use lunatui_term::{KeyCode, Modifiers};
use lunatui_theme::{Theme, Token};
use lunatui_widgets::{
    Block, BorderType, CommandPalette, PaletteEntry, Paragraph, RenderCx, SearchBox, Widget,
};

const OPEN: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('o'),
    Modifiers::new().control(),
)];
const SAVE: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('s'),
    Modifiers::new().control(),
)];
const COMMANDS: [Command; 8] = [
    Command::new("open", "Open file", OPEN),
    Command::new("save", "Save file", SAVE),
    Command::new("split", "Split pane right", &[]),
    Command::new("preview", "Show preview", &[]),
    Command::new("sidebar", "Toggle sidebar", &[]),
    Command::new("search", "Search workspace", &[]),
    Command::new("theme", "Change color theme", &[]),
    Command::new("terminal", "Open terminal", &[]),
];
const DETAILS: [&str; 8] = [
    "workspace",
    "write to disk",
    "editor layout",
    "render markdown",
    "navigation",
    "find in files",
    "appearance",
    "shell session",
];

#[derive(Clone, Copy)]
enum Key {
    Open,
    Char(char),
    Down,
    Up,
    Enter,
    Escape,
}
const SCRIPT: [(u32, Key); 14] = [
    (25, Key::Char('o')),
    (40, Key::Char('f')),
    (80, Key::Enter),
    (105, Key::Open),
    (130, Key::Char('s')),
    (145, Key::Char('p')),
    (170, Key::Down),
    (185, Key::Up),
    (205, Key::Enter),
    (230, Key::Open),
    (245, Key::Char('z')),
    (255, Key::Char('z')),
    (275, Key::Escape),
    (299, Key::Escape),
];

struct App {
    open: bool,
    query: String,
    selected: Option<usize>,
    file: &'static str,
    split: bool,
    note: &'static str,
}
impl App {
    fn new() -> Self {
        Self {
            open: true,
            query: String::new(),
            selected: Some(0),
            file: "src/main.rs",
            split: false,
            note: "Palette has focus · type a command",
        }
    }
    fn press(&mut self, key: Key) {
        if let Key::Open = key {
            self.open = true;
            self.query.clear();
            self.selected = Some(0);
            self.note = "Palette has focus · type a command";
            return;
        }
        if !self.open {
            return;
        }
        match key {
            Key::Char(ch) => {
                self.query.push(ch);
                self.selected = Some(0);
            }
            Key::Down | Key::Up => {
                let entries: Vec<_> = COMMANDS
                    .iter()
                    .map(|command| PaletteEntry::new(command.title()))
                    .collect();
                let count = CommandPalette::new(self.query.as_str(), self.query.len(), &entries)
                    .matches()
                    .len();
                self.selected = SearchBox::step(self.selected, count, matches!(key, Key::Down));
            }
            Key::Enter => {
                let entries: Vec<_> = COMMANDS
                    .iter()
                    .map(|command| PaletteEntry::new(command.title()))
                    .collect();
                let hits =
                    CommandPalette::new(self.query.as_str(), self.query.len(), &entries).matches();
                if let Some(hit) = self.selected.and_then(|at| hits.get(at)) {
                    match COMMANDS[hit.index].id() {
                        "open" => {
                            self.file = "src/session.rs";
                            self.note = "Opened src/session.rs · focus back on editor";
                        }
                        "split" => {
                            self.split = true;
                            self.note = "Split editor right · two files side by side";
                        }
                        _ => self.note = "Command completed · focus back on editor",
                    }
                    self.open = false;
                }
            }
            Key::Escape => {
                self.open = false;
                self.note = "Cancelled palette · focus back on editor";
            }
            Key::Open => {}
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
    if area.width < 40 || area.height < 12 {
        return;
    }
    let mut app = App::new();
    for (at, key) in SCRIPT {
        if tick >= at {
            app.press(key);
        }
    }
    let mut cx = RenderCx::new(grid, clusters, theme);
    let base = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0));
    let accent = base
        .fg(cx.token(Token::Accent))
        .with_attrs(Attrs::new().set_bold(true));
    let dim = base.fg(cx.token(Token::TextMuted));
    let blank = (0..area.height)
        .map(|_| " ".repeat(usize::from(area.width)))
        .collect::<Vec<_>>()
        .join("\n");
    Paragraph::new(blank).style(base).render(area, &mut cx);
    Paragraph::new("LUNA EDITOR   /   workspace: orbit")
        .style(accent)
        .render(
            Rect::new(area.x + 2, area.y + 1, area.width - 4, 1),
            &mut cx,
        );
    Paragraph::new("main   ·   Rust   ·   2 modified files                                      Ctrl+P  Commands")
        .style(dim).render(Rect::new(area.x+2,area.y+2,area.width-4,1),&mut cx);
    palette_workspace(&mut cx, area, &app);
    Paragraph::new(format!(
        "CHECKS   {} / 24 passed   ·   language server ready   ·   no errors",
        20 + (tick / 30).min(4)
    ))
    .style(base.fg(cx.token(Token::Ok)))
    .render(
        Rect::new(area.x + 2, area.y + area.height - 3, area.width - 4, 1),
        &mut cx,
    );
    Paragraph::new(app.note).style(accent).render(
        Rect::new(area.x + 2, area.y + area.height - 2, area.width - 4, 1),
        &mut cx,
    );
    if app.open {
        let hints: Vec<String> = COMMANDS
            .iter()
            .map(|command| {
                command
                    .chords()
                    .first()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            })
            .collect();
        let entries: Vec<_> = COMMANDS
            .iter()
            .zip(&hints)
            .zip(DETAILS)
            .map(|((command, hint), detail)| {
                PaletteEntry::new(command.title())
                    .detail(detail)
                    .shortcut(hint)
            })
            .collect();
        CommandPalette::new(app.query.as_str(), app.query.len(), &entries)
            .selected(app.selected)
            .title(" Run command ")
            .width(68)
            .max_rows(5)
            .render(area, &mut cx);
    }
}

fn palette_workspace(cx: &mut RenderCx<'_>, area: Rect, app: &App) {
    let base = Style::new()
        .fg(cx.token(Token::TextPrimary))
        .bg(cx.token(Token::Surface0));
    let dim = base.fg(cx.token(Token::TextMuted));
    let files = Rect::new(area.x + 1, area.y + 4, 23, area.height - 8);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Explorer ")
        .border_style(dim)
        .render(files, cx);
    Paragraph::new("orbit\n  Cargo.toml\n  README.md\n  src/\n    main.rs\n    session.rs\n    render.rs\n    commands.rs\n  tests/\n    session_test.rs\n\n  8 files · Rust workspace")
        .style(dim).render(Rect::new(files.x+2,files.y+2,files.width-4,files.height-3),cx);
    let editor = Rect::new(area.x + 25, area.y + 4, area.width - 26, area.height - 8);
    let pane_width = if app.split {
        editor.width / 2
    } else {
        editor.width
    };
    let pane = Rect::new(editor.x, editor.y, pane_width, editor.height);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", app.file))
        .border_style(dim)
        .render(pane, cx);
    let code = if app.file == "src/main.rs" {
        "use orbit::Session;\n\nfn main() -> Result<()> {\n    let mut session = Session::new();\n    session.open_workspace(\"orbit\")?;\n    session.run()\n}"
    } else {
        "pub struct Session {\n    files: Vec<Document>,\n    active: usize,\n}\n\nimpl Session {\n    pub fn open(&mut self, path: &str) {\n        let document = Document::read(path);\n        self.files.push(document);\n        self.active = self.files.len() - 1;\n    }\n}"
    };
    Paragraph::new(code).style(base).render(
        Rect::new(
            pane.x + 2,
            pane.y + 2,
            pane.width.saturating_sub(4),
            pane.height - 3,
        ),
        cx,
    );
    if app.split {
        let right = Rect::new(
            editor.x + pane_width,
            editor.y,
            editor.width - pane_width,
            editor.height,
        );
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" tests/session_test.rs ")
            .border_style(dim)
            .render(right, cx);
        Paragraph::new("#[test]\nfn opens_the_selected_file() {\n    let mut app = Session::new();\n    app.open(\"src/session.rs\");\n    assert_eq!(app.active(), 0);\n}\n\n// Saved workspace layout")
            .style(base).render(Rect::new(right.x+2,right.y+2,right.width.saturating_sub(4),right.height-3),cx);
    }
}
