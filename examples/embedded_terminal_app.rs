// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use lunatui_core::{Attrs, ClusterTable, Grid, Rect, Style};
use lunatui_runtime::{App, ControlFlow, RunOptions};
use lunatui_term::{
    ColorMode, Encoder, Event, KeyCode, KeyEvent, KeyKind, Modifiers, PtyProcess, PtySize,
};
use lunatui_theme::{PreparedPalette, Profile, Theme, Token, Variant};
use lunatui_widgets::{
    Block, BorderType, Paragraph, PseudoTerminal, RenderCx, TerminalScreen, Widget, WidgetState,
};

struct Options {
    width: u16,
    height: u16,
    frames: u32,
    fps: u32,
    scripted: bool,
    workspace: PathBuf,
    variant: Variant,
    profile: Profile,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

impl Options {
    fn parse() -> io::Result<Self> {
        let mut out = Self {
            width: 120,
            height: 36,
            frames: 300,
            fps: 30,
            scripted: false,
            workspace: std::env::current_dir()?,
            variant: Variant::Dark,
            profile: Profile::TrueColor,
        };
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            if flag == "--scripted" {
                out.scripted = true;
                continue;
            }
            let value = args
                .next()
                .ok_or_else(|| invalid("an option is missing its value"))?;
            match flag.as_str() {
                "--width" => out.width = value.parse().map_err(|_| invalid("invalid width"))?,
                "--height" => out.height = value.parse().map_err(|_| invalid("invalid height"))?,
                "--frames" => {
                    out.frames = value.parse().map_err(|_| invalid("invalid frame count"))?;
                }
                "--fps" => out.fps = value.parse().map_err(|_| invalid("invalid frame rate"))?,
                "--workspace" => out.workspace = value.into(),
                "--theme" => {
                    out.variant = match value.as_str() {
                        "dark" => Variant::Dark,
                        "light" => Variant::Light,
                        _ => return Err(invalid("unknown theme")),
                    }
                }
                "--profile" => {
                    out.profile = match value.as_str() {
                        "truecolor" => Profile::TrueColor,
                        "ansi256" => Profile::Ansi256,
                        "ansi16" => Profile::Ansi16,
                        _ => return Err(invalid("unknown color profile")),
                    }
                }
                _ => return Err(invalid("unknown option")),
            }
        }
        if out.width < 60
            || out.height < 16
            || out.frames == 0
            || out.frames > 9000
            || out.fps == 0
            || out.fps > 240
        {
            return Err(invalid(
                "screen, frames or frame rate is outside the example limits",
            ));
        }
        PtySize::new(out.width, out.height)?;
        Ok(out)
    }
}

fn pane(area: Rect) -> Rect {
    let left = area.width / 3;
    Rect::new(
        area.x + left + 2,
        area.y + 4,
        area.width - left - 3,
        area.height - 7,
    )
}

fn inner(area: Rect) -> Rect {
    Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2)
}

struct Workspace {
    process: PtyProcess,
    screen: TerminalScreen,
    theme: Theme,
    palette: PreparedPalette,
    source: String,
    focused: bool,
    note: String,
    error: Option<io::Error>,
}

