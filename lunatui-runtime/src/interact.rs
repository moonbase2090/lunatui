// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::time::Duration;

use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers};

use crate::{FocusDirection, FocusManager, Layer, LayerSlot, LayerStack};

/// How long a pause ends a [`Typeahead`] burst: the next key starts a new query.
pub const TYPEAHEAD_TIMEOUT: Duration = Duration::from_secs(1);

/// What a widget state did with an event, from [`Handle::handle`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Not for this state: the app can use the event.
    Ignored,
    /// Taken, with nothing to redraw, such as an arrow at the end of a list that does not wrap.
    Handled,
    /// Taken, and the state changed: redraw.
    Changed,
    /// Enter on the item at this index.
    Activated(usize),
    /// Space toggled or chose the item at this index.
    Toggled(usize),
    /// The state's layer closed, on Esc or a press outside it.
    Dismissed,
}

impl Outcome {
    /// Whether the state took the event.
    #[must_use]
    pub const fn is_taken(self) -> bool {
        !matches!(self, Self::Ignored)
    }
}

/// The widget input contract: a state handles its own keys.
///
/// Rendering stays immediate-mode. The app keeps the state between frames, passes it each event
/// and acts on the [`Outcome`]; a state answers [`Outcome::Ignored`] for anything it does not use.
pub trait Handle {
    /// Applies `event` and reports what changed.
    fn handle(&mut self, event: &Event) -> Outcome;
}

/// Which arrow keys move a [`RovingFocus`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Up and Down, as in a list.
    #[default]
    Vertical,
    /// Left and Right, as in a tab bar.
    Horizontal,
}

/// The one focused item in a group, moved with the arrow keys, Home and End.
///
/// Every widget steps through its items with [`RovingFocus::step_by`], which skips disabled
/// items and either wraps around or stops at the ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RovingFocus {
    current: Option<usize>,
    orientation: Orientation,
    clamp: bool,
}

impl RovingFocus {
    /// Nothing focused yet; arrows along `orientation` move and wrap around.
    #[must_use]
    pub const fn new(orientation: Orientation) -> Self {
        Self {
            current: None,
            orientation,
            clamp: false,
        }
    }

