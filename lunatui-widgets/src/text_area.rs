// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use lunatui_core::{Color, Rect, Size, Style, clip_display_prefix, cluster_display_width};
use lunatui_runtime::keymap::{Command, KeyChord, Keymap};
use lunatui_term::{Event, KeyCode, KeyKind, Modifiers, Paste};
use lunatui_theme::Token;
use unicode_segmentation::UnicodeSegmentation;

use crate::{RenderCx, Widget, state::WidgetState};

const HISTORY_LIMIT: usize = 256;
const HISTORY_BYTES: usize = 1024 * 1024;
const PASTE_LIMIT: usize = 256 * 1024;

/// Cursor position expressed as a line and grapheme ordinal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextPosition {
    /// Zero-based line.
    pub line: usize,
    /// Zero-based grapheme boundary within the line.
    pub grapheme: usize,
}

/// Directed text selection. `anchor` remains fixed while `head` moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSelection {
    /// Fixed end of the selection.
    pub anchor: TextPosition,
    /// Moving end of the selection.
    pub head: TextPosition,
}

/// Input style used by the editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeymapMode {
    /// Familiar control-key editing.
    #[default]
    Emacs,
    /// Vim Normal and Insert modes.
    Vim,
}

/// Current Vim mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VimMode {
    /// Commands move the cursor or change mode.
    #[default]
    Normal,
    /// Printable input edits the document.
    Insert,
}

/// Outcome of handling one input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditResult {
    /// Event did not affect this editor.
    Ignored,
    /// Cursor, selection, mode, search or viewport changed.
    StateChanged,
    /// Document changed and the edit is undoable.
    DocumentChanged,
    /// Document changed but was too large for bounded undo history.
    DocumentChangedWithoutUndo,
    /// Paste was rejected because its size limit was exceeded.
    PasteRejected,
}

#[derive(Clone, Debug)]
struct Edit {
    start: usize,
    removed: String,
    inserted: String,
    cursor_before: usize,
    cursor_after: usize,
    anchor_before: Option<usize>,
    anchor_after: Option<usize>,
}

#[derive(Clone, Debug)]
enum PasteBuffer {
    Collecting(String),
    Rejected,
}

#[derive(Clone, Copy)]
struct RenderLayout {
    body: Rect,
    first_line: usize,
    horizontal: usize,
    gutter: usize,
    gutter_x: u16,
    foreground: Color,
    surface: Color,
}

/// Editable multi-line UTF-8 text with cursor, selection, search and bounded undo.
#[derive(Clone, Debug)]
pub struct TextArea {
    text: String,
    line_starts: Vec<usize>,
    cursor: usize,
    anchor: Option<usize>,
    preferred_col: Option<usize>,
    keymap_mode: KeymapMode,
    vim_mode: VimMode,
    widget_state: WidgetState,
    line_numbers: bool,
    scroll_x: usize,
    scroll_y: usize,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    history_bytes: usize,
    yank: String,
    search: Option<String>,
    search_origin: usize,
    search_match: Option<(usize, usize)>,
    paste: Option<PasteBuffer>,
}

const LEFT: &[KeyChord] = &[
    KeyChord::new(KeyCode::Left, Modifiers::new()),
    KeyChord::new(KeyCode::Left, Modifiers::new().shift()),
];
const RIGHT: &[KeyChord] = &[
    KeyChord::new(KeyCode::Right, Modifiers::new()),
    KeyChord::new(KeyCode::Right, Modifiers::new().shift()),
];
const UP: &[KeyChord] = &[KeyChord::new(KeyCode::Up, Modifiers::new())];
const DOWN: &[KeyChord] = &[KeyChord::new(KeyCode::Down, Modifiers::new())];
const SHIFT_UP: &[KeyChord] = &[KeyChord::new(KeyCode::Up, Modifiers::new().shift())];
const SHIFT_DOWN: &[KeyChord] = &[KeyChord::new(KeyCode::Down, Modifiers::new().shift())];
const UNDO: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('z'),
    Modifiers::new().control(),
)];
const REDO: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('y'),
    Modifiers::new().control(),
)];
const SEARCH_EMACS: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('s'),
    Modifiers::new().control(),
)];
const COPY: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('c'),
    Modifiers::new().control(),
)];
const CUT: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('x'),
    Modifiers::new().control(),
)];
const PASTE: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('v'),
    Modifiers::new().control(),
)];
const VIM_LEFT: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('h'), Modifiers::new()),
    KeyChord::new(KeyCode::Left, Modifiers::new()),
];
const VIM_DOWN: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('j'), Modifiers::new()),
    KeyChord::new(KeyCode::Down, Modifiers::new()),
];
const VIM_UP: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('k'), Modifiers::new()),
    KeyChord::new(KeyCode::Up, Modifiers::new()),
];
const VIM_RIGHT: &[KeyChord] = &[
    KeyChord::new(KeyCode::Char('l'), Modifiers::new()),
    KeyChord::new(KeyCode::Right, Modifiers::new()),
];
const ARROW_LEFT: &[KeyChord] = &[KeyChord::new(KeyCode::Left, Modifiers::new())];
const ARROW_DOWN: &[KeyChord] = &[KeyChord::new(KeyCode::Down, Modifiers::new())];
const ARROW_UP: &[KeyChord] = &[KeyChord::new(KeyCode::Up, Modifiers::new())];
const ARROW_RIGHT: &[KeyChord] = &[KeyChord::new(KeyCode::Right, Modifiers::new())];
const INSERT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('i'), Modifiers::new())];
const APPEND: &[KeyChord] = &[KeyChord::new(KeyCode::Char('a'), Modifiers::new())];
const NORMAL: &[KeyChord] = &[KeyChord::new(KeyCode::Esc, Modifiers::new())];
const SEARCH: &[KeyChord] = &[KeyChord::new(KeyCode::Char('/'), Modifiers::new())];
const VIM_UNDO: &[KeyChord] = &[KeyChord::new(KeyCode::Char('u'), Modifiers::new())];
const FIND_NEXT: &[KeyChord] = &[KeyChord::new(KeyCode::Char('n'), Modifiers::new())];
const DELETE_CHAR: &[KeyChord] = &[KeyChord::new(KeyCode::Char('x'), Modifiers::new())];
const VIM_REDO: &[KeyChord] = &[KeyChord::new(
    KeyCode::Char('r'),
    Modifiers::new().control(),
)];

