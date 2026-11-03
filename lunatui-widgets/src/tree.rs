// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget};

const OPEN: &str = "▾";
const CLOSED: &str = "▸";
const GUIDE: &str = "│";
const DEFAULT_INDENT: u16 = 2;

/// One node of a [`Tree`]: a label, an optional badge, and its children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeItem {
    label: String,
    badge: Option<String>,
    children: Vec<Self>,
    disabled: bool,
}

impl TreeItem {
    /// A node with no children.
    #[must_use]
    pub fn leaf(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            badge: None,
            children: Vec::new(),
            disabled: false,
        }
    }

    /// A node holding `children`, shown while the node is open.
    #[must_use]
    pub fn node(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            label: label.into(),
            badge: None,
            children,
            disabled: false,
        }
    }

    /// Dim text on the right of the row, such as a count.
    #[must_use]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Dims the node. A disabled node cannot be opened, closed, or selected.
    #[must_use]
    pub const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// The node's text.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The node's children, in display order.
    #[must_use]
    pub fn children(&self) -> &[Self] {
        &self.children
    }

    /// Whether the node has children to open.
    #[must_use]
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Whether the node is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// One visible row of a flattened tree, as [`TreeState::visible`] reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow<'a> {
    /// Index of the node at each nesting level, from the root down.
    pub path: Vec<usize>,
    /// Nesting level of the row; roots are depth zero.
    pub depth: usize,
    /// The node's text.
    pub label: &'a str,
    /// Whether the node has children.
    pub has_children: bool,
    /// Whether the node's children are shown below it.
    pub open: bool,
    /// Whether the node is disabled.
    pub disabled: bool,
    last_ancestors: Vec<bool>,
}

/// Application-owned state for [`Tree`]: the nodes, which of them are open, and the selection.
///
/// Store this across frames, call its navigation methods from the app's keymap, and render it
/// through [`Tree::new`]. Nodes are addressed by path: the index of the node at each nesting
/// level, so `[0, 1]` is the second child of the first root. A node's children are visible only
/// while every ancestor on its path is open. Disabled nodes stay visible but cannot be opened,
/// closed, or selected, and navigation skips them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeState {
    items: Vec<TreeItem>,
    open: BTreeSet<Vec<usize>>,
    selected: Option<Vec<usize>>,
    offset: usize,
}

impl TreeState {
    /// A tree of these root nodes, all closed and nothing selected.
    #[must_use]
    pub fn new(items: Vec<TreeItem>) -> Self {
        Self {
            items,
            open: BTreeSet::new(),
            selected: None,
            offset: 0,
        }
    }

    /// The root nodes, in display order.
    #[must_use]
    pub fn items(&self) -> &[TreeItem] {
        &self.items
    }

    /// The node at `path`, whether or not it is currently visible.
    #[must_use]
    pub fn item_at(&self, path: &[usize]) -> Option<&TreeItem> {
        item_at_path(&self.items, path)
    }

    /// Whether the node at `path` is open.
    #[must_use]
    pub fn is_open(&self, path: &[usize]) -> bool {
        self.open.contains(path)
    }

    /// Opens the node at `path` so its children show. Returns whether it changed: the node must
    /// exist, be enabled, and have children.
    pub fn open(&mut self, path: &[usize]) -> bool {
        if !self.can_toggle(path) || self.open.contains(path) {
            return false;
        }
        self.open.insert(path.to_vec());
        true
    }

    /// Closes the node at `path`, hiding its subtree. A selection inside the subtree moves to
    /// this node. Returns whether it changed.
    pub fn close(&mut self, path: &[usize]) -> bool {
        if !self.can_toggle(path) || !self.open.remove(path) {
            return false;
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| selected.len() > path.len() && selected.starts_with(path))
        {
            self.selected = Some(path.to_vec());
        }
        true
    }

    /// Opens the node at `path` if it is closed, or closes it if it is open.
    pub fn toggle(&mut self, path: &[usize]) -> bool {
        if self.open.contains(path) {
            self.close(path)
        } else {
            self.open(path)
        }
    }

