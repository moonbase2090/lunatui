// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

use lunatui_runtime::{
    Handle, Orientation, Outcome, RovingFocus, Selection, SelectionMode, Typeahead, Typed,
};
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers};

#[cfg(any(feature = "select", feature = "file-picker"))]
use lunatui_runtime::{Dismissal, FocusManager, Layer, LayerStack};

fn key_of(event: &Event) -> Option<&KeyEvent> {
    match event {
        Event::Key(key) if matches!(key.kind, KeyKind::Press | KeyKind::Repeat) => Some(key),
        _ => None,
    }
}

fn plain(key: &KeyEvent, code: KeyCode) -> bool {
    key.code == code && key.modifiers == Modifiers::new()
}

/// Keyboard state for a [`List`](crate::List): the focused row and the chosen rows.
///
/// Up and Down move focus and stop at the ends, Home and End jump to them, Page Up and Page Down
/// move a page. Space chooses the focused row ([`Outcome::Toggled`]) and Enter activates it
/// ([`Outcome::Activated`]). Call [`sync`](Self::sync) when the rows or the visible height change,
/// and draw with `List::selected(state.focused())`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListState {
    roving: RovingFocus,
    selection: Selection,
    count: usize,
    page: usize,
}

impl ListState {
    /// No rows yet, choosing one row at a time in `mode`.
    #[must_use]
    pub const fn new(mode: SelectionMode) -> Self {
        Self {
            roving: RovingFocus::new(Orientation::Vertical).wrap(false),
            selection: Selection::new(mode),
            count: 0,
            page: 1,
        }
    }

    /// Records the number of rows and how many fit on screen, keeping focus and choices on rows
    /// that still exist.
    pub fn sync(&mut self, count: usize, visible_rows: usize) {
        self.count = count;
        self.page = visible_rows.max(1);
        self.roving.sync(count, |_| true);
        self.selection.retain_below(count);
    }

    /// The focused row.
    #[must_use]
    pub const fn focused(&self) -> Option<usize> {
        self.roving.current()
    }

    /// Focuses `row`.
    pub fn focus(&mut self, row: Option<usize>) {
        self.roving.set(row.filter(|row| *row < self.count));
    }

    /// The chosen rows.
    #[must_use]
    pub const fn selection(&self) -> &Selection {
        &self.selection
    }
}

impl Handle for ListState {
    fn handle(&mut self, event: &Event) -> Outcome {
        let Some(key) = key_of(event) else {
            return Outcome::Ignored;
        };
        let page = isize::try_from(self.page).unwrap_or(isize::MAX);
        let before = self.roving.current();
        if plain(key, KeyCode::PageDown) || plain(key, KeyCode::PageUp) {
            let delta = if key.code == KeyCode::PageDown {
                page
            } else {
                -page
            };
            self.roving.move_by(self.count, delta, |_| true);
            return if self.roving.current() == before {
                Outcome::Handled
            } else {
                Outcome::Changed
            };
        }
        if plain(key, KeyCode::Enter) {
            return before.map_or(Outcome::Handled, Outcome::Activated);
        }
        if plain(key, KeyCode::Char(' ')) {
            let Some(row) = before else {
                return Outcome::Handled;
            };
            self.selection.toggle(row);
            return Outcome::Toggled(row);
        }
        self.roving.handle_key(key, self.count, |_| true)
    }
}

/// Keyboard state for a [`Tabs`](crate::Tabs) bar: Left and Right move to the next tab and wrap
/// around, Home and End jump to the first and last. Call [`sync`](Self::sync) when the tabs change
/// and draw with `Tabs::select(state.selected())`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabsState {
    roving: RovingFocus,
    count: usize,
}

impl TabsState {
    /// The first of `count` tabs selected.
    #[must_use]
    pub fn new(count: usize) -> Self {
        let mut roving = RovingFocus::new(Orientation::Horizontal);
        roving.set((count > 0).then_some(0));
        Self { roving, count }
    }

    /// Records the number of tabs, keeping the selection on a tab that still exists.
    pub fn sync(&mut self, count: usize) {
        self.count = count;
        self.roving.sync(count, |_| true);
        if self.roving.current().is_none() && count > 0 {
            self.roving.set(Some(0));
        }
    }

