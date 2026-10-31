// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Rect, Size, Style, UnderlineStyle, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{Block, Borders, RenderCx, Widget, WidgetState};

/// An action emitted when a leaf menu item is selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuEvent<T> {
    /// The payload attached to the selected leaf.
    Selected(T),
}

/// A selectable leaf or a group that opens a nested menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem<T> {
    label: Cow<'static, str>,
    data: Option<T>,
    children: Vec<Self>,
    disabled: bool,
    mnemonic: Option<char>,
}

impl<T> MenuItem<T> {
    /// Creates a leaf item with an action payload.
    #[must_use]
    pub fn item(label: impl Into<Cow<'static, str>>, data: T) -> Self {
        Self {
            label: label.into(),
            data: Some(data),
            children: Vec::new(),
            disabled: false,
            mnemonic: None,
        }
    }

    /// Creates a group whose children open in a nested panel.
    #[must_use]
    pub fn group(label: impl Into<Cow<'static, str>>, children: Vec<Self>) -> Self {
        Self {
            label: label.into(),
            data: None,
            children,
            disabled: false,
            mnemonic: None,
        }
    }

    /// Disables this item and prevents navigation into or selection of it.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The key that chooses this item through [`MenuState::press_mnemonic`], matched without
    /// regard to case. The first letter of the label that matches is underlined.
    #[must_use]
    pub const fn mnemonic(mut self, key: char) -> Self {
        self.mnemonic = Some(key);
        self
    }

    /// The item's mnemonic key, if it has one.
    #[must_use]
    pub const fn mnemonic_key(&self) -> Option<char> {
        self.mnemonic
    }

    /// The displayed label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether this item opens a nested menu.
    #[must_use]
    pub fn is_group(&self) -> bool {
        !self.children.is_empty()
    }

    /// Whether this item is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// The action payload, if this is a leaf item.
    #[must_use]
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Child menu items in display order.
    #[must_use]
    pub fn children(&self) -> &[Self] {
        &self.children
    }
}

/// Application-owned interaction state and action queue for [`Menu`].
///
/// Store this across frames, call its navigation methods from the app's keymap, render it through
/// [`Menu::new`], and consume completed actions with [`drain_events`](Self::drain_events).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuState<T> {
    items: Vec<MenuItem<T>>,
    selected_path: Vec<usize>,
    events: Vec<MenuEvent<T>>,
}