fn emacs_keymap() -> Keymap {
    let mut map = Keymap::new();
    map.register(Command::new("move-left", "Move left", LEFT));
    map.register(Command::new("move-right", "Move right", RIGHT));
    map.register(Command::new("move-up", "Move up", UP));
    map.register(Command::new("extend-up", "Extend selection up", SHIFT_UP));
    map.register(Command::new("move-down", "Move down", DOWN));
    map.register(Command::new(
        "extend-down",
        "Extend selection down",
        SHIFT_DOWN,
    ));
    map.register(Command::new("undo", "Undo", UNDO));
    map.register(Command::new("redo", "Redo", REDO));
    map.register(Command::new("search", "Search", SEARCH_EMACS));
    map.register(Command::new("copy", "Copy selection", COPY));
    map.register(Command::new("cut", "Cut selection", CUT));
    map.register(Command::new("paste", "Paste selection", PASTE));
    map
}

fn vim_insert_keymap() -> Keymap {
    let mut map = Keymap::new();
    map.register(Command::new("move-left", "Move left", ARROW_LEFT));
    map.register(Command::new("move-down", "Move down", ARROW_DOWN));
    map.register(Command::new("move-up", "Move up", ARROW_UP));
    map.register(Command::new("move-right", "Move right", ARROW_RIGHT));
    map.register(Command::new("normal-mode", "Normal mode", NORMAL));
    map
}

fn vim_normal_keymap() -> Keymap {
    let mut map = Keymap::new();
    map.register(Command::new("move-left", "Move left", VIM_LEFT));
    map.register(Command::new("move-down", "Move down", VIM_DOWN));
    map.register(Command::new("move-up", "Move up", VIM_UP));
    map.register(Command::new("move-right", "Move right", VIM_RIGHT));
    map.register(Command::new("insert-mode", "Insert mode", INSERT));
    map.register(Command::new("append-mode", "Append mode", APPEND));
    map.register(Command::new("search", "Search", SEARCH));
    map.register(Command::new("undo", "Undo", VIM_UNDO));
    map.register(Command::new("redo", "Redo", VIM_REDO));
    map.register(Command::new("find-next", "Find next", FIND_NEXT));
    map.register(Command::new("delete-char", "Delete character", DELETE_CHAR));
    map
}

impl TextArea {
    /// Creates an editor initialized with `text`.
    #[must_use]
    pub fn new(text: impl AsRef<str>) -> Self {
        let text = normalize_newlines(text.as_ref());
        let line_starts = make_line_starts(&text);
        Self {
            text,
            line_starts,
            cursor: 0,
            anchor: None,
            preferred_col: None,
            keymap_mode: KeymapMode::Emacs,
            vim_mode: VimMode::Insert,
            widget_state: WidgetState::Normal,
            line_numbers: false,
            scroll_x: 0,
            scroll_y: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            history_bytes: 0,
            yank: String::new(),
            search: None,
            search_origin: 0,
            search_match: None,
            paste: None,
        }
    }

    /// Current document contents.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the document and clears edit history.
    pub fn set_text(&mut self, text: impl AsRef<str>) {
        self.text = normalize_newlines(text.as_ref());
        self.line_starts = make_line_starts(&self.text);
        self.cursor = self.cursor.min(self.text.len());
        self.cursor = snap_grapheme_right(&self.text, self.cursor);
        self.anchor = None;
        self.preferred_col = None;
        self.undo.clear();
        self.redo.clear();
        self.history_bytes = 0;
        self.search = None;
        self.search_match = None;
        self.paste = None;
    }