    /// Opens every enabled node that has children.
    pub fn expand_all(&mut self) {
        fn walk(items: &[TreeItem], path: &mut Vec<usize>, open: &mut BTreeSet<Vec<usize>>) {
            for (index, item) in items.iter().enumerate() {
                path.push(index);
                if item.has_children() && !item.disabled {
                    open.insert(path.clone());
                    walk(&item.children, path, open);
                }
                path.pop();
            }
        }
        walk(&self.items, &mut Vec::new(), &mut self.open);
    }

    /// Closes every node. A selection below the roots moves to its root, or clears when that
    /// root is disabled.
    pub fn collapse_all(&mut self) {
        self.open.clear();
        if let Some(selected) = &self.selected
            && selected.len() > 1
        {
            let root = vec![selected[0]];
            self.selected = self
                .item_at(&root)
                .is_some_and(|item| !item.disabled)
                .then_some(root);
        }
    }

    /// The selected node's path.
    #[must_use]
    pub fn selected_path(&self) -> Option<&[usize]> {
        self.selected.as_deref()
    }

    /// The selected node.
    #[must_use]
    pub fn selected(&self) -> Option<&TreeItem> {
        self.selected.as_deref().and_then(|path| self.item_at(path))
    }

    /// Selects the node at `path`. Returns whether it changed: the node must be visible and
    /// enabled. Use [`reveal`](Self::reveal) to select a node inside closed ancestors.
    pub fn select(&mut self, path: &[usize]) -> bool {
        if self.selected.as_deref() == Some(path) || !self.is_selectable(path) {
            return false;
        }
        self.selected = Some(path.to_vec());
        true
    }

    /// Opens every ancestor of the node at `path` and selects it. Returns whether the selection
    /// changed: the node and its ancestors must exist and be enabled.
    pub fn reveal(&mut self, path: &[usize]) -> bool {
        if path.is_empty() || self.selected.as_deref() == Some(path) {
            return false;
        }
        for end in 1..path.len() {
            let ancestor = &path[..end];
            match self.item_at(ancestor) {
                Some(item) if item.has_children() && !item.disabled => {}
                _ => return false,
            }
        }
        if self.item_at(path).is_none_or(|item| item.disabled) {
            return false;
        }
        for end in 1..path.len() {
            self.open.insert(path[..end].to_vec());
        }
        self.selected = Some(path.to_vec());
        true
    }

    /// Selects the first visible enabled node.
    pub fn select_first(&mut self) -> bool {
        self.visible()
            .iter()
            .find(|row| !row.disabled)
            .map(|row| row.path.clone())
            .is_some_and(|path| self.select(&path))
    }

    /// Selects the last visible enabled node.
    pub fn select_last(&mut self) -> bool {
        self.visible()
            .iter()
            .rev()
            .find(|row| !row.disabled)
            .map(|row| row.path.clone())
            .is_some_and(|path| self.select(&path))
    }

    /// Moves the selection to the next visible enabled node. With no selection, selects the
    /// first one.
    pub fn move_down(&mut self) -> bool {
        self.step(true)
    }

    /// Moves the selection to the previous visible enabled node. With no selection, selects the
    /// last one.
    pub fn move_up(&mut self) -> bool {
        self.step(false)
    }

    /// Opens the selected node if it is a closed parent, or moves to its first enabled child if
    /// it is already open.
    pub fn move_right(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        let Some(item) = self.item_at(&path) else {
            return false;
        };
        if !item.has_children() {
            return false;
        }
        if !self.open.contains(&path) {
            return self.open(&path);
        }
        item.children
            .iter()
            .position(|child| !child.disabled)
            .is_some_and(|index| {
                let mut child = path.clone();
                child.push(index);
                self.select(&child)
            })
    }

    /// Closes the selected node if it is an open parent, or moves to its parent otherwise.
    pub fn move_left(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        if self.open.contains(&path) {
            return self.close(&path);
        }
        if path.len() <= 1 {
            return false;
        }
        self.select(&path[..path.len() - 1])
    }