impl<T> MenuState<T> {
    /// Builds a menu state from top-level items. It starts inactive.
    #[must_use]
    pub fn new(items: Vec<MenuItem<T>>) -> Self {
        Self {
            items,
            selected_path: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Top-level items in display order.
    #[must_use]
    pub fn items(&self) -> &[MenuItem<T>] {
        &self.items
    }

    /// Whether a top-level item is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.selected_path.is_empty()
    }

    /// The selected item's index at each nesting level.
    #[must_use]
    pub fn selected_path(&self) -> &[usize] {
        &self.selected_path
    }

    /// The currently highlighted item, including an item inside an open submenu.
    #[must_use]
    pub fn selected(&self) -> Option<&MenuItem<T>> {
        item_at_path(&self.items, &self.selected_path)
    }

    /// Activates the first enabled top-level item.
    pub fn activate(&mut self) -> bool {
        if self.is_active() {
            return false;
        }
        let Some(index) = first_enabled(&self.items) else {
            return false;
        };
        self.selected_path.push(index);
        true
    }

    /// Moves to the previous enabled item at this level.
    pub fn up(&mut self) -> bool {
        match self.selected_path.len() {
            0 | 1 => false,
            2 if self.previous_sibling_is_absent() => self.pop(),
            _ => self.move_sibling(false),
        }
    }

    /// Enters the selected top-level group, or moves down within an open submenu.
    pub fn down(&mut self) -> bool {
        if self.selected_path.len() == 1 {
            self.push()
        } else {
            self.move_sibling(true)
        }
    }

    /// Moves to the previous top-level item, closing one open submenu first when needed.
    pub fn left(&mut self) -> bool {
        match self.selected_path.len() {
            0 => false,
            1 => self.move_sibling(false),
            2 => {
                let popped = self.pop();
                self.move_sibling(false) || popped
            }
            _ => self.pop(),
        }
    }

    /// Moves to the next top-level item or opens the selected nested group.
    pub fn right(&mut self) -> bool {
        match self.selected_path.len() {
            0 => false,
            1 => self.move_sibling(true),
            2 => {
                if self.push() {
                    true
                } else {
                    let popped = self.pop();
                    self.move_sibling(true) || popped
                }
            }
            _ => self.push(),
        }
    }

    /// Enters the selected group or emits its leaf action.
    pub fn select(&mut self)
    where
        T: Clone,
    {
        let Some(item) = self.selected() else {
            return;
        };
        if item.disabled {
            return;
        }
        if item.is_group() {
            self.push();
        } else if let Some(data) = &item.data {
            self.events.push(MenuEvent::Selected(data.clone()));
        }
    }

    /// Highlights top-level item `index`, as a click on it in the bar does. Returns whether the
    /// item exists and is enabled.
    pub fn open(&mut self, index: usize) -> bool {
        if self.items.get(index).is_none_or(|item| item.disabled) {
            return false;
        }
        self.selected_path.clear();
        self.selected_path.push(index);
        true
    }

    /// Chooses the enabled item whose mnemonic is `key`, matched without regard to case.
    ///
    /// Keys are looked up in the deepest open dropdown: the one with a selected row, or the
    /// dropdown of a highlighted top-level group, such as after [`open`](Self::open). A key no
    /// item there has is looked up in the bar. When one item has the key, it is chosen as
    /// [`select`](Self::select) would choose it: a group opens and a leaf queues its action.
    /// When several share it, each press moves to the next of them without choosing. Returns
    /// whether an item had the key.
    pub fn press_mnemonic(&mut self, key: char) -> bool
    where
        T: Clone,
    {
        let open_group = self.selected_path.len() == 1
            && self
                .selected()
                .is_some_and(|item| !item.disabled && item.is_group());
        if open_group {
            self.selected_path.push(usize::MAX);
            if self.choose_mnemonic(key) {
                return true;
            }
            self.selected_path.pop();
            let root = self.selected_path.pop();
            if self.choose_mnemonic(key) {
                return true;
            }
            self.selected_path.extend(root);
            return false;
        }
        self.choose_mnemonic(key)
    }

    fn choose_mnemonic(&mut self, key: char) -> bool
    where
        T: Clone,
    {
        let depth = self.selected_path.len().max(1);
        let parent = &self.selected_path[..depth - 1];
        let Some(siblings) = items_at_path(&self.items, parent) else {
            return false;
        };
        let matches = |item: &MenuItem<T>| {
            !item.disabled && item.mnemonic.is_some_and(|own| same_key(own, key))
        };
        let count = siblings.iter().filter(|item| matches(item)).count();
        let start = self
            .selected_path
            .get(depth - 1)
            .copied()
            .filter(|index| *index < siblings.len())
            .map_or(0, |index| index + 1);
        let Some(target) = (0..siblings.len())
            .map(|step| (start + step) % siblings.len())
            .find(|index| matches(&siblings[*index]))
        else {
            return false;
        };
        self.selected_path.truncate(depth - 1);
        self.selected_path.push(target);
        if count == 1 {
            self.select();
        }
        true
    }

    /// Closes every open level while leaving queued actions available to drain.
    pub fn reset(&mut self) -> bool {
        if self.selected_path.is_empty() {
            return false;
        }
        self.selected_path.clear();
        true
    }

    /// Enters the selected group's first enabled child.
    pub fn push(&mut self) -> bool {
        let Some(item) = self.selected() else {
            return false;
        };
        if item.disabled {
            return false;
        }
        let Some(index) = first_enabled(&item.children) else {
            return false;
        };
        self.selected_path.push(index);
        true
    }

    /// Closes the deepest submenu without changing its parent's selection.
    pub fn pop(&mut self) -> bool {
        if self.selected_path.len() <= 1 {
            return false;
        }
        self.selected_path.pop();
        true
    }

    /// Removes and returns all queued actions.
    pub fn drain_events(&mut self) -> impl Iterator<Item = MenuEvent<T>> + '_ {
        std::mem::take(&mut self.events).into_iter()
    }