    /// Current cursor as a grapheme position.
    #[must_use]
    pub fn cursor(&self) -> TextPosition {
        self.position(self.cursor)
    }

    /// Current directed selection, if its endpoints differ.
    #[must_use]
    pub fn selection(&self) -> Option<TextSelection> {
        let anchor = self.anchor?;
        (anchor != self.cursor).then(|| TextSelection {
            anchor: self.position(anchor),
            head: self.position(self.cursor),
        })
    }

    /// Selects an editing keymap.
    #[must_use]
    pub fn keymap_mode(mut self, mode: KeymapMode) -> Self {
        self.keymap_mode = mode;
        self.vim_mode = if mode == KeymapMode::Vim {
            VimMode::Normal
        } else {
            VimMode::Insert
        };
        self
    }

    /// Selects the keymap after construction.
    pub fn set_keymap_mode(&mut self, mode: KeymapMode) {
        self.keymap_mode = mode;
        self.vim_mode = if mode == KeymapMode::Vim {
            VimMode::Normal
        } else {
            VimMode::Insert
        };
    }

    /// Current keymap mode.
    #[must_use]
    pub const fn active_keymap_mode(&self) -> KeymapMode {
        self.keymap_mode
    }

    /// Current Vim submode.
    #[must_use]
    pub const fn vim_mode(&self) -> VimMode {
        self.vim_mode
    }