impl Workspace {
    fn new(options: &Options) -> io::Result<Self> {
        let area = Rect::new(0, 0, options.width, options.height);
        let content = inner(pane(area));
        let size = PtySize::new(content.width, content.height)?;
        let mut command = Command::new("/bin/sh");
        command.args(["-c","printf 'ORBIT WORKSPACE\\n'; test -t 0 && printf 'Controlling terminal attached\\n'; printf 'Terminal size: '; stty size; printf '\\nRun cargo test to check this workspace.\\n\\n'; exec /bin/sh -i"])
            .current_dir(&options.workspace).env("PS1","orbit % ").env("ENV","");
        if options.scripted {
            command.env_remove("CARGO_TARGET_DIR");
        }
        let source =
            fs::read_to_string(options.workspace.join("src/lib.rs")).unwrap_or_else(|_| {
                String::from(
                    "Open a Rust workspace with --workspace.\nThe shell runs in that directory.",
                )
            });
        let theme = Theme::builtin(options.variant);
        let palette = theme.prepare(options.profile);
        let mut app = Self {
            process: PtyProcess::spawn(command, size)?,
            screen: TerminalScreen::new(size),
            theme,
            palette,
            source,
            focused: true,
            note: "Terminal focused · Ctrl+] switches focus".into(),
            error: None,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.screen.contents().contains("orbit % ") {
            app.poll()?;
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "shell prompt did not appear",
                ));
            }
            thread::sleep(Duration::from_millis(2));
        }
        Ok(app)
    }

    fn state(&self) -> WidgetState {
        if self.focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        }
    }

    fn poll(&mut self) -> io::Result<()> {
        self.process.flush_input()?;
        let mut bytes = [0_u8; 4096];
        // Limit each frame's work even when the child continuously writes.
        for _ in 0..8 {
            match self.process.read(&mut bytes) {
                Ok(0) => break,
                Ok(count) => self.screen.feed(&bytes[..count]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        let replies = self.screen.take_replies();
        if !replies.is_empty() {
            self.process.queue_input(&replies)?;
        }
        self.process.flush_input()?;
        if let Some(status) = self.process.try_wait()? {
            self.note = format!("Shell exited: {status}");
        }
        Ok(())
    }

    fn scripted(&mut self, tick: u32) {
        for (start, text, enter) in [
            (20, "cargo test --color always", 74),
            (120, "cargo test --color always", 174),
            (225, "printf 'PTY ready\\n'", 275),
        ] {
            if tick >= start
                && (tick - start).is_multiple_of(2)
                && let Some(ch) = text
                    .chars()
                    .nth(usize::try_from((tick - start) / 2).unwrap_or(usize::MAX))
            {
                self.update(&Event::Key(KeyEvent::press(KeyCode::Char(ch))));
            }
            if tick == enter {
                self.update(&Event::Key(KeyEvent::press(KeyCode::Enter)));
            }
        }
        if tick == 200 || tick == 220 {
            self.update(&Event::Key(
                KeyEvent::press(KeyCode::Char(']')).modifiers(Modifiers::new().control()),
            ));
        }
        if tick == 205 {
            self.update(&Event::Key(KeyEvent::press(KeyCode::Char('x'))));
        }
    }
}

impl App for Workspace {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if matches!(event,Event::Key(key) if key.kind==KeyKind::Press && key.code==KeyCode::Char(']') && key.modifiers.is_control())
        {
            self.focused = !self.focused;
            self.note = if self.focused {
                "Terminal focused · input resumes"
            } else {
                "Editor focused · terminal input paused"
            }
            .into();
        } else if !self.focused
            && matches!(event,Event::Key(key) if key.kind==KeyKind::Press && key.code==KeyCode::Char('q') && key.modifiers==Modifiers::new())
        {
            return ControlFlow::Break;
        } else if let Some(bytes) = PseudoTerminal::new(&self.screen)
            .state(self.state())
            .input(event)
            && let Err(error) = self.process.queue_input(&bytes)
        {
            self.error = Some(error);
        }
        ControlFlow::Continue
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        if let Err(error) = self.poll() {
            self.error = Some(error);
        }
        let terminal = pane(area);
        let content = inner(terminal);
        if let Ok(size) = PtySize::new(content.width, content.height)
            && size != self.process.size()
        {
            match self.process.resize(size) {
                Ok(()) => self.screen.resize(size),
                Err(error) => self.error = Some(error),
            }
        }
        let mut cx = RenderCx::new(grid, clusters, &self.theme).palette(&self.palette);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::Surface0));
        let accent = base
            .fg(cx.token(Token::Accent))
            .with_attrs(Attrs::new().set_bold(true));
        let blank = (0..area.height)
            .map(|_| " ".repeat(usize::from(area.width)))
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(blank).style(base).render(area, &mut cx);
        Paragraph::new("ORBIT   /   Rust workspace")
            .style(accent)
            .render(Rect::new(2, 1, area.width - 4, 1), &mut cx);
        Paragraph::new("src/lib.rs   ·   workspace tests   ·   interactive shell")
            .style(base)
            .render(Rect::new(2, 2, area.width - 4, 1), &mut cx);
        let editor = Rect::new(1, 4, area.width / 3, area.height - 7);
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" src/lib.rs ")
            .style(base)
            .border_style(base.fg(cx.token(if self.focused {
                Token::Border
            } else {
                Token::FocusRing
            })))
            .render(editor, &mut cx);
        Paragraph::new(self.source.as_str())
            .style(base)
            .render(inner(editor), &mut cx);
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" terminal · /bin/sh ")
            .style(base)
            .border_style(base.fg(cx.token(if self.focused {
                Token::FocusRing
            } else {
                Token::Border
            })))
            .render(terminal, &mut cx);
        PseudoTerminal::new(&self.screen)
            .state(self.state())
            .render(content, &mut cx);
        let message = self
            .error
            .as_ref()
            .map_or_else(|| self.note.clone(), |error| format!("PTY error: {error}"));
        Paragraph::new(message)
            .style(accent)
            .render(Rect::new(2, area.height - 2, area.width - 4, 1), &mut cx);
        Paragraph::new("Ctrl+]  change focus     q  close while editor focused")
            .style(base)
            .render(Rect::new(2, area.height - 1, area.width - 4, 1), &mut cx);
    }

    fn needs_animation_frames(&self) -> bool {
        true
    }
}

fn scripted(mut app: Workspace, options: &Options) -> io::Result<()> {
    let area = Rect::new(0, 0, options.width, options.height);
    let mut previous = Grid::new(options.width, options.height);
    let mut next = Grid::new(options.width, options.height);
    let mut clusters = ClusterTable::new();
    let mut encoder = Encoder::new();
    let mut output = io::stdout().lock();
    output.write_all(b"\x1b[?1049h\x1b[?25l")?;
    let epoch = Instant::now();
    for tick in 0..options.frames {
        app.scripted(tick);
        app.render(&mut next, area, &mut clusters);
        let bytes = encoder.encode_diff(&previous, &next, &clusters, ColorMode::TrueColor);
        output.write_all(b"\x1b[?2026h")?;
        output.write_all(bytes)?;
        output.write_all(b"\x1b[?2026l")?;
        output.flush()?;
        std::mem::swap(&mut previous, &mut next);
        let deadline = Duration::from_secs_f64(f64::from(tick + 1) / f64::from(options.fps));
        thread::sleep(deadline.saturating_sub(epoch.elapsed()));
    }
    output.write_all(b"\x1b[?25h\x1b[?1049l")?;
    if let Some(error) = app.error.take() {
        return Err(error);
    }
    if !(0..app.screen.size().rows()).any(|row| app.screen.row_text(row) == "PTY ready") {
        return Err(io::Error::other(
            "scripted child did not reach its final output",
        ));
    }
    Ok(())
}

pub(super) fn run() -> io::Result<()> {
    let options = Options::parse()?;
    let app = Workspace::new(&options)?;
    if options.scripted {
        scripted(app, &options)
    } else {
        lunatui_runtime::run(app, RunOptions::new(options.width, options.height))
    }
}
