// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("README.md")]

use std::io::{self, Write};

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{
    App, ChangeContext, ChangeDetails, ChangeSource, ControlFlow, FocusManager, Handle, HitId,
    LayerStack, RunOptions, render_once, run,
};
use lunatui_term::{ColorMode, Encoder, Event, KeyCode, KeyEvent, MouseButton, MouseKind};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Block, Dialog, DialogKind, DialogState, Paragraph, RenderCx, TextArea, Widget, WidgetState,
};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 28;
const EDITOR: HitId = HitId(1);
const DIALOG: HitId = HitId(2);
const DRAFT: &str = "Release notes\n\nKeep the draft until its author approves a replacement.\nEvery change has a source. The application decides what to accept.";
const INCOMING: &str = "Release notes\n\nThe reviewed replacement is ready.\nAccepted with permission. Continue editing the reviewed draft.";

struct GuardedEditor {
    editor: TextArea,
    layers: LayerStack,
    focus: FocusManager,
    dialog: DialogState,
    protected: bool,
    last: Option<ChangeDetails>,
    theme: Theme,
}

impl GuardedEditor {
    fn new() -> Self {
        let mut editor = TextArea::new(DRAFT).line_numbers(true);
        editor.set_widget_state(WidgetState::Focused);
        let mut focus = FocusManager::new();
        focus.sync(vec![(EDITOR, Rect::new(2, 5, 96, 16))]);
        focus.focus(EDITOR);
        let mut layers = LayerStack::new();
        let mut dialog = DialogState::new();
        dialog.open(DialogKind::Confirm, &mut layers, Some(EDITOR));
        Self {
            editor,
            layers,
            focus,
            dialog,
            protected: true,
            last: None,
            theme: Theme::builtin(Variant::Dark),
        }
    }

    fn message(&self) -> &'static str {
        match self.last {
            Some(details) if details.source() == ChangeSource::Agent && details.cancel => {
                "External edit refused. Your draft and undo history are unchanged."
            }
            Some(details) if details.source() == ChangeSource::Agent => {
                "External edit accepted. The reviewed replacement is in the editor."
            }
            Some(details) if details.cancel => {
                "Close refused. The dialog stays open and keeps keyboard focus."
            }
            Some(_) => "Change accepted. Continue editing your draft.",
            None => "Your draft is protected. Try closing the dialog or replacing the text.",
        }
    }

    fn modal(&self) -> Dialog<'static> {
        Dialog::confirm(
            "Keep this draft?",
            "The application can refuse a close before focus or text changes.",
        )
        .confirm_label("Keep editing")
        .cancel_label("Close")
        .width(68)
        .state(&self.dialog)
    }
}