    fn move_sibling(&mut self, next: bool) -> bool {
        let Some(current) = self.selected_path.last().copied() else {
            return false;
        };
        let parent_path = &self.selected_path[..self.selected_path.len() - 1];
        let Some(siblings) = items_at_path(&self.items, parent_path) else {
            return false;
        };
        let target = if next {
            (current.saturating_add(1)..siblings.len()).find(|index| !siblings[*index].disabled)
        } else {
            (0..current).rev().find(|index| !siblings[*index].disabled)
        };
        let Some(target) = target else {
            return false;
        };
        let Some(selected) = self.selected_path.last_mut() else {
            return false;
        };
        *selected = target;
        true
    }

    fn previous_sibling_is_absent(&self) -> bool {
        let Some(current) = self.selected_path.last().copied() else {
            return true;
        };
        let parent_path = &self.selected_path[..self.selected_path.len() - 1];
        items_at_path(&self.items, parent_path)
            .is_none_or(|siblings| !(0..current).any(|index| !siblings[index].disabled))
    }
}

/// Renders a stateful menu bar and its nested dropdown panels.
#[derive(Clone, Copy, Debug)]
pub struct Menu<'a, T> {
    state: &'a MenuState<T>,
    dropdown_width: u16,
    widget_state: WidgetState,
    mnemonics: bool,
}

impl<'a, T> Menu<'a, T> {
    /// Creates a view over application-owned menu state.
    #[must_use]
    pub const fn new(state: &'a MenuState<T>) -> Self {
        Self {
            state,
            dropdown_width: 20,
            widget_state: WidgetState::Normal,
            mnemonics: true,
        }
    }

    /// Maximum width used when positioning nested panels.
    #[must_use]
    pub const fn dropdown_width(mut self, width: u16) -> Self {
        self.dropdown_width = width;
        self
    }

    /// Focus appearance for the menu.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.widget_state = state;
        self
    }

    /// Whether to underline mnemonic letters, on by default.
    #[must_use]
    pub const fn mnemonics(mut self, show: bool) -> Self {
        self.mnemonics = show;
        self
    }

    /// The top-level item drawn at `(column, row)` in the bar along the top of `area`.
    #[must_use]
    pub fn bar_item_at(&self, area: Rect, column: u16, row: u16) -> Option<usize> {
        if row != area.y || column < area.x || column - area.x >= area.width {
            return None;
        }
        let mut x = usize::from(area.x) + 1;
        let column = usize::from(column);
        for (index, item) in self.state.items.iter().enumerate() {
            let end = x + width(&item.label) + 2;
            if column >= x && column < end {
                return Some(index);
            }
            x = end;
        }
        None
    }

    fn underline(
        &self,
        cx: &mut RenderCx<'_>,
        item: &MenuItem<T>,
        at: (u16, u16),
        shown: u16,
        style: Style,
    ) {
        if !self.mnemonics || item.disabled {
            return;
        }
        let Some(key) = item.mnemonic else {
            return;
        };
        let Some((byte, letter)) = item.label.char_indices().find(|(_, ch)| same_key(*ch, key))
        else {
            return;
        };
        let offset = u16::try_from(width(&item.label[..byte])).unwrap_or(u16::MAX);
        let cells = u16::try_from(lunatui_core::char_width(letter))
            .unwrap_or(1)
            .max(1);
        if offset.saturating_add(cells) > shown {
            return;
        }
        let underlined = style.with_attrs(style.attrs.set_underline(UnderlineStyle::Single));
        cx.set_style(
            Rect::new(at.0.saturating_add(offset), at.1, cells, 1),
            underlined,
        );
    }
}