    /// The selected tab, `0` when there are none.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.roving.current().unwrap_or(0)
    }

    /// Selects `index` when it is a tab.
    pub fn select(&mut self, index: usize) {
        if index < self.count {
            self.roving.set(Some(index));
        }
    }
}

impl Handle for TabsState {
    fn handle(&mut self, event: &Event) -> Outcome {
        key_of(event).map_or(Outcome::Ignored, |key| {
            self.roving.handle_key(key, self.count, |_| true)
        })
    }
}

/// Keyboard state for a [`RadioGroup`](crate::RadioGroup): the arrow keys move focus to the next
/// enabled option, wrapping around, and select it; Space selects the focused option. Call
/// [`sync`](Self::sync) with the group each frame and draw with
/// `group.select(state.selected()).focus(state.focused())`.
#[cfg(feature = "radio")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RadioState {
    roving: RovingFocus,
    selected: Option<usize>,
    count: usize,
    disabled: Vec<usize>,
}

#[cfg(feature = "radio")]
impl RadioState {
    /// Nothing selected.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            roving: RovingFocus::new(Orientation::Vertical),
            selected: None,
            count: 0,
            disabled: Vec::new(),
        }
    }

    /// Records the group's options and which are disabled.
    pub fn sync(&mut self, group: &crate::RadioGroup) {
        self.count = group.len();
        self.disabled.clear();
        self.disabled
            .extend((0..self.count).filter(|index| !group.is_enabled(*index)));
        let disabled = &self.disabled;
        self.roving
            .sync(self.count, |index| !disabled.contains(&index));
        if self
            .selected
            .is_some_and(|index| index >= self.count || disabled.contains(&index))
        {
            self.selected = None;
        }
    }

    /// The selected option.
    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The focused option.
    #[must_use]
    pub const fn focused(&self) -> Option<usize> {
        self.roving.current()
    }

    /// Selects and focuses `index` when it is an enabled option.
    pub fn select(&mut self, index: usize) {
        if index < self.count && !self.disabled.contains(&index) {
            self.selected = Some(index);
            self.roving.set(Some(index));
        }
    }
}

#[cfg(feature = "radio")]
impl Handle for RadioState {
    fn handle(&mut self, event: &Event) -> Outcome {
        let Some(key) = key_of(event) else {
            return Outcome::Ignored;
        };
        if plain(key, KeyCode::Char(' ')) {
            let Some(index) = self.roving.current() else {
                return Outcome::Handled;
            };
            let changed = self.selected != Some(index);
            self.selected = Some(index);
            return if changed {
                Outcome::Changed
            } else {
                Outcome::Handled
            };
        }
        let forward = if plain(key, KeyCode::Down) || plain(key, KeyCode::Right) {
            true
        } else if plain(key, KeyCode::Up) || plain(key, KeyCode::Left) {
            false
        } else {
            return Outcome::Ignored;
        };
        let disabled = &self.disabled;
        let start = self.roving.current().or(self.selected);
        let next = RovingFocus::step(start, self.count, forward, true, |index| {
            !disabled.contains(&index)
        });
        let Some(next) = next else {
            return Outcome::Handled;
        };
        self.roving.set(Some(next));
        if self.selected == Some(next) {
            Outcome::Handled
        } else {
            self.selected = Some(next);
            Outcome::Changed
        }
    }
}

/// Keyboard state for a [`Select`](crate::Select): the chosen option and the drop-down list.
///
/// While the list is closed, Up and Down change the choice, typing jumps to the option that starts
/// with what was typed, and Enter, Space or Alt+Down answer [`Outcome::Activated`] for the app to
/// [`open`](Self::open) the list. While it is open, the keys move the active row, Enter chooses it
/// ([`Outcome::Toggled`]) and the app [`close`](Self::close)s the list. Esc and outside presses
/// close it through the app's [`LayerStack`]; [`sync_layer`](Self::sync_layer) reports that as
/// [`Outcome::Dismissed`]. Type-ahead time comes from [`set_time`](Self::set_time), usually a
/// [`Clock`](lunatui_runtime::Clock).
#[cfg(feature = "select")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectState {
    chosen: Option<usize>,
    active: RovingFocus,
    labels: Vec<String>,
    typeahead: Typeahead,
    now: Duration,
    list: Dismissal,
}