    /// Registry used by the selected mode, for shortcut overlays.
    #[must_use]
    pub fn keymap_registry(&self) -> &'static Keymap {
        static EMACS: OnceLock<Keymap> = OnceLock::new();
        static VIM_NORMAL: OnceLock<Keymap> = OnceLock::new();
        static VIM_INSERT: OnceLock<Keymap> = OnceLock::new();
        let (cell, build): (&OnceLock<Keymap>, fn() -> Keymap) =
            match (self.keymap_mode, self.vim_mode) {
                (KeymapMode::Emacs, _) => (&EMACS, emacs_keymap),
                (KeymapMode::Vim, VimMode::Insert) => (&VIM_INSERT, vim_insert_keymap),
                (KeymapMode::Vim, VimMode::Normal) => (&VIM_NORMAL, vim_normal_keymap),
            };
        cell.get_or_init(build)
    }

    /// Sets the widget's visual interaction state.
    pub fn set_widget_state(&mut self, state: WidgetState) {
        self.widget_state = state;
        if state == WidgetState::Disabled {
            self.paste = None;
        }
    }

    /// Sets line-number display.
    #[must_use]
    pub fn line_numbers(mut self, enabled: bool) -> Self {
        self.line_numbers = enabled;
        self
    }

    /// Sets the first visible line.
    pub fn scroll_to(&mut self, line: usize) {
        self.scroll_y = line;
    }

    /// Sets the first visible display column.
    pub fn scroll_to_column(&mut self, column: usize) {
        self.scroll_x = column;
    }

    /// Current search query, if interactive search is open.
    #[must_use]
    pub fn search_query(&self) -> Option<&str> {
        self.search.as_deref()
    }

    /// Executes a command registered for the active keymap; unknown ids are ignored.
    pub fn execute_command(&mut self, command_id: &str) -> EditResult {
        let registered = self
            .keymap_registry()
            .commands()
            .iter()
            .any(|command| command.id() == command_id);
        if registered {
            self.run_command(command_id, Modifiers::new())
        } else {
            EditResult::Ignored
        }
    }

    /// Opens incremental search at the current cursor position.
    pub fn start_search(&mut self) {
        self.search_origin = self.cursor;
        self.search = Some(String::new());
        self.search_match = None;
    }

    /// Accepts the current search match and closes the search input.
    pub fn accept_search(&mut self) {
        self.search = None;
    }

    /// Cancels search and restores the cursor position from before it opened.
    pub fn cancel_search(&mut self) {
        if self.search.take().is_some() {
            self.cursor = self.search_origin;
            self.search_match = None;
        }
    }

    /// Moves the cursor to `position`, optionally extending a directed selection.
    pub fn move_to(&mut self, position: TextPosition, extend: bool) {
        let next = self.byte_at_position(position);
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = next;
        self.preferred_col = None;
    }

    /// Handles one terminal event.
    pub fn handle_event(&mut self, event: &Event) -> EditResult {
        if self.widget_state == WidgetState::Disabled {
            return EditResult::Ignored;
        }
        match event {
            Event::Paste(Paste::Start) => {
                self.paste = Some(PasteBuffer::Collecting(String::new()));
                EditResult::StateChanged
            }
            Event::Paste(Paste::Chunk(chunk)) => {
                let Some(state) = self.paste.as_mut() else {
                    return if self.search.is_some() {
                        self.append_search_text(chunk)
                    } else {
                        self.insert_text(chunk)
                    };
                };
                match state {
                    PasteBuffer::Rejected => EditResult::Ignored,
                    PasteBuffer::Collecting(buffer) => {
                        if buffer.len().saturating_add(chunk.len()) > PASTE_LIMIT {
                            *state = PasteBuffer::Rejected;
                            EditResult::PasteRejected
                        } else {
                            buffer.push_str(chunk);
                            EditResult::StateChanged
                        }
                    }
                }
            }
            Event::Paste(Paste::End) => {
                let Some(state) = self.paste.take() else {
                    return EditResult::Ignored;
                };
                let PasteBuffer::Collecting(buffer) = state else {
                    return EditResult::Ignored;
                };
                if self.search.is_some() {
                    self.append_search_text(&buffer)
                } else {
                    self.insert_text(&buffer)
                }
            }
            Event::Key(key) if key.kind == KeyKind::Release => EditResult::Ignored,
            Event::Key(_) => self.handle_key(event),
            _ => EditResult::Ignored,
        }
    }

    fn handle_key(&mut self, event: &Event) -> EditResult {
        let Event::Key(key) = event else {
            return EditResult::Ignored;
        };
        let code = key.code;
        let modifiers = key.modifiers;
        let text = key.text.as_deref();
        if self.search.is_some() {
            return self.search_key(code, modifiers, text);
        }
        if let Some(command) = self.keymap_registry().dispatch(event) {
            return self.run_command(command, modifiers);
        }
        if self.keymap_mode == KeymapMode::Vim && self.vim_mode == VimMode::Normal {
            if modifiers != Modifiers::new() {
                return EditResult::Ignored;
            }
            return match code {
                KeyCode::Char('h') | KeyCode::Left => self.move_horizontal(false, false),
                KeyCode::Char('l') | KeyCode::Right => self.move_horizontal(true, false),
                KeyCode::Char('j') | KeyCode::Down => self.move_vertical(1, false),
                KeyCode::Char('k') | KeyCode::Up => self.move_vertical(-1, false),
                KeyCode::Char('i') => {
                    self.vim_mode = VimMode::Insert;
                    EditResult::StateChanged
                }
                KeyCode::Char('a') => {
                    self.move_horizontal(true, false);
                    self.vim_mode = VimMode::Insert;
                    EditResult::StateChanged
                }
                KeyCode::Char('/') => {
                    self.start_search();
                    EditResult::StateChanged
                }
                KeyCode::Char('x') => self.delete_forward(),
                KeyCode::Char('u') => self.undo(),
                KeyCode::Char('n') => self.find_next(),
                _ => EditResult::Ignored,
            };
        }
        let control = modifiers == Modifiers::new().control();
        if control {
            return match code {
                KeyCode::Char('a') => self.move_line_edge(false, modifiers.is_shift()),
                KeyCode::Char('e') => self.move_line_edge(true, modifiers.is_shift()),
                KeyCode::Char('s') => {
                    self.start_search();
                    EditResult::StateChanged
                }
                KeyCode::Char('z') => self.undo(),
                KeyCode::Char('y') => self.redo(),
                KeyCode::Char('c') => self.copy_selection(),
                KeyCode::Char('x') => {
                    if self.copy_selection() == EditResult::StateChanged {
                        self.delete_selection()
                    } else {
                        EditResult::Ignored
                    }
                }
                KeyCode::Char('v') if !self.yank.is_empty() => self.insert_text(&self.yank.clone()),
                KeyCode::Char('f') => self.find_next(),
                _ => EditResult::Ignored,
            };
        }
        match code {
            KeyCode::Left => self.move_horizontal(false, modifiers.is_shift()),
            KeyCode::Right => self.move_horizontal(true, modifiers.is_shift()),
            KeyCode::Up => self.move_vertical(-1, modifiers.is_shift()),
            KeyCode::Down => self.move_vertical(1, modifiers.is_shift()),
            KeyCode::Home => self.move_line_edge(false, modifiers.is_shift()),
            KeyCode::End => self.move_line_edge(true, modifiers.is_shift()),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Enter => self.insert_text("\n"),
            KeyCode::Tab => self.insert_text("\t"),
            KeyCode::Esc if self.keymap_mode == KeymapMode::Vim => {
                self.vim_mode = VimMode::Normal;
                EditResult::StateChanged
            }
            KeyCode::Char(c) => {
                if !allows_text_input(modifiers) {
                    return EditResult::Ignored;
                }
                if let Some(text) = text {
                    self.insert_text(text)
                } else {
                    self.insert_text(&c.to_string())
                }
            }
            _ => EditResult::Ignored,
        }
    }

    fn run_command(&mut self, command_id: &str, modifiers: Modifiers) -> EditResult {
        match command_id {
            "move-left" => self.move_horizontal(false, modifiers.is_shift()),
            "move-right" => self.move_horizontal(true, modifiers.is_shift()),
            "move-up" => self.move_vertical(-1, modifiers.is_shift()),
            "move-down" => self.move_vertical(1, modifiers.is_shift()),
            "extend-up" => self.move_vertical(-1, true),
            "extend-down" => self.move_vertical(1, true),
            "undo" => self.undo(),
            "redo" => self.redo(),
            "search" => {
                self.start_search();
                EditResult::StateChanged
            }
            "copy" => self.copy_selection(),
            "cut" => {
                if self.copy_selection() == EditResult::StateChanged {
                    self.delete_selection()
                } else {
                    EditResult::Ignored
                }
            }
            "paste" if !self.yank.is_empty() => self.insert_text(&self.yank.clone()),
            "insert-mode" if self.keymap_mode == KeymapMode::Vim => {
                self.vim_mode = VimMode::Insert;
                EditResult::StateChanged
            }
            "append-mode" if self.keymap_mode == KeymapMode::Vim => {
                self.move_horizontal(true, false);
                self.vim_mode = VimMode::Insert;
                EditResult::StateChanged
            }
            "normal-mode" if self.keymap_mode == KeymapMode::Vim => {
                self.vim_mode = VimMode::Normal;
                EditResult::StateChanged
            }
            "find-next" => self.find_next(),
            "delete-char" => self.delete_forward(),
            _ => EditResult::Ignored,
        }
    }

    fn insert_text(&mut self, text: &str) -> EditResult {
        let text = normalize_newlines(text);
        let (start, end) = self.selection_bytes().unwrap_or((self.cursor, self.cursor));
        self.replace(start, end, &text)
    }
    fn backspace(&mut self) -> EditResult {
        if self.selection_bytes().is_some() {
            return self.delete_selection();
        }
        let start = previous_grapheme(&self.text, self.cursor);
        if start == self.cursor {
            return EditResult::Ignored;
        }
        self.replace(start, self.cursor, "")
    }
    fn delete_forward(&mut self) -> EditResult {
        if self.selection_bytes().is_some() {
            return self.delete_selection();
        }
        let end = next_grapheme(&self.text, self.cursor);
        if end == self.cursor {
            return EditResult::Ignored;
        }
        self.replace(self.cursor, end, "")
    }
    fn delete_selection(&mut self) -> EditResult {
        let Some((start, end)) = self.selection_bytes() else {
            return EditResult::Ignored;
        };
        self.replace(start, end, "")
    }
    fn copy_selection(&mut self) -> EditResult {
        if let Some((start, end)) = self.selection_bytes() {
            self.yank.clear();
            self.yank.push_str(&self.text[start..end]);
            EditResult::StateChanged
        } else {
            EditResult::Ignored
        }
    }

    fn replace(&mut self, start: usize, end: usize, insert: &str) -> EditResult {
        let before = self.cursor;
        let anchor_before = self.anchor;
        let removed = self.text[start..end].to_owned();
        let cost = removed.len().saturating_add(insert.len());
        self.replace_document(start, end, insert);
        self.cursor = snap_grapheme_right(&self.text, start + insert.len());
        self.anchor = None;
        self.preferred_col = None;
        for old in self.redo.drain(..) {
            self.history_bytes = self
                .history_bytes
                .saturating_sub(old.removed.len() + old.inserted.len());
        }
        self.search_match = None;
        if cost > HISTORY_BYTES {
            self.undo.clear();
            self.history_bytes = 0;
            return EditResult::DocumentChangedWithoutUndo;
        }
        let edit = Edit {
            start,
            removed,
            inserted: insert.to_owned(),
            cursor_before: before,
            cursor_after: self.cursor,
            anchor_before,
            anchor_after: self.anchor,
        };
        self.history_bytes += cost;
        self.undo.push(edit);
        while self.undo.len() > HISTORY_LIMIT || self.history_bytes > HISTORY_BYTES {
            if let Some(old) = self.undo.first() {
                self.history_bytes = self
                    .history_bytes
                    .saturating_sub(old.removed.len() + old.inserted.len());
            }
            self.undo.remove(0);
        }
        EditResult::DocumentChanged
    }

    /// Reverts the most recent undoable document change.
    pub fn undo(&mut self) -> EditResult {
        let Some(edit) = self.undo.pop() else {
            return EditResult::Ignored;
        };
        let end = edit.start + edit.inserted.len();
        self.replace_document(edit.start, end, &edit.removed);
        self.cursor = edit.cursor_before;
        self.anchor = edit.anchor_before;
        self.redo.push(edit);
        EditResult::DocumentChanged
    }
    /// Reapplies the most recently undone document change.
    pub fn redo(&mut self) -> EditResult {
        let Some(edit) = self.redo.pop() else {
            return EditResult::Ignored;
        };
        let end = edit.start + edit.removed.len();
        self.replace_document(edit.start, end, &edit.inserted);
        self.cursor = edit.cursor_after;
        self.anchor = edit.anchor_after;
        self.undo.push(edit);
        EditResult::DocumentChanged
    }

    fn search_key(
        &mut self,
        code: KeyCode,
        modifiers: Modifiers,
        text: Option<&str>,
    ) -> EditResult {
        match code {
            KeyCode::Esc => {
                self.cancel_search();
                EditResult::StateChanged
            }
            KeyCode::Enter => {
                self.accept_search();
                EditResult::StateChanged
            }
            KeyCode::Backspace => {
                if let Some(q) = &mut self.search {
                    q.pop();
                }
                self.find_next();
                EditResult::StateChanged
            }
            KeyCode::Char(_) => {
                if !allows_text_input(modifiers) {
                    return EditResult::Ignored;
                }
                if let Some(text) = text {
                    self.append_search_text(text)
                } else {
                    match code {
                        KeyCode::Char(c) => self.append_search_text(&c.to_string()),
                        _ => EditResult::Ignored,
                    }
                }
            }
            _ => EditResult::Ignored,
        }
    }

    fn append_search_text(&mut self, text: &str) -> EditResult {
        if let Some(query) = self.search.as_mut() {
            query.push_str(&normalize_newlines(text));
            self.find_next();
            EditResult::StateChanged
        } else {
            EditResult::Ignored
        }
    }

    fn find_next(&mut self) -> EditResult {
        let Some(query) = self.search.as_deref().filter(|q| !q.is_empty()) else {
            return EditResult::Ignored;
        };
        let from = next_grapheme(&self.text, self.cursor);
        let found = self.text[from..]
            .find(query)
            .map(|i| from + i)
            .or_else(|| self.text[..from].find(query));
        if let Some(pos) = found {
            self.cursor = snap_grapheme_right(&self.text, pos);
            self.anchor = None;
            self.search_match = Some((pos, pos + query.len()));
            EditResult::StateChanged
        } else {
            self.search_match = None;
            EditResult::Ignored
        }
    }

    fn move_horizontal(&mut self, right: bool, extend: bool) -> EditResult {
        self.preferred_col = None;
        let next = if right {
            next_grapheme(&self.text, self.cursor)
        } else {
            previous_grapheme(&self.text, self.cursor)
        };
        self.move_byte(next, extend)
    }
    fn move_vertical(&mut self, direction: i32, extend: bool) -> EditResult {
        let pos = self.position(self.cursor);
        let line = if direction < 0 {
            pos.line.saturating_sub(1)
        } else {
            (pos.line + 1).min(self.line_starts.len() - 1)
        };
        let current_start = self.line_starts[pos.line];
        let col = *self
            .preferred_col
            .get_or_insert_with(|| display_columns(&self.text[current_start..self.cursor]));
        let target_start = self.line_starts[line];
        let target_end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        let target_text = self.text[target_start..target_end].trim_end_matches('\n');
        let mut display_col = 0;
        let mut target_byte = target_start + target_text.len();
        for (index, grapheme) in target_text.grapheme_indices(true) {
            let next = display_col + grapheme_columns(grapheme, display_col);
            if next > col {
                target_byte = if col - display_col < next - col {
                    target_start + index
                } else {
                    target_start + index + grapheme.len()
                };
                break;
            }
            display_col = next;
        }
        self.move_byte(target_byte, extend)
    }
    fn move_line_edge(&mut self, end: bool, extend: bool) -> EditResult {
        self.preferred_col = None;
        let p = self.position(self.cursor);
        let line_start = self.line_starts[p.line];
        let line_end = self
            .line_starts
            .get(p.line + 1)
            .copied()
            .unwrap_or(self.text.len())
            .saturating_sub(usize::from(p.line + 1 < self.line_starts.len()));
        self.move_byte(if end { line_end } else { line_start }, extend)
    }
    fn move_byte(&mut self, next: usize, extend: bool) -> EditResult {
        if next == self.cursor && (!extend || self.anchor.is_some()) {
            return EditResult::Ignored;
        }
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = next;
        EditResult::StateChanged
    }

    fn replace_document(&mut self, start: usize, end: usize, insert: &str) {
        let removed_len = end - start;
        let inserted_len = insert.len();
        let mut starts = Vec::with_capacity(self.line_starts.len() + insert.matches('\n').count());
        starts.extend(
            self.line_starts
                .iter()
                .copied()
                .take_while(|line| *line <= start),
        );
        starts.extend(
            insert
                .match_indices('\n')
                .map(|(offset, _)| start + offset + 1),
        );
        if inserted_len >= removed_len {
            let shift = inserted_len - removed_len;
            starts.extend(
                self.line_starts
                    .iter()
                    .copied()
                    .filter(|line| *line > end)
                    .map(|line| line + shift),
            );
        } else {
            let shift = removed_len - inserted_len;
            starts.extend(
                self.line_starts
                    .iter()
                    .copied()
                    .filter(|line| *line > end)
                    .map(|line| line - shift),
            );
        }
        self.text.replace_range(start..end, insert);
        self.line_starts = starts;
    }
    fn selection_bytes(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        (anchor != self.cursor).then_some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }
    fn position(&self, byte: usize) -> TextPosition {
        let line = self
            .line_starts
            .partition_point(|start| *start <= byte)
            .saturating_sub(1);
        let start = self.line_starts[line];
        let end = byte.min(self.text.len());
        TextPosition {
            line,
            grapheme: self.text[start..end].graphemes(true).count(),
        }
    }
    fn byte_at_position(&self, position: TextPosition) -> usize {
        let line = position.line.min(self.line_starts.len() - 1);
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        let line_text = self.text[start..end].trim_end_matches('\n');
        start
            + line_text
                .grapheme_indices(true)
                .nth(position.grapheme)
                .map_or(line_text.len(), |(i, _)| i)
    }

    fn render_visible_lines(
        &self,
        layout: RenderLayout,
        cursor: TextPosition,
        cx: &mut RenderCx<'_>,
    ) -> Option<(u16, u16)> {
        let mut cursor_screen = None;
        for row in 0..layout.body.height {
            let line = layout.first_line + usize::from(row);
            if line >= self.line_starts.len() {
                break;
            }
            cursor_screen = self
                .render_line(line, row, layout, cursor, cx)
                .or(cursor_screen);
        }
        cursor_screen
    }

    fn render_line(
        &self,
        line: usize,
        row: u16,
        layout: RenderLayout,
        cursor: TextPosition,
        cx: &mut RenderCx<'_>,
    ) -> Option<(u16, u16)> {
        let RenderLayout {
            body,
            horizontal,
            gutter,
            gutter_x,
            surface,
            ..
        } = layout;
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.text.len());
        let value = self.text[start..end].trim_end_matches('\n');
        let y = body.y + row;
        if self.line_numbers {
            let (label, len) = line_number_label(line + 1, gutter);
            cx.put_str(
                gutter_x,
                y,
                std::str::from_utf8(&label[..len]).unwrap_or(""),
                Style::new().fg(cx.token(Token::TextMuted)).bg(surface),
            );
        }
        let mut cursor_screen = None;
        let mut column = 0;
        for (index, (byte_offset, text)) in value.grapheme_indices(true).enumerate() {
            let cells = grapheme_columns(text, column);
            let next = column + cells;
            if next <= horizontal || (column < horizontal && text != "\t") {
                column = next;
                continue;
            }
            let visual = column.saturating_sub(horizontal);
            if line == cursor.line && index == cursor.grapheme && visual < usize::from(body.width) {
                cursor_screen = Some((body.x + u16::try_from(visual).unwrap_or(u16::MAX), y));
            }
            if visual >= usize::from(body.width)
                || (visual + cells > usize::from(body.width) && text != "\t")
            {
                break;
            }
            let x = body.x + u16::try_from(visual).unwrap_or(u16::MAX);
            let style = self.grapheme_style(start + byte_offset, text.len(), layout, cx);
            if text == "\t" {
                let visible_end = next.saturating_sub(horizontal).min(usize::from(body.width));
                cx.fill(
                    Rect::new(x, y, u16::try_from(visible_end - visual).unwrap_or(0), 1),
                    style,
                    ' ',
                );
            } else {
                cx.put_str(x, y, text, style);
            }
            column = next;
        }
        if line == cursor.line && cursor.grapheme == value.graphemes(true).count() {
            let visual = display_columns(value).saturating_sub(horizontal);
            if visual < usize::from(body.width) {
                cursor_screen = Some((body.x + u16::try_from(visual).unwrap_or(0), y));
            }
        }
        cursor_screen
    }

    fn grapheme_style(
        &self,
        start: usize,
        len: usize,
        layout: RenderLayout,
        cx: &RenderCx<'_>,
    ) -> Style {
        let selected = self
            .selection_bytes()
            .is_some_and(|(a, b)| start < b && start + len > a);
        let search_hit = self
            .search_match
            .is_some_and(|(a, b)| start < b && start + len > a);
        if selected {
            Style::new()
                .fg(cx.token(Token::Surface0))
                .bg(cx.token(Token::FocusFill))
        } else if search_hit {
            Style::new()
                .fg(cx.token(Token::TextStrong))
                .bg(cx.token(Token::Warn))
        } else {
            Style::new().fg(layout.foreground).bg(layout.surface)
        }
    }

    fn render_cursor(&self, cursor: Option<(u16, u16)>, focused: bool, cx: &mut RenderCx<'_>) {
        if self.widget_state != WidgetState::Disabled
            && let Some((x, y)) = cursor
        {
            let token = if focused {
                Token::FocusFill
            } else {
                Token::SurfaceRaised
            };
            cx.set_style(
                Rect::new(x, y, 1, 1),
                Style::new()
                    .bg(cx.token(token))
                    .fg(cx.token(Token::TextPrimary)),
            );
        }
    }

    fn render_search(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let Some(query) = self.search.as_deref() else {
            return;
        };
        let y = area.y.saturating_add(area.height.saturating_sub(1));
        let style = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(cx.token(Token::SurfaceRaised));
        cx.put_str(area.x, y, "/", style);
        let visible = clip_display_prefix(query, area.width - 1);
        cx.put_str(area.x.saturating_add(1), y, visible, style);
    }

    fn render_inner(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let focused = self.widget_state == WidgetState::Focused;
        let surface = cx.token(Token::Surface0);
        let foreground = cx.token(if self.widget_state == WidgetState::Disabled {
            Token::TextMuted
        } else {
            Token::TextPrimary
        });
        let gutter = if self.line_numbers {
            decimal_digits(self.line_starts.len()) + 1
        } else {
            0
        };
        let x_offset = u16::try_from(gutter).unwrap_or(u16::MAX).min(area.width);
        let body = Rect::new(
            area.x.saturating_add(x_offset),
            area.y,
            area.width - x_offset,
            area.height,
        );
        cx.fill(area, Style::new().fg(foreground).bg(surface), ' ');
        let cursor = self.position(self.cursor);
        let first = self
            .scroll_y
            .min(self.line_starts.len().saturating_sub(1))
            .min(cursor.line);
        let first = if cursor.line >= first.saturating_add(usize::from(body.height)) {
            cursor
                .line
                .saturating_sub(usize::from(body.height).saturating_sub(1))
        } else {
            first
        };
        let cursor_line_start = self.line_starts[cursor.line];
        let cursor_column = display_columns(&self.text[cursor_line_start..self.cursor]);
        let horizontal = if cursor_column < self.scroll_x {
            cursor_column
        } else if body.width > 0
            && cursor_column >= self.scroll_x.saturating_add(usize::from(body.width))
        {
            cursor_column.saturating_sub(usize::from(body.width).saturating_sub(1))
        } else {
            self.scroll_x
        };
        let layout = RenderLayout {
            body,
            first_line: first,
            horizontal,
            gutter,
            gutter_x: area.x,
            foreground,
            surface,
        };
        let cursor_screen = self.render_visible_lines(layout, cursor, cx);
        self.render_cursor(cursor_screen, focused, cx);
        if self.search.is_some() {
            self.render_search(area, cx);
        }
    }
}