impl<T> Widget for Menu<'_, T> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let colors = MenuColors::new(cx, self.widget_state);
        self.render_bar(area, cx, colors);
        let Some(&selected_root) = self.state.selected_path.first() else {
            return;
        };
        let Some(root_item) = self.state.items.get(selected_root) else {
            return;
        };
        if root_item.disabled || !root_item.is_group() {
            return;
        }
        let dropdowns = dropdown_count(&self.state.items, &self.state.selected_path);
        self.render_dropdown(
            area,
            cx,
            colors,
            &Dropdown {
                x: area
                    .x
                    .saturating_add(root_item_bar_offset(&self.state.items, selected_root)),
                y: area.y.saturating_add(1),
                items: &root_item.children,
                selected_path: self.state.selected_path.get(1..).unwrap_or_default(),
                remaining: dropdowns,
            },
        );
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            Size::new(0, 0)
        } else {
            avail
        }
    }
}

#[derive(Clone, Copy)]
struct MenuColors {
    bar: Style,
    panel: Style,
    border: Style,
    selected: Style,
    disabled: Style,
}

impl MenuColors {
    fn new(cx: &RenderCx<'_>, state: WidgetState) -> Self {
        let disabled = Style::new()
            .fg(cx.token(Token::TextMuted))
            .bg(cx.token(Token::SurfaceRaised))
            .with_attrs(lunatui_core::Attrs::new().set_dim(true));
        let normal = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(cx.token(Token::SurfaceRaised));
        let panel = if state == WidgetState::Disabled {
            disabled
        } else {
            normal
        };
        let bar = panel;
        let border = panel;
        let selected = match state {
            WidgetState::Focused => Style::new()
                .fg(cx.token(Token::TextMutedOnFocus))
                .bg(cx.token(Token::FocusFill)),
            WidgetState::Disabled => disabled,
            WidgetState::Normal => Style::new()
                .fg(cx.token(Token::Accent))
                .bg(cx.token(Token::SurfaceRaised))
                .with_attrs(lunatui_core::Attrs::new().set_bold(true)),
        };
        Self {
            bar,
            panel,
            border,
            selected,
            disabled,
        }
    }
}

struct Dropdown<'a, T> {
    x: u16,
    y: u16,
    items: &'a [MenuItem<T>],
    selected_path: &'a [usize],
    remaining: u16,
}