#[cfg(feature = "select")]
impl SelectState {
    /// Nothing chosen and the list closed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            chosen: None,
            active: RovingFocus::new(Orientation::Vertical).wrap(false),
            labels: Vec::new(),
            typeahead: Typeahead::new(),
            now: Duration::ZERO,
            list: Dismissal::new(),
        }
    }

    /// Records the options, for stepping and type-ahead.
    pub fn sync<S: AsRef<str>>(&mut self, options: &[S]) {
        let same = self.labels.len() == options.len()
            && self
                .labels
                .iter()
                .zip(options)
                .all(|(held, option)| held == option.as_ref());
        if !same {
            self.labels = options
                .iter()
                .map(|option| option.as_ref().to_owned())
                .collect();
        }
        let count = self.labels.len();
        self.chosen = self.chosen.filter(|index| *index < count);
        self.active.sync(count, |_| true);
    }

    /// The current time, for the type-ahead timeout.
    pub fn set_time(&mut self, now: Duration) {
        self.now = now;
    }

    /// The chosen option.
    #[must_use]
    pub const fn chosen(&self) -> Option<usize> {
        self.chosen
    }

    /// The active row of the open list.
    #[must_use]
    pub const fn active(&self) -> Option<usize> {
        self.active.current()
    }

    /// Whether the list is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.list.is_open()
    }

    /// Opens the list as `layer`, from [`Select::layer`](crate::Select::layer), with the chosen
    /// option active.
    pub fn open(&mut self, layers: &mut LayerStack, layer: Layer) {
        self.active
            .set(self.chosen.or((!self.labels.is_empty()).then_some(0)));
        self.typeahead.clear();
        self.list.open(layers, layer);
    }

    /// Closes the list and gives focus back to the field.
    pub fn close(&mut self, layers: &mut LayerStack, focus: &mut FocusManager) {
        self.list.close(layers, focus);
    }

    /// Records the cells the open list drew this frame, from
    /// [`Select::list_area`](crate::Select::list_area).
    pub fn place(&self, layers: &mut LayerStack, list: lunatui_core::Rect) {
        self.list.place(layers, list);
    }

    /// [`Outcome::Dismissed`] once after the stack closed the list.
    pub fn sync_layer(&mut self, layers: &LayerStack) -> Outcome {
        self.list.sync(layers)
    }

    fn typed(&mut self, key: &KeyEvent, current: Option<usize>) -> Typed {
        let labels = &self.labels;
        self.typeahead
            .handle_key(key, self.now, current, labels.len(), |index| {
                labels[index].as_str()
            })
    }
}

#[cfg(feature = "select")]
impl Handle for SelectState {
    fn handle(&mut self, event: &Event) -> Outcome {
        let Some(key) = key_of(event) else {
            return Outcome::Ignored;
        };
        let count = self.labels.len();
        if self.list.is_open() {
            if plain(key, KeyCode::Enter)
                || plain(key, KeyCode::Char(' ')) && self.typeahead.query().is_empty()
            {
                let Some(row) = self.active.current() else {
                    return Outcome::Handled;
                };
                self.chosen = Some(row);
                return Outcome::Toggled(row);
            }
            match self.typed(key, self.active.current()) {
                Typed::Found(row) if Some(row) != self.active.current() => {
                    self.active.set(Some(row));
                    return Outcome::Changed;
                }
                Typed::Found(_) | Typed::NotFound => return Outcome::Handled,
                Typed::Ignored => {}
            }
            return self.active.handle_key(key, count, |_| true);
        }
        let opens = plain(key, KeyCode::Enter)
            || plain(key, KeyCode::Char(' '))
            || key.code == KeyCode::Down && key.modifiers == Modifiers::new().alt();
        if opens {
            return Outcome::Activated(self.chosen.unwrap_or(0));
        }
        let next = match self.typed(key, self.chosen) {
            Typed::Found(index) => Some(index),
            Typed::NotFound => None,
            Typed::Ignored if plain(key, KeyCode::Down) || plain(key, KeyCode::Up) => {
                RovingFocus::step(self.chosen, count, key.code == KeyCode::Down, false, |_| {
                    true
                })
            }
            Typed::Ignored => return Outcome::Ignored,
        };
        match next {
            Some(index) if Some(index) != self.chosen => {
                self.chosen = Some(index);
                Outcome::Changed
            }
            _ => Outcome::Handled,
        }
    }
}