    /// Whether moving past the last item wraps to the first (the default) or stops.
    #[must_use]
    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.clamp = !wrap;
        self
    }

    /// The focused item.
    #[must_use]
    pub const fn current(&self) -> Option<usize> {
        self.current
    }

    /// Focuses `index`, or nothing.
    pub fn set(&mut self, index: Option<usize>) {
        self.current = index;
    }

    /// The item `delta` places from `from` among `count`, skipping items `enabled` rejects. With
    /// `wrap` the move runs around the ends; without it the move stops at the last enabled item in
    /// its direction. From `None`, a forward move gives the first enabled item and a backward one
    /// the last. `None` when no item is enabled.
    #[must_use]
    pub fn step_by(
        from: Option<usize>,
        count: usize,
        delta: isize,
        wrap: bool,
        enabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let forward = !delta.is_negative();
        let Some(from) = from.filter(|index| *index < count) else {
            return if forward {
                (0..count).find(|index| enabled(*index))
            } else {
                (0..count).rev().find(|index| enabled(*index))
            };
        };
        let distance = delta.unsigned_abs().max(1);
        if wrap {
            let shift = distance % count;
            let target = if forward {
                (from + shift) % count
            } else {
                (from + count - shift) % count
            };
            return (0..count)
                .map(|offset| {
                    if forward {
                        (target + offset) % count
                    } else {
                        (target + count - offset) % count
                    }
                })
                .find(|index| enabled(*index));
        }
        let last = count - 1;
        let target = if forward {
            from.saturating_add(distance).min(last)
        } else {
            from.saturating_sub(distance)
        };
        let ahead = if forward {
            (target..=last).find(|index| enabled(*index))
        } else {
            (0..=target).rev().find(|index| enabled(*index))
        };
        ahead
            .or_else(|| {
                if forward {
                    (from + 1..target).rev().find(|index| enabled(*index))
                } else {
                    (target + 1..from).find(|index| enabled(*index))
                }
            })
            .or_else(|| enabled(from).then_some(from))
    }

    /// [`step_by`](Self::step_by) one place forward or back.
    #[must_use]
    pub fn step(
        from: Option<usize>,
        count: usize,
        forward: bool,
        wrap: bool,
        enabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        Self::step_by(from, count, if forward { 1 } else { -1 }, wrap, enabled)
    }

    /// Moves focus `delta` places among `count` items. Returns whether focus moved.
    pub fn move_by(&mut self, count: usize, delta: isize, enabled: impl Fn(usize) -> bool) -> bool {
        let next = Self::step_by(self.current, count, delta, !self.clamp, enabled);
        let moved = next != self.current;
        self.current = next;
        moved
    }

    /// Keeps focus on an item that still exists and is enabled, after the items changed: the
    /// nearest enabled item at or before the old one, else the first enabled item.
    pub fn sync(&mut self, count: usize, enabled: impl Fn(usize) -> bool) {
        let Some(index) = self.current else {
            return;
        };
        if index < count && enabled(index) {
            return;
        }
        let near = index.min(count.saturating_sub(1));
        self.current = (0..count.min(near + 1))
            .rev()
            .find(|candidate| enabled(*candidate))
            .or_else(|| (0..count).find(|candidate| enabled(*candidate)));
    }

    /// Applies an arrow, Home or End key to `count` items. Other keys are ignored.
    pub fn handle_key(
        &mut self,
        key: &KeyEvent,
        count: usize,
        enabled: impl Fn(usize) -> bool,
    ) -> Outcome {
        if !is_plain_press(key) {
            return Outcome::Ignored;
        }
        let (back, ahead) = match self.orientation {
            Orientation::Vertical => (KeyCode::Up, KeyCode::Down),
            Orientation::Horizontal => (KeyCode::Left, KeyCode::Right),
        };
        let before = self.current;
        match key.code {
            code if code == back => {
                self.move_by(count, -1, enabled);
            }
            code if code == ahead => {
                self.move_by(count, 1, enabled);
            }
            KeyCode::Home => self.current = (0..count).find(|index| enabled(*index)),
            KeyCode::End => self.current = (0..count).rev().find(|index| enabled(*index)),
            _ => return Outcome::Ignored,
        }
        if self.current == before {
            Outcome::Handled
        } else {
            Outcome::Changed
        }
    }
}

/// What [`Typeahead::handle_key`] made of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    /// Not a printable key: the query is unchanged.
    Ignored,
    /// The query now picks this item.
    Found(usize),
    /// The key joined the query, but no item starts with it.
    NotFound,
}

/// Jumps to the item whose label starts with what was just typed.
///
/// Keys typed within [`TYPEAHEAD_TIMEOUT`] of each other build one query; a longer pause starts a
/// new one. Time comes from the caller, usually a [`Clock`](crate::Clock), so tests stay exact.
/// Typing the same letter again moves on to the next item with that letter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typeahead {
    query: String,
    last: Option<Duration>,
    timeout: Duration,
}

impl Default for Typeahead {
    fn default() -> Self {
        Self::new()
    }
}