    /// Index of the first visible row the view asks to start from. Rendering still pulls it
    /// back far enough to keep the selection on screen.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Asks the view to start from visible row `offset`.
    pub const fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Every row currently shown, in display order: each node followed by its children while it
    /// is open.
    #[must_use]
    pub fn visible(&self) -> Vec<TreeRow<'_>> {
        fn walk<'a>(
            items: &'a [TreeItem],
            path: &mut Vec<usize>,
            last_ancestors: &mut Vec<bool>,
            open: &BTreeSet<Vec<usize>>,
            rows: &mut Vec<TreeRow<'a>>,
        ) {
            let last = items.len().saturating_sub(1);
            for (index, item) in items.iter().enumerate() {
                path.push(index);
                let is_open = item.has_children() && open.contains(path.as_slice());
                rows.push(TreeRow {
                    path: path.clone(),
                    depth: path.len() - 1,
                    label: &item.label,
                    has_children: item.has_children(),
                    open: is_open,
                    disabled: item.disabled,
                    last_ancestors: last_ancestors.clone(),
                });
                if is_open {
                    last_ancestors.push(index == last);
                    walk(&item.children, path, last_ancestors, open, rows);
                    last_ancestors.pop();
                }
                path.pop();
            }
        }
        let mut rows = Vec::new();
        walk(
            &self.items,
            &mut Vec::new(),
            &mut Vec::new(),
            &self.open,
            &mut rows,
        );
        rows
    }

    fn can_toggle(&self, path: &[usize]) -> bool {
        self.item_at(path)
            .is_some_and(|item| item.has_children() && !item.disabled)
    }

    fn is_selectable(&self, path: &[usize]) -> bool {
        !path.is_empty()
            && self.item_at(path).is_some_and(|item| !item.disabled)
            && (1..path.len()).all(|end| self.open.contains(&path[..end]))
    }

    fn step(&mut self, forward: bool) -> bool {
        let visible = self.visible();
        let enabled: Vec<&TreeRow<'_>> = visible.iter().filter(|row| !row.disabled).collect();
        let Some(current) = self
            .selected
            .as_ref()
            .and_then(|selected| enabled.iter().position(|row| &row.path == selected))
        else {
            return if forward {
                self.select_first()
            } else {
                self.select_last()
            };
        };
        let next = if forward {
            enabled.get(current + 1)
        } else {
            current.checked_sub(1).and_then(|index| enabled.get(index))
        };
        next.map(|row| row.path.clone())
            .is_some_and(|path| self.select(&path))
    }
}

fn item_at_path<'a>(items: &'a [TreeItem], path: &[usize]) -> Option<&'a TreeItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get(*first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        item_at_path(&item.children, rest)
    }
}

/// A tree of expandable nodes with guide lines, a caret per parent, and a selected row.
///
/// Render it from a [`TreeState`] the app keeps across frames. Every row of the area is cleared
/// across its full width before it is drawn, and rows past the last visible node are cleared
/// too, so a shrinking tree never leaves stale glyphs behind. When the visible rows do not fit,
/// the view starts at the state's offset, pulled back far enough to keep the selection on
/// screen. [`row_areas`](Tree::row_areas) gives the rectangle of each drawn row for hit
/// testing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tree<'a> {
    state: &'a TreeState,
    style: Option<Style>,
    highlight_style: Option<Style>,
    indent: u16,
}

impl<'a> Tree<'a> {
    /// A tree drawing `state`.
    #[must_use]
    pub const fn new(state: &'a TreeState) -> Self {
        Self {
            state,
            style: None,
            highlight_style: None,
            indent: DEFAULT_INDENT,
        }
    }