/// Keyboard state for a [`FilePicker`](crate::FilePicker): the focused row, the marked rows and
/// type-ahead over the names.
///
/// Up and Down move and stop at the ends, Page Up and Page Down move a page, Home and End jump,
/// typing jumps to the next name that starts with what was typed, Space marks or unmarks the
/// focused row ([`Outcome::Toggled`]) and Enter opens it ([`Outcome::Activated`]). Type-ahead time
/// comes from [`set_time`](Self::set_time).
#[cfg(feature = "file-picker")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePickerState {
    roving: RovingFocus,
    marked: Selection,
    names: Vec<String>,
    typeahead: Typeahead,
    now: Duration,
    page: usize,
}

#[cfg(feature = "file-picker")]
impl Default for FilePickerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "file-picker")]
impl FilePickerState {
    /// No entries yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            roving: RovingFocus::new(Orientation::Vertical).wrap(false),
            marked: Selection::new(SelectionMode::Multiple),
            names: Vec::new(),
            typeahead: Typeahead::new(),
            now: Duration::ZERO,
            page: 1,
        }
    }

    /// Records the folder's entries and how many rows fit on screen. Moving to another folder
    /// should start from a fresh state.
    pub fn sync(&mut self, entries: &[crate::FileEntry<'_>], visible_rows: usize) {
        let same = self.names.len() == entries.len()
            && self
                .names
                .iter()
                .zip(entries)
                .all(|(held, entry)| held == entry.name());
        if !same {
            self.names = entries
                .iter()
                .map(|entry| entry.name().to_owned())
                .collect();
        }
        self.page = visible_rows.max(1);
        self.roving.sync(self.names.len(), |_| true);
        self.marked.retain_below(self.names.len());
    }

    /// The current time, for the type-ahead timeout.
    pub fn set_time(&mut self, now: Duration) {
        self.now = now;
    }

    /// The focused row.
    #[must_use]
    pub const fn focused(&self) -> Option<usize> {
        self.roving.current()
    }

    /// The marked rows, lowest first.
    #[must_use]
    pub fn marked(&self) -> &[usize] {
        self.marked.chosen()
    }
}

#[cfg(feature = "file-picker")]
impl Handle for FilePickerState {
    fn handle(&mut self, event: &Event) -> Outcome {
        let Some(key) = key_of(event) else {
            return Outcome::Ignored;
        };
        let count = self.names.len();
        let before = self.roving.current();
        if plain(key, KeyCode::Enter) {
            return before.map_or(Outcome::Handled, Outcome::Activated);
        }
        if plain(key, KeyCode::Char(' ')) && self.typeahead.query().is_empty() {
            let Some(row) = before else {
                return Outcome::Handled;
            };
            self.marked.toggle(row);
            return Outcome::Toggled(row);
        }
        let names = &self.names;
        let typed = self
            .typeahead
            .handle_key(key, self.now, before, count, |index| names[index].as_str());
        if let Typed::Found(row) = typed {
            self.roving.set(Some(row));
        } else if typed == Typed::NotFound {
            return Outcome::Handled;
        } else if plain(key, KeyCode::PageDown) || plain(key, KeyCode::PageUp) {
            let page = isize::try_from(self.page).unwrap_or(isize::MAX);
            let delta = if key.code == KeyCode::PageDown {
                page
            } else {
                -page
            };
            self.roving.move_by(count, delta, |_| true);
        } else {
            return self.roving.handle_key(key, count, |_| true);
        }
        if self.roving.current() == before {
            Outcome::Handled
        } else {
            Outcome::Changed
        }
    }
}
