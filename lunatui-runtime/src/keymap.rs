// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::fmt;

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{Event, KeyCode, KeyKind, Modifiers};

/// One key press that runs a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyChord {
    code: KeyCode,
    modifiers: Modifiers,
}

impl KeyChord {
    /// A chord that matches a press of `code` with exactly `modifiers`.
    #[must_use]
    pub const fn new(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }

    /// The key identity.
    #[must_use]
    pub const fn code(self) -> KeyCode {
        self.code
    }

    /// The modifier bits, matched exactly.
    #[must_use]
    pub const fn modifiers(self) -> Modifiers {
        self.modifiers
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_control() {
            f.write_str("Ctrl+")?;
        }
        if self.modifiers.is_alt() {
            f.write_str("Alt+")?;
        }
        if self.modifiers.is_shift() {
            f.write_str("Shift+")?;
        }
        match self.code {
            KeyCode::Char(c) => write!(f, "{}", c.to_ascii_uppercase()),
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::Enter => f.write_str("Enter"),
            KeyCode::Esc => f.write_str("Esc"),
            KeyCode::Tab => f.write_str("Tab"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// A user-visible command with the chords that run it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    id: &'static str,
    title: &'static str,
    chords: &'static [KeyChord],
}

impl Command {
    /// A command with a stable `id`, a `title` for the overlay and palette, and its chords.
    #[must_use]
    pub const fn new(id: &'static str, title: &'static str, chords: &'static [KeyChord]) -> Self {
        Self { id, title, chords }
    }

    /// Stable identifier returned by [`Keymap::dispatch`].
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Human-readable name shown in the overlay and palette.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// Every chord that runs this command, in preference order.
    #[must_use]
    pub const fn chords(&self) -> &'static [KeyChord] {
        self.chords
    }
}

/// Ordered registry of commands. The shortcut overlay and command palette both read from it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keymap {
    commands: Vec<Command>,
}

impl Keymap {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `command`. Registration order is the overlay and palette order.
    pub fn register(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Every registered command, in registration order.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// The id of the command whose chord matches a key press, or `None`.
    ///
    /// Only presses match. Repeats and releases are ignored, and modifiers must match exactly.
    #[must_use]
    pub fn dispatch(&self, event: &Event) -> Option<&'static str> {
        let Event::Key(key) = event else {
            return None;
        };
        if key.kind != KeyKind::Press {
            return None;
        }
        self.commands
            .iter()
            .find(|command| {
                command
                    .chords
                    .iter()
                    .any(|chord| chord.code == key.code && chord.modifiers == key.modifiers)
            })
            .map(Command::id)
    }

    /// Commands whose title contains `query`, ignoring ASCII case, in registration order.
    #[must_use]
    pub fn palette(&self, query: &str) -> Vec<&Command> {
        let needle = query.to_ascii_lowercase();
        self.commands
            .iter()
            .filter(|command| command.title.to_ascii_lowercase().contains(&needle))
            .collect()
    }
}

/// Draws the shortcut overlay: one row per command, with its chords and title.
pub fn render_overlay(keymap: &Keymap, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
    let style = Style::new();
    grid.fill(area, style, ' ', clusters);
    grid.put_str(area.x, area.y, style, "Shortcuts", clusters);
    for (row, command) in keymap.commands().iter().enumerate() {
        let chords = command
            .chords()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let line = format!("  {chords:<24}{}", command.title());
        grid.put_str(area.x, list_row(area, row), style, &line, clusters);
    }
}

/// Draws the command palette: a `> query` prompt, then the matching commands.
///
/// The row at `selected` is marked with `>`. Its index counts matches, not registrations.
pub fn render_palette(
    keymap: &Keymap,
    query: &str,
    selected: usize,
    grid: &mut Grid,
    area: Rect,
    clusters: &mut ClusterTable,
) {
    let style = Style::new();
    grid.fill(area, style, ' ', clusters);
    grid.put_str(area.x, area.y, style, &format!("> {query}"), clusters);
    for (row, command) in keymap.palette(query).into_iter().enumerate() {
        let marker = if row == selected { "> " } else { "  " };
        let line = format!("{marker}{}", command.title());
        grid.put_str(area.x, list_row(area, row), style, &line, clusters);
    }
}

fn list_row(area: Rect, index: usize) -> u16 {
    let offset = u16::try_from(index + 1).unwrap_or(u16::MAX);
    area.y.saturating_add(offset)
}