impl<T> Menu<'_, T> {
    fn render_bar(&self, area: Rect, cx: &mut RenderCx<'_>, colors: MenuColors) {
        let mut x = area.x;
        put_clipped(cx, area, x, area.y, " ", colors.bar);
        x = x.saturating_add(1);
        for (index, item) in self.state.items.iter().enumerate() {
            if x >= area.x.saturating_add(area.width) {
                break;
            }
            let style = if item.disabled {
                colors.disabled
            } else if self.state.selected_path.first() == Some(&index) {
                colors.selected
            } else {
                colors.bar
            };
            let start = x;
            x = x.saturating_add(put_clipped(cx, area, x, area.y, " ", style));
            let shown = put_clipped(cx, area, x, area.y, &item.label, style);
            self.underline(cx, item, (x, area.y), shown, style);
            x = x.saturating_add(shown);
            x = x.saturating_add(put_clipped(cx, area, x, area.y, " ", style));
            if x == start {
                break;
            }
        }
    }

    fn render_dropdown(
        &self,
        bounds: Rect,
        cx: &mut RenderCx<'_>,
        colors: MenuColors,
        dropdown: &Dropdown<'_, T>,
    ) {
        let Dropdown {
            x,
            y,
            items,
            selected_path,
            remaining,
        } = *dropdown;
        let selected = selected_path.first().copied();
        if items.is_empty() || bounds.width == 0 || bounds.height < 3 {
            return;
        }
        let child_width = items
            .iter()
            .map(|item| width(&item.label))
            .max()
            .unwrap_or(0);
        let preferred_width = u16::try_from(child_width)
            .unwrap_or(u16::MAX)
            .saturating_add(6);
        let panel_width = preferred_width.min(bounds.width);
        let panel_height = u16::try_from(items.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2)
            .min(bounds.height);
        if panel_width < 3 || panel_height < 3 {
            return;
        }
        let width_reserve = self
            .dropdown_width
            .saturating_mul(remaining.max(1))
            .min(bounds.width);
        let max_x = bounds
            .x
            .saturating_add(bounds.width)
            .saturating_sub(width_reserve);
        let rightmost_fit = bounds
            .x
            .saturating_add(bounds.width)
            .saturating_sub(panel_width);
        let max_y = bounds
            .y
            .saturating_add(bounds.height)
            .saturating_sub(panel_height);
        let panel_x = x.max(bounds.x).min(max_x).min(rightmost_fit);
        let panel_y = y.max(bounds.y).min(max_y);
        let panel = Rect::new(panel_x, panel_y, panel_width, panel_height);
        for row in panel.y..panel.y.saturating_add(panel.height) {
            fill_spaces(cx, panel, panel.x, row, panel.width, colors.panel);
        }
        if panel.width > 2 {
            Block::new()
                .borders(Borders::ALL)
                .style(colors.panel)
                .border_style(colors.border)
                .render(
                    Rect::new(
                        panel.x.saturating_add(1),
                        panel.y,
                        panel.width.saturating_sub(2),
                        panel.height,
                    ),
                    cx,
                );
        }
        let max_label = u16::try_from(child_width).unwrap_or(u16::MAX);
        self.render_dropdown_items(dropdown, panel, cx, colors, max_label);

        let Some(index) = selected else {
            return;
        };
        let Some(item) = items.get(index) else {
            return;
        };
        if item.disabled || !item.is_group() || remaining <= 1 {
            return;
        }
        let nested_x = panel.x.saturating_add(2).saturating_add(max_label);
        let nested_y = panel
            .y
            .saturating_add(1)
            .saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
        self.render_dropdown(
            bounds,
            cx,
            colors,
            &Dropdown {
                x: nested_x,
                y: nested_y,
                items: &item.children,
                selected_path: selected_path.get(1..).unwrap_or_default(),
                remaining: remaining.saturating_sub(1),
            },
        );
    }

    fn render_dropdown_items(
        &self,
        dropdown: &Dropdown<'_, T>,
        panel: Rect,
        cx: &mut RenderCx<'_>,
        colors: MenuColors,
        max_label: u16,
    ) {
        let selected = dropdown.selected_path.first().copied();
        for (index, item) in dropdown.items.iter().enumerate() {
            let row_y = panel
                .y
                .saturating_add(1)
                .saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            if row_y >= panel.y.saturating_add(panel.height).saturating_sub(1) {
                break;
            }
            let is_selected = selected == Some(index);
            let style = if item.disabled {
                colors.disabled
            } else if is_selected {
                colors.selected
            } else {
                colors.panel
            };
            let row_x = panel.x.saturating_add(2);
            let mut label_x = row_x;
            label_x = label_x.saturating_add(put_clipped(cx, panel, label_x, row_y, " ", style));
            let shown = put_clipped(cx, panel, label_x, row_y, &item.label, style);
            self.underline(cx, item, (label_x, row_y), shown, style);
            label_x = label_x.saturating_add(shown);
            let label_width = u16::try_from(width(&item.label)).unwrap_or(u16::MAX);
            let padding = max_label.saturating_sub(label_width);
            fill_spaces(cx, panel, label_x, row_y, padding, style);
            label_x = label_x.saturating_add(padding);
            put_clipped(
                cx,
                panel,
                label_x,
                row_y,
                if item.is_group() { ">" } else { " " },
                style,
            );
        }
    }
}

fn put_clipped(
    cx: &mut RenderCx<'_>,
    bounds: Rect,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
) -> u16 {
    if x < bounds.x || y < bounds.y || y >= bounds.y.saturating_add(bounds.height) {
        return 0;
    }
    let right = bounds.x.saturating_add(bounds.width);
    let remaining = right.saturating_sub(x);
    let clipped = clip_display_prefix(text, remaining);
    if clipped.is_empty() {
        return 0;
    }
    cx.put_str(x, y, clipped, style);
    u16::try_from(width(clipped)).unwrap_or(remaining)
}