impl App for GuardedEditor {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::F(10) => return ControlFlow::Break,
                KeyCode::F(2) => {
                    self.protected = !self.protected;
                    return ControlFlow::Continue;
                }
                KeyCode::F(3) => {
                    let protected = self.protected;
                    let result = self.editor.set_text_with(
                        INCOMING,
                        ChangeContext::new(ChangeSource::Agent)
                            .guard(&mut |d| d.cancel = protected),
                    );
                    self.last = result.details().copied();
                    return ControlFlow::Continue;
                }
                _ => {}
            }
        }
        let protected = self.protected;
        let mut policy = |details: &mut ChangeDetails| {
            details.cancel = protected;
        };
        let receipt = self.layers.handle_with(
            event,
            &mut self.focus,
            ChangeContext::for_event(event).guard(&mut policy),
        );
        if let Some(details) = receipt.details() {
            self.last = Some(*details);
        }
        self.dialog.sync(&self.layers, &receipt);
        if !receipt.delivers() {
            return ControlFlow::Continue;
        }
        if self.dialog.is_open() {
            match event {
                Event::Key(key) if key.code == KeyCode::Tab => self.dialog.next(),
                Event::Key(key) if key.code == KeyCode::Enter => {
                    let result = self.dialog.activate_with(
                        &mut self.layers,
                        &mut self.focus,
                        ChangeContext::for_event(event).guard(&mut policy),
                    );
                    self.last = result.details().copied();
                }
                Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                    let target = self.modal().control_at(
                        Rect::new(0, 0, WIDTH, HEIGHT),
                        mouse.column,
                        mouse.row,
                    );
                    let result = self.dialog.click_with(
                        target,
                        &mut self.layers,
                        &mut self.focus,
                        ChangeContext::for_event(event).guard(&mut policy),
                    );
                    if let Some(details) = result.details() {
                        self.last = Some(*details);
                    }
                }
                _ => {}
            }
        } else {
            self.editor.handle(event);
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = &self.theme;
        let mut cx = RenderCx::new(grid, clusters, theme);
        let text = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(cx.token(Token::Surface0));
        Block::bordered()
            .title(" Draft review ")
            .style(text)
            .border_style(accent)
            .render(area, &mut cx);
        Paragraph::new("A change needs your permission")
            .style(accent)
            .render(Rect::new(3, 2, 94, 1), &mut cx);
        let protection = if self.protected {
            "PROTECTED  |  Close and external edits need approval"
        } else {
            "APPROVED   |  Close and external edits are allowed"
        };
        Paragraph::new(protection)
            .style(text)
            .render(Rect::new(3, 3, 94, 1), &mut cx);
        let frame = Rect::new(2, 5, 96, 16);
        let block = Block::bordered()
            .title(" release-notes.txt ")
            .style(text)
            .border_style(accent);
        block.render(frame, &mut cx);
        self.editor.render(block.inner(frame), &mut cx);
        self.layers.clear_focusable();
        if self.dialog.is_open() {
            let widget = self.modal();
            let panel = widget.area(area);
            widget.render(area, &mut cx);
            self.dialog.place(&mut self.layers, panel);
            if let Some(layer) = self.dialog.layer() {
                self.layers.assign(layer, DIALOG, panel);
            }
        }
        self.focus
            .sync(self.layers.focus_entries(vec![(EDITOR, frame)]));
        Paragraph::new(self.message())
            .style(accent)
            .render(Rect::new(3, 23, 94, 1), &mut cx);
        Paragraph::new("Esc Close dialog    F2 Toggle permission    F3 External edit    F10 Exit")
            .style(text)
            .render(Rect::new(3, 25, 94, 1), &mut cx);
    }
}

fn scripted_events(tick: u32) -> Vec<Event> {
    let code = match tick {
        30 | 185 => Some(KeyCode::Esc),
        90 | 280 => Some(KeyCode::F(3)),
        140 => Some(KeyCode::F(2)),
        205 => Some(KeyCode::End),
        _ => None,
    };
    if let Some(code) = code {
        return vec![Event::Key(KeyEvent::press(code))];
    }
    if tick >= 210
        && (tick - 210).is_multiple_of(2)
        && let Some(ch) = " - ready for review"
            .chars()
            .nth(((tick - 210) / 2) as usize)
    {
        return vec![Event::Key(KeyEvent::press(KeyCode::Char(ch)))];
    }
    Vec::new()
}

/// Runs the guarded editor or replays its deterministic input sequence.
///
/// # Errors
/// Returns terminal or output errors.
pub fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let mut app = GuardedEditor::new();
    if !args.iter().any(|arg| arg == "--snapshot") {
        return run(app, RunOptions::new(WIDTH, HEIGHT));
    }
    let tick = args
        .windows(2)
        .find(|pair| pair[0] == "--tick")
        .and_then(|pair| pair[1].parse::<u32>().ok())
        .unwrap_or(0);
    let _ = render_once(&mut app, WIDTH, HEIGHT);
    for step in 0..=tick {
        for event in scripted_events(step) {
            app.update(&event);
            let _ = render_once(&mut app, WIDTH, HEIGHT);
        }
    }
    if args.iter().any(|arg| arg == "--document") {
        print!("{}", app.editor.text());
    } else if args.iter().any(|arg| arg == "--state") {
        println!(
            "protected={}\ndialog_open={}\nfocus={:?}\nlast={:?}",
            app.protected,
            app.dialog.is_open(),
            app.focus.focused(),
            app.last
        );
    } else {
        let (grid, clusters) = render_once(&mut app, WIDTH, HEIGHT);
        if args.iter().any(|arg| arg == "--ansi") {
            let mut encoder = Encoder::new();
            io::stdout().write_all(encoder.encode_diff(
                &Grid::new(WIDTH, HEIGHT),
                &grid,
                &clusters,
                ColorMode::TrueColor,
            ))?;
        } else {
            print!("{}", grid.display_text(&clusters));
        }
    }
    Ok(())
}