impl Typeahead {
    /// An empty query with the default timeout.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            query: String::new(),
            last: None,
            timeout: TYPEAHEAD_TIMEOUT,
        }
    }

    /// The pause that starts a new query.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The query typed so far.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Forgets the query.
    pub fn clear(&mut self) {
        self.query.clear();
        self.last = None;
    }

    /// Adds `ch` typed at time `now`, starting a new query after a pause.
    pub fn push(&mut self, ch: char, now: Duration) {
        if self
            .last
            .is_none_or(|last| now.saturating_sub(last) >= self.timeout)
        {
            self.query.clear();
        }
        self.query.push(ch);
        self.last = Some(now);
    }

    /// The item among `count` that the query picks, given the focused `current`. `label` gives
    /// each item's text. A one-letter query, or the same letter repeated, moves past `current`;
    /// a longer query keeps `current` while it still matches.
    #[must_use]
    pub fn find<L: AsRef<str>>(
        &self,
        current: Option<usize>,
        count: usize,
        label: impl Fn(usize) -> L,
    ) -> Option<usize> {
        let mut letters = self.query.chars();
        let first = letters.next()?;
        let repeated = letters.all(|ch| ch.to_lowercase().eq(first.to_lowercase()));
        let mut buffer = [0_u8; 4];
        let prefix: &str = if repeated {
            first.encode_utf8(&mut buffer)
        } else {
            &self.query
        };
        let current = current.filter(|index| *index < count);
        let start = match current {
            Some(index) if repeated => index + 1,
            Some(index) => index,
            None => 0,
        };
        (0..count)
            .map(|offset| (start + offset) % count)
            .find(|index| starts_with_ignoring_case(label(*index).as_ref(), prefix))
    }

    /// Takes a printable key typed at `now` and returns the item it picks among `count`. Other
    /// keys are [`Typed::Ignored`] and leave the query alone.
    pub fn handle_key<L: AsRef<str>>(
        &mut self,
        key: &KeyEvent,
        now: Duration,
        current: Option<usize>,
        count: usize,
        label: impl Fn(usize) -> L,
    ) -> Typed {
        let KeyCode::Char(ch) = key.code else {
            return Typed::Ignored;
        };
        let shifted_only =
            key.modifiers == Modifiers::new() || key.modifiers == Modifiers::new().shift();
        if !is_press(key)
            || !shifted_only
            || ch.is_control()
            || (ch == ' ' && self.query.is_empty())
        {
            return Typed::Ignored;
        }
        self.push(ch, now);
        self.find(current, count, label)
            .map_or(Typed::NotFound, Typed::Found)
    }
}

/// Whether `text` starts with `prefix`, ignoring case.
#[must_use]
pub fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    let mut text = text.chars().flat_map(char::to_lowercase);
    prefix
        .chars()
        .flat_map(char::to_lowercase)
        .all(|ch| text.next() == Some(ch))
}

/// Whether a [`Selection`] holds one item or many.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// Choosing an item replaces the choice.
    #[default]
    Single,
    /// Choosing an item adds it or takes it out.
    Multiple,
}

/// The chosen items of a list, kept sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    mode: SelectionMode,
    chosen: Vec<usize>,
}

impl Selection {
    /// Nothing chosen.
    #[must_use]
    pub const fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            chosen: Vec::new(),
        }
    }

    /// Single or multiple.
    #[must_use]
    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// The chosen items, lowest first.
    #[must_use]
    pub fn chosen(&self) -> &[usize] {
        &self.chosen
    }

    /// The lowest chosen item.
    #[must_use]
    pub fn first(&self) -> Option<usize> {
        self.chosen.first().copied()
    }

    /// Whether `index` is chosen.
    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        self.chosen.binary_search(&index).is_ok()
    }

    /// Chooses `index`: the only choice in single mode, one more in multiple mode.
    pub fn choose(&mut self, index: usize) {
        match self.mode {
            SelectionMode::Single => {
                self.chosen.clear();
                self.chosen.push(index);
            }
            SelectionMode::Multiple => {
                if let Err(at) = self.chosen.binary_search(&index) {
                    self.chosen.insert(at, index);
                }
            }
        }
    }

    /// Takes `index` out when chosen and chooses it otherwise. In single mode, toggling the
    /// chosen item clears the choice.
    pub fn toggle(&mut self, index: usize) {
        match self.chosen.binary_search(&index) {
            Ok(at) => {
                self.chosen.remove(at);
            }
            Err(_) => self.choose(index),
        }
    }

    /// Drops every choice.
    pub fn clear(&mut self) {
        self.chosen.clear();
    }

    /// Drops choices at or past `count`, after the items shrank.
    pub fn retain_below(&mut self, count: usize) {
        self.chosen.retain(|index| *index < count);
    }
}

/// A widget's overlay in the app's [`LayerStack`], such as a select's drop-down.
///
/// The stack alone handles Esc and outside presses. A state holds a `Dismissal`, opens its layer
/// with it, and turns the stack closing that layer into [`Outcome::Dismissed`] with
/// [`sync`](Self::sync).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dismissal {
    slot: LayerSlot,
}