fn fill_spaces(
    cx: &mut RenderCx<'_>,
    bounds: Rect,
    mut x: u16,
    y: u16,
    mut count: u16,
    style: Style,
) {
    const SPACES: &str = "                                                                ";
    while count > 0 {
        let chunk = count.min(64);
        let painted = put_clipped(cx, bounds, x, y, &SPACES[..usize::from(chunk)], style);
        if painted == 0 {
            return;
        }
        x = x.saturating_add(painted);
        count -= painted;
    }
}

fn root_item_bar_offset<T>(items: &[MenuItem<T>], selected: usize) -> u16 {
    let width = items[..selected]
        .iter()
        .map(|item| width(&item.label).saturating_add(2))
        .sum::<usize>();
    u16::try_from(width).unwrap_or(u16::MAX)
}

fn dropdown_count<T>(items: &[MenuItem<T>], selected_path: &[usize]) -> u16 {
    let mut count = 0_u16;
    let mut level = items;
    for (depth, &index) in selected_path.iter().enumerate() {
        let Some(item) = level.get(index) else {
            break;
        };
        if item.is_group() || level.iter().any(MenuItem::is_group) {
            count = count.saturating_add(1);
        }
        if depth + 1 == selected_path.len() {
            break;
        }
        level = &item.children;
    }
    count
}

fn same_key(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

fn first_enabled<T>(items: &[MenuItem<T>]) -> Option<usize> {
    items.iter().position(|item| !item.disabled)
}

fn item_at_path<'a, T>(items: &'a [MenuItem<T>], path: &[usize]) -> Option<&'a MenuItem<T>> {
    let (&index, rest) = path.split_first()?;
    let item = items.get(index)?;
    if rest.is_empty() {
        Some(item)
    } else {
        item_at_path(&item.children, rest)
    }
}

fn items_at_path<'a, T>(items: &'a [MenuItem<T>], path: &[usize]) -> Option<&'a [MenuItem<T>]> {
    let mut level = items;
    for &index in path {
        level = &level.get(index)?.children;
    }
    Some(level)
}

#[cfg(test)]
mod tests {
    use super::{MenuEvent, MenuItem, MenuState};

    fn fixture() -> MenuState<&'static str> {
        MenuState::new(vec![
            MenuItem::group(
                "File",
                vec![
                    MenuItem::item("New", "new"),
                    MenuItem::group("Recent", vec![MenuItem::item("notes.txt", "notes")]),
                ],
            ),
            MenuItem::group("Edit", vec![MenuItem::item("Copy", "copy")]),
        ])
    }

    #[test]
    fn enter_navigate_and_emit_leaf_action() {
        let mut state = fixture();
        assert!(state.activate());
        assert!(state.down());
        assert_eq!(state.selected_path(), [0, 0]);
        assert!(state.down());
        assert_eq!(state.selected_path(), [0, 1]);
        state.select();
        assert_eq!(state.selected_path(), [0, 1, 0]);
        state.select();
        assert_eq!(
            state.drain_events().collect::<Vec<_>>(),
            [MenuEvent::Selected("notes")]
        );
    }

    #[test]
    fn left_and_right_follow_the_menu_levels() {
        let mut state = fixture();
        state.activate();
        state.down();
        assert_eq!(state.selected_path(), [0, 0]);
        assert!(state.left());
        assert_eq!(state.selected_path(), [0]);
        assert!(state.right());
        assert_eq!(state.selected_path(), [1]);
    }

    #[test]
    fn disabled_items_are_skipped_and_cannot_emit_actions() {
        let mut state = MenuState::new(vec![
            MenuItem::item("Disabled", "bad").disabled(true),
            MenuItem::item("Enabled", "good"),
        ]);
        assert!(state.activate());
        assert_eq!(state.selected_path(), [1]);
        state.select();
        assert_eq!(
            state.drain_events().collect::<Vec<_>>(),
            [MenuEvent::Selected("good")]
        );
    }
}
