// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::env;
use std::io::{self, IsTerminal};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::keymap::{Command, KeyChord, Keymap, render_overlay, render_palette};
use lunatui_runtime::{App, ControlFlow, RunOptions, render_once, run};
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers};

const OVERLAY: &str = "overlay.toggle";
const PALETTE: &str = "palette.open";
const QUIT: &str = "app.quit";

const OVERLAY_CHORDS: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('/'), Modifiers::new().control().shift()),
    KeyChord::new(KeyCode::F(1), Modifiers::new()),
];
const PALETTE_CHORDS: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('p'),
    Modifiers::new().control().shift(),
)];
const QUIT_CHORDS: &[KeyChord] = &[KeyChord::new(KeyCode::Char('q'), Modifiers::new())];

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

fn keymap() -> Keymap {
    let mut keymap = Keymap::new();
    keymap.register(Command::new(OVERLAY, "Show shortcuts", OVERLAY_CHORDS));
    keymap.register(Command::new(
        PALETTE,
        "Open command palette",
        PALETTE_CHORDS,
    ));
    keymap.register(Command::new(QUIT, "Quit", QUIT_CHORDS));
    keymap
}

struct Demo {
    keymap: Keymap,
    overlay: bool,
    palette: bool,
    query: String,
    selected: usize,
}

impl Demo {
    fn new() -> Self {
        Self {
            keymap: keymap(),
            overlay: false,
            palette: false,
            query: String::new(),
            selected: 0,
        }
    }

    fn run_command(&mut self, id: &str) -> ControlFlow {
        match id {
            OVERLAY => self.overlay = !self.overlay,
            PALETTE => {
                self.palette = true;
                self.query.clear();
                self.selected = 0;
            }
            QUIT => return ControlFlow::Break,
            _ => {}
        }
        ControlFlow::Continue
    }

    fn update_palette(&mut self, key: &KeyEvent) -> ControlFlow {
        match key.code {
            KeyCode::Esc => self.palette = false,
            KeyCode::Enter => {
                let id = self
                    .keymap
                    .palette(&self.query)
                    .get(self.selected)
                    .map(|command| command.id());
                self.palette = false;
                if let Some(id) = id {
                    return self.run_command(id);
                }
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                let last = self.keymap.palette(&self.query).len().saturating_sub(1);
                self.selected = (self.selected + 1).min(last);
            }
            KeyCode::Char(c) if key.modifiers == Modifiers::new() => {
                self.query.push(c);
                self.selected = 0;
            }
            _ => {}
        }
        ControlFlow::Continue
    }
}

impl App for Demo {
    fn update(&mut self, event: &Event) -> ControlFlow {
        let Event::Key(key) = event else {
            return ControlFlow::Continue;
        };
        if key.kind != KeyKind::Press {
            return ControlFlow::Continue;
        }
        if self.palette {
            return self.update_palette(key);
        }
        match self.keymap.dispatch(event) {
            Some(id) => self.run_command(id),
            None => ControlFlow::Continue,
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        if self.palette {
            render_palette(
                &self.keymap,
                &self.query,
                self.selected,
                grid,
                area,
                clusters,
            );
        } else if self.overlay {
            render_overlay(&self.keymap, grid, area, clusters);
        } else {
            let style = Style::new();
            grid.fill(area, style, ' ', clusters);
            grid.put_str(area.x, area.y, style, "Press F1 for shortcuts", clusters);
        }
    }
}

fn print_snapshot() {
    let mut app = Demo::new();
    app.overlay = true;
    let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
    println!("{}", grid.display_text(&clusters));

    let mut app = Demo::new();
    app.palette = true;
    app.query = String::from("pal");
    let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
    println!("{}", grid.display_text(&clusters));
}

fn main() -> io::Result<()> {
    let snapshot = env::args().skip(1).any(|arg| arg == "--snapshot");
    if snapshot || !io::stdout().is_terminal() {
        print_snapshot();
        return Ok(());
    }
    run(Demo::new(), RunOptions::new(WIDTH, HEIGHT))
}