impl Dismissal {
    /// Closed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slot: LayerSlot::new(),
        }
    }

    /// Opens `layer` above every other layer, unless it is open already.
    pub fn open(&mut self, layers: &mut LayerStack, layer: Layer) {
        self.slot.open(layers, layer);
    }

    /// Closes the layer and gives focus back to its opener.
    pub fn close(&mut self, layers: &mut LayerStack, focus: &mut FocusManager) {
        self.slot.close(layers, focus);
    }

    /// Whether the layer is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.slot.is_open()
    }

    /// Records the cells the overlay drew this frame.
    pub fn place(&self, layers: &mut LayerStack, area: lunatui_core::Rect) {
        self.slot.place(layers, area);
    }

    /// [`Outcome::Dismissed`] once after the stack closed the layer, and
    /// [`Outcome::Ignored`] otherwise.
    pub fn sync(&mut self, layers: &LayerStack) -> Outcome {
        if self.slot.sync(layers) {
            Outcome::Dismissed
        } else {
            Outcome::Ignored
        }
    }
}

impl Handle for FocusManager {
    fn handle(&mut self, event: &Event) -> Outcome {
        let Event::Key(key) = event else {
            return Outcome::Ignored;
        };
        if !is_press(key) {
            return Outcome::Ignored;
        }
        let before = self.focused();
        let plain = key.modifiers == Modifiers::new();
        match key.code {
            KeyCode::Tab if plain => self.tab(false),
            KeyCode::Tab if key.modifiers == Modifiers::new().shift() => self.tab(true),
            KeyCode::Up if plain => self.arrow(FocusDirection::Up),
            KeyCode::Down if plain => self.arrow(FocusDirection::Down),
            KeyCode::Left if plain => self.arrow(FocusDirection::Left),
            KeyCode::Right if plain => self.arrow(FocusDirection::Right),
            _ => return Outcome::Ignored,
        }
        if self.focused() == before {
            Outcome::Handled
        } else {
            Outcome::Changed
        }
    }
}

fn is_press(key: &KeyEvent) -> bool {
    matches!(key.kind, KeyKind::Press | KeyKind::Repeat)
}