    /// Draws unselected rows in `style` instead of theme tokens.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Draws the selected row in `style` instead of the accented default.
    #[must_use]
    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = Some(style);
        self
    }

    /// Cells of indentation per nesting level, at least one.
    #[must_use]
    pub const fn indent(mut self, cells: u16) -> Self {
        self.indent = if cells == 0 { 1 } else { cells };
        self
    }

    /// Where each drawn row sits when the tree is rendered into `area`, as `(path, rect)` pairs
    /// from the first drawn row down, for hit testing.
    #[must_use]
    pub fn row_areas(&self, area: Rect) -> Vec<(Vec<usize>, Rect)> {
        if area.width == 0 || area.height == 0 {
            return Vec::new();
        }
        let visible = self.state.visible();
        let offset = effective_offset(self.state, &visible, area.height);
        visible
            .into_iter()
            .skip(offset)
            .take(usize::from(area.height))
            .enumerate()
            .map(|(row, visible_row)| {
                (
                    visible_row.path,
                    Rect::new(
                        area.x,
                        area.y + u16::try_from(row).unwrap_or(u16::MAX),
                        area.width,
                        1,
                    ),
                )
            })
            .collect()
    }

    fn row_styles(&self, cx: &RenderCx<'_>, row: &TreeRow<'_>, selected: bool) -> (Style, Style) {
        let base = self
            .style
            .unwrap_or_else(|| Style::new().fg(cx.token(Token::TextPrimary)));
        let text = if selected {
            self.highlight_style.unwrap_or_else(|| {
                Style::new()
                    .fg(cx.token(Token::Accent))
                    .with_attrs(Attrs::new().set_bold(true))
            })
        } else if row.disabled {
            base.with_attrs(Attrs::new().set_dim(true))
        } else {
            base
        };
        let guide = if selected {
            text
        } else {
            base.with_attrs(Attrs::new().set_dim(true))
        };
        (text, guide)
    }
}

impl Widget for Tree<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let visible = self.state.visible();
        let offset = effective_offset(self.state, &visible, area.height);
        let blank = self
            .style
            .unwrap_or_else(|| Style::new().fg(cx.token(Token::TextPrimary)));
        for slot in 0..area.height {
            let y = area.y + slot;
            let Some(row) = visible.get(offset + usize::from(slot)) else {
                cx.fill(Rect::new(area.x, y, area.width, 1), blank, ' ');
                continue;
            };
            let selected = self.state.selected.as_ref() == Some(&row.path);
            let (text, guide) = self.row_styles(cx, row, selected);
            cx.fill(Rect::new(area.x, y, area.width, 1), text, ' ');
            draw_row(self, row, area, y, text, guide, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let rows = self.state.visible().len();
        if rows == 0 {
            return Size::new(0, 0);
        }
        let height = u16::try_from(rows).unwrap_or(u16::MAX).min(avail.height);
        Size::new(avail.width, height)
    }
}

fn effective_offset(state: &TreeState, visible: &[TreeRow<'_>], height: u16) -> usize {
    let height = usize::from(height);
    if height == 0 || visible.len() <= height {
        return 0;
    }
    let mut offset = state.offset.min(visible.len() - height);
    if let Some(selected) = state
        .selected
        .as_ref()
        .and_then(|path| visible.iter().position(|row| &row.path == path))
    {
        if selected < offset {
            offset = selected;
        } else if selected >= offset + height {
            offset = selected + 1 - height;
        }
    }
    offset
}

fn draw_row(
    tree: &Tree<'_>,
    row: &TreeRow<'_>,
    area: Rect,
    y: u16,
    text: Style,
    guide: Style,
    cx: &mut RenderCx<'_>,
) {
    let end = area.x.saturating_add(area.width);
    let mut x = area.x;
    for last in &row.last_ancestors {
        if x >= end {
            return;
        }
        if !last {
            cx.put_str(x, y, GUIDE, guide);
        }
        x = x.saturating_add(tree.indent).min(end);
    }
    if x >= end {
        return;
    }
    if row.has_children {
        cx.put_str(x, y, if row.open { OPEN } else { CLOSED }, text);
    }
    x = x.saturating_add(2).min(end);
    if x >= end {
        return;
    }
    let item = tree.state.item_at(&row.path);
    let badge = item.and_then(|item| item.badge.as_deref());
    let badge_width = badge.map_or(0, |label| {
        u16::try_from(width(label) + 1).unwrap_or(u16::MAX)
    });
    let room = end - x;
    let (label_room, badge_width) = if room > badge_width {
        (room - badge_width, badge_width)
    } else {
        (room, 0)
    };
    cx.put_str(x, y, clip_display_prefix(row.label, label_room), text);
    if let (Some(label), true) = (badge, badge_width > 0) {
        let dim = guide.with_attrs(Attrs::new().set_dim(true));
        cx.put_str(end - (badge_width - 1), y, label, dim);
    }
}