impl Widget for TextArea {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        self.render_inner(area, cx);
    }
    fn measure(&self, avail: Size) -> Size {
        avail
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
fn allows_text_input(modifiers: Modifiers) -> bool {
    !modifiers.is_control()
        && !modifiers.is_alt()
        && !modifiers.is_super()
        && !modifiers.is_hyper()
        && !modifiers.is_meta()
}
fn make_line_starts(text: &str) -> Vec<usize> {
    let mut v = vec![0];
    v.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    v
}
fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}
fn line_number_label(value: usize, width: usize) -> ([u8; 21], usize) {
    let len = width.min(21);
    let mut output = [b' '; 21];
    let mut digits = [0; 20];
    let mut value = value;
    let mut count = 0;
    loop {
        digits[count] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        count += 1;
        value /= 10;
        if value == 0 || count == digits.len() {
            break;
        }
    }
    let digits_start = len.saturating_sub(1 + count);
    for (index, digit) in digits[..count].iter().rev().enumerate() {
        let target = digits_start + index;
        if target < len.saturating_sub(1) {
            output[target] = *digit;
        }
    }
    if len > 0 {
        output[len - 1] = b' ';
    }
    (output, len)
}
fn floor_char_boundary(text: &str, at: usize) -> usize {
    let mut i = at.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}
fn snap_grapheme_right(text: &str, at: usize) -> usize {
    let at = floor_char_boundary(text, at);
    if at >= text.len() {
        return at;
    }
    for (start, grapheme) in text.grapheme_indices(true) {
        let end = start + grapheme.len();
        if at == start {
            return at;
        }
        if at < end {
            return end;
        }
    }
    text.len()
}
fn previous_grapheme(text: &str, at: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|i| *i < at)
        .last()
        .unwrap_or(0)
}
fn next_grapheme(text: &str, at: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, g)| i + g.len())
        .find(|end| *end > at)
        .unwrap_or(text.len())
}

fn grapheme_columns(text: &str, column: usize) -> usize {
    if text == "\t" {
        4 - column % 4
    } else {
        cluster_display_width(text)
    }
}

fn display_columns(text: &str) -> usize {
    text.graphemes(true).fold(0, |column, grapheme| {
        column + grapheme_columns(grapheme, column)
    })
}