fn is_plain_press(key: &KeyEvent) -> bool {
    is_press(key) && key.modifiers == Modifiers::new()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lunatui_core::Rect;
    use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers};

    use super::{
        Dismissal, Handle, Orientation, Outcome, RovingFocus, Selection, SelectionMode,
        TYPEAHEAD_TIMEOUT, Typeahead, Typed,
    };
    use crate::{Clock, FocusManager, HitId, Layer, LayerStack};

    const FRUIT: [&str; 5] = ["apple", "apricot", "banana", "blueberry", "cherry"];

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::press(code)
    }

    fn all(_: usize) -> bool {
        true
    }

    #[test]
    fn stepping_wraps_and_skips_disabled_items() {
        let enabled = |index: usize| index != 1 && index != 4;
        assert_eq!(RovingFocus::step(Some(0), 5, true, true, enabled), Some(2));
        assert_eq!(RovingFocus::step(Some(3), 5, true, true, enabled), Some(0));
        assert_eq!(RovingFocus::step(Some(0), 5, false, true, enabled), Some(3));
        assert_eq!(RovingFocus::step(None, 5, true, true, enabled), Some(0));
        assert_eq!(RovingFocus::step(None, 5, false, true, enabled), Some(3));
        assert_eq!(RovingFocus::step(Some(2), 5, true, true, |_| false), None);
        assert_eq!(RovingFocus::step(Some(2), 0, true, true, all), None);
    }

    #[test]
    fn stepping_without_wrap_stops_at_the_last_enabled_item() {
        let enabled = |index: usize| index != 4;
        assert_eq!(RovingFocus::step(Some(3), 5, true, false, enabled), Some(3));
        assert_eq!(RovingFocus::step(Some(0), 5, false, false, all), Some(0));
        assert_eq!(
            RovingFocus::step_by(Some(1), 5, 10, false, enabled),
            Some(3)
        );
        assert_eq!(RovingFocus::step_by(Some(4), 5, -3, false, all), Some(1));
        assert_eq!(RovingFocus::step_by(Some(2), 5, 2, true, all), Some(4));
    }

    #[test]
    fn arrows_move_along_the_orientation_only() {
        let mut list = RovingFocus::new(Orientation::Vertical);
        assert_eq!(
            list.handle_key(&key(KeyCode::Down), 3, all),
            Outcome::Changed
        );
        assert_eq!(list.current(), Some(0));
        assert_eq!(
            list.handle_key(&key(KeyCode::Right), 3, all),
            Outcome::Ignored
        );
        assert_eq!(
            list.handle_key(&key(KeyCode::End), 3, all),
            Outcome::Changed
        );
        assert_eq!(list.current(), Some(2));
        assert_eq!(
            list.handle_key(&key(KeyCode::Down), 3, all),
            Outcome::Changed
        );
        assert_eq!(list.current(), Some(0), "wraps by default");

        let mut tabs = RovingFocus::new(Orientation::Horizontal).wrap(false);
        tabs.set(Some(2));
        assert_eq!(
            tabs.handle_key(&key(KeyCode::Right), 3, all),
            Outcome::Handled
        );
        assert_eq!(
            tabs.handle_key(&key(KeyCode::Left), 3, all),
            Outcome::Changed
        );
        assert_eq!(tabs.current(), Some(1));
        assert_eq!(
            tabs.handle_key(&key(KeyCode::Home), 3, all),
            Outcome::Changed
        );
        assert_eq!(tabs.current(), Some(0));
        assert_eq!(
            tabs.handle_key(&key(KeyCode::Down), 3, all),
            Outcome::Ignored
        );
    }

    #[test]
    fn released_or_modified_arrows_are_ignored() {
        let mut list = RovingFocus::new(Orientation::Vertical);
        let release = key(KeyCode::Down).kind(KeyKind::Release);
        let shifted = key(KeyCode::Down).modifiers(Modifiers::new().shift());
        assert_eq!(list.handle_key(&release, 3, all), Outcome::Ignored);
        assert_eq!(list.handle_key(&shifted, 3, all), Outcome::Ignored);
        assert_eq!(list.current(), None);
    }

    #[test]
    fn sync_moves_focus_off_items_that_left() {
        let mut list = RovingFocus::new(Orientation::Vertical);
        list.set(Some(4));
        list.sync(3, all);
        assert_eq!(list.current(), Some(2));
        list.sync(3, |index| index != 2);
        assert_eq!(list.current(), Some(1));
        list.sync(0, all);
        assert_eq!(list.current(), None);
    }

    #[test]
    fn typeahead_builds_a_query_within_the_timeout_and_restarts_after_it() {
        let mut clock = Clock::new();
        let mut typed = Typeahead::new();
        let label = |index: usize| FRUIT[index];
        let pick = |typed: &mut Typeahead, ch: char, now, current| {
            typed.handle_key(&key(KeyCode::Char(ch)), now, current, FRUIT.len(), label)
        };
        assert_eq!(pick(&mut typed, 'b', clock.now(), None), Typed::Found(2));
        clock.advance(Duration::from_millis(300));
        assert_eq!(pick(&mut typed, 'l', clock.now(), Some(2)), Typed::Found(3));
        assert_eq!(typed.query(), "bl");
        clock.advance(TYPEAHEAD_TIMEOUT);
        assert_eq!(pick(&mut typed, 'a', clock.now(), Some(3)), Typed::Found(0));
        assert_eq!(typed.query(), "a", "a pause starts a new query");
        clock.advance(TYPEAHEAD_TIMEOUT.saturating_sub(Duration::from_millis(1)));
        assert_eq!(pick(&mut typed, 'p', clock.now(), Some(0)), Typed::Found(0));
        assert_eq!(typed.query(), "ap");
        assert_eq!(pick(&mut typed, 'r', clock.now(), Some(0)), Typed::Found(1));
        assert_eq!(pick(&mut typed, 'z', clock.now(), Some(1)), Typed::NotFound);
    }

    #[test]
    fn typing_one_letter_again_cycles_through_its_items() {
        let mut typed = Typeahead::new();
        let now = Duration::ZERO;
        let label = |index: usize| FRUIT[index];
        let first = typed.handle_key(&key(KeyCode::Char('B')), now, Some(0), 5, label);
        assert_eq!(first, Typed::Found(2), "case is ignored");
        let second = typed.handle_key(&key(KeyCode::Char('b')), now, Some(2), 5, label);
        assert_eq!(second, Typed::Found(3));
        let third = typed.handle_key(&key(KeyCode::Char('b')), now, Some(3), 5, label);
        assert_eq!(third, Typed::Found(2));
    }

    #[test]
    fn repeating_an_accented_letter_in_either_case_cycles_through_its_items() {
        const WORDS: [&str; 3] = ["Éclair", "apple", "École"];
        let mut typed = Typeahead::new();
        let now = Duration::ZERO;
        let label = |index: usize| WORDS[index];
        let first = typed.handle_key(&key(KeyCode::Char('É')), now, None, 3, label);
        assert_eq!(first, Typed::Found(0));
        let second = typed.handle_key(&key(KeyCode::Char('é')), now, Some(0), 3, label);
        assert_eq!(second, Typed::Found(2));
        let third = typed.handle_key(&key(KeyCode::Char('É')), now, Some(2), 3, label);
        assert_eq!(third, Typed::Found(0));
    }

    #[test]
    fn typeahead_leaves_other_keys_alone() {
        let mut typed = Typeahead::new();
        let label = |index: usize| FRUIT[index];
        let ctrl = key(KeyCode::Char('a')).modifiers(Modifiers::new().control());
        assert_eq!(
            typed.handle_key(&ctrl, Duration::ZERO, None, 5, label),
            Typed::Ignored
        );
        let space = key(KeyCode::Char(' '));
        assert_eq!(
            typed.handle_key(&space, Duration::ZERO, None, 5, label),
            Typed::Ignored
        );
        assert_eq!(
            typed.handle_key(&key(KeyCode::Down), Duration::ZERO, None, 5, label),
            Typed::Ignored
        );
        assert_eq!(typed.query(), "");
    }

    #[test]
    fn single_selection_replaces_and_multiple_accumulates() {
        let mut single = Selection::new(SelectionMode::Single);
        single.choose(3);
        single.choose(1);
        assert_eq!(single.chosen(), &[1]);
        single.toggle(1);
        assert_eq!(single.first(), None);

        let mut many = Selection::new(SelectionMode::Multiple);
        many.toggle(4);
        many.toggle(1);
        many.choose(4);
        assert_eq!(many.chosen(), &[1, 4]);
        many.toggle(4);
        assert!(!many.contains(4));
        many.choose(7);
        many.retain_below(5);
        assert_eq!(many.chosen(), &[1]);
    }

    #[test]
    fn focus_manager_takes_tab_and_arrows_and_ignores_the_rest() {
        let mut focus = FocusManager::new();
        focus.sync(vec![
            (HitId(1), Rect::new(0, 0, 4, 1)),
            (HitId(2), Rect::new(6, 0, 4, 1)),
        ]);
        let tab = Event::Key(key(KeyCode::Tab));
        let back = Event::Key(key(KeyCode::Tab).modifiers(Modifiers::new().shift()));
        assert_eq!(focus.handle(&tab), Outcome::Changed);
        assert_eq!(focus.focused(), Some(HitId(2)));
        assert_eq!(focus.handle(&back), Outcome::Changed);
        assert_eq!(
            focus.handle(&Event::Key(key(KeyCode::Right))),
            Outcome::Changed
        );
        assert_eq!(focus.focused(), Some(HitId(2)));
        assert_eq!(
            focus.handle(&Event::Key(key(KeyCode::Right))),
            Outcome::Handled
        );
        assert_eq!(
            focus.handle(&Event::Key(key(KeyCode::Char('x')))),
            Outcome::Ignored
        );
    }

    #[test]
    fn dismissal_reports_the_stack_closing_its_layer_once() {
        let mut layers = LayerStack::new();
        let mut focus = FocusManager::new();
        let mut list = Dismissal::new();
        list.open(&mut layers, Layer::popover(Rect::new(0, 1, 10, 4)));
        assert_eq!(list.sync(&layers), Outcome::Ignored);
        layers.handle(&Event::Key(key(KeyCode::Esc)), &mut focus);
        assert_eq!(list.sync(&layers), Outcome::Dismissed);
        assert_eq!(list.sync(&layers), Outcome::Ignored);
        assert!(!list.is_open());
    }
}
