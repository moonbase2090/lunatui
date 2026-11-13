// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix};
use lunatui_runtime::{HitId, Layer};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const CLOSED: &str = " ▾ ";
const OPEN: &str = " ▴ ";
const CHECK: &str = "✓";
const INDICATOR_WIDTH: u16 = 3;
const MIN_WITH_INDICATOR: u16 = 8;
const DEFAULT_ROWS: u16 = 6;

/// A one-row field showing the chosen option, with a drop-down list of every option.
///
/// The app owns which option is chosen, whether the list is [`open`](Select::open), and which
/// row is [`active`](Select::active) while it is open. The field is not edited like a text box:
/// [`step`](Select::step) moves the active row for the arrow keys and
/// [`type_ahead`](Select::type_ahead) jumps to the next option starting with what was typed.
/// The chosen option carries a check in the list. The list shows at most
/// [`max_rows`](Select::max_rows) rows, scrolled to keep the active one in view, and says
/// "No options" when there are none. [`field_area`](Select::field_area),
/// [`list_area`](Select::list_area), and [`option_at`](Select::option_at) are for hit testing.
/// A choice or active row past the last option is ignored. Drawing allocates nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Select<'a, S = &'a str> {
    options: &'a [S],
    selected: Option<usize>,
    placeholder: Cow<'a, str>,
    open: bool,
    active: Option<usize>,
    max_rows: u16,
    state: WidgetState,
}

impl<'a, S: AsRef<str>> Select<'a, S> {
    /// A field offering `options` with `selected` chosen (an index into `options`), or the
    /// placeholder when it is `None`.
    #[must_use]
    pub fn new(options: &'a [S], selected: Option<usize>) -> Self {
        Self {
            options,
            selected,
            placeholder: Cow::Borrowed(""),
            open: false,
            active: None,
            max_rows: DEFAULT_ROWS,
            state: WidgetState::Normal,
        }
    }

    /// Dim text shown while nothing is chosen.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Whether the list of options is drawn below the field.
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The highlighted row of the open list, as an index into the options.
    #[must_use]
    pub const fn active(mut self, active: Option<usize>) -> Self {
        self.active = active;
        self
    }

    /// The most rows the list may take (at least one). Six by default.
    #[must_use]
    pub fn max_rows(mut self, rows: u16) -> Self {
        self.max_rows = rows.max(1);
        self
    }

    /// Interaction state: focused draws the arrow bold in the accent color and the active row
    /// on it, disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// The option after (or before) `current` out of `total`, wrapping around. From `None`,
    /// forward gives the first option and backward the last; `None` when there are no options.
    #[must_use]
    pub fn step(current: Option<usize>, total: usize, forward: bool) -> Option<usize> {
        if total == 0 {
            return None;
        }
        Some(match (current.filter(|index| *index < total), forward) {
            (Some(index), true) => (index + 1) % total,
            (Some(index), false) => (index + total - 1) % total,
            (None, true) => 0,
            (None, false) => total - 1,
        })
    }

    /// The next option after `current`, wrapping around, whose text starts with `prefix`
    /// (ignoring case), for type-ahead. `None` for an empty prefix or when nothing starts
    /// with it.
    #[must_use]
    pub fn type_ahead(options: &[S], current: Option<usize>, prefix: &str) -> Option<usize> {
        if prefix.is_empty() || options.is_empty() {
            return None;
        }
        let start = current
            .filter(|index| *index < options.len())
            .map_or(0, |index| index + 1);
        (0..options.len())
            .map(|offset| (start + offset) % options.len())
            .find(|index| starts_with_ignoring_case(options[*index].as_ref(), prefix))
    }

    fn chosen(&self) -> Option<usize> {
        self.selected.filter(|index| *index < self.options.len())
    }

    fn rows_for(&self, height: u16) -> u16 {
        if !self.open {
            return 0;
        }
        self.open_rows(height)
    }

    fn open_rows(&self, height: u16) -> u16 {
        let wanted = u16::try_from(self.options.len().max(1)).unwrap_or(u16::MAX);
        wanted.min(self.max_rows).min(height.saturating_sub(1))
    }

    /// The row of the field in `area`.
    #[must_use]
    pub fn field_area(&self, area: Rect) -> Rect {
        Rect::new(area.x, area.y, area.width, area.height.min(1))
    }

    /// The drop-down's layer for the app's [`LayerStack`](lunatui_runtime::LayerStack): a
    /// non-modal layer over the list the select shows in `area` once open, anchored on the field
    /// so a press there reaches the select instead of closing the list. Esc or a press outside
    /// closes it, and focus goes back to `opener`.
    #[must_use]
    pub fn layer(&self, area: Rect, opener: Option<HitId>) -> Layer {
        let list = Rect::new(area.x, area.y + 1, area.width, self.open_rows(area.height));
        let layer = Layer::popover(list).anchor(self.field_area(area));
        match opener {
            Some(opener) => layer.return_focus(opener),
            None => layer,
        }
    }

    /// The cells of the drop-down list in `area` (empty while it is closed).
    #[must_use]
    pub fn list_area(&self, area: Rect) -> Rect {
        Rect::new(area.x, area.y + 1, area.width, self.rows_for(area.height))
    }

    /// The index of the option on the list row at (`x`, `y`), if any.
    #[must_use]
    pub fn option_at(&self, area: Rect, x: u16, y: u16) -> Option<usize> {
        let list = self.list_area(area);
        let inside =
            x >= list.x && x - list.x < list.width && y >= list.y && y - list.y < list.height;
        let index = self.first_row(list.height) + usize::from(y.saturating_sub(list.y));
        (inside && index < self.options.len()).then_some(index)
    }

    fn active_index(&self) -> Option<usize> {
        self.active.filter(|index| *index < self.options.len())
    }

    fn first_row(&self, rows: u16) -> usize {
        let active = self.active_index().or_else(|| self.chosen()).unwrap_or(0);
        (active + 1).saturating_sub(usize::from(rows))
    }

    fn text_area(area: Rect) -> Rect {
        let right = if area.width >= MIN_WITH_INDICATOR {
            INDICATOR_WIDTH
        } else {
            1
        };
        Rect::new(
            area.x + 1.min(area.width),
            area.y,
            area.width.saturating_sub(1 + right),
            area.height.min(1),
        )
    }

    fn render_field(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let surface = cx.token(Token::Surface1);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        cx.fill(Rect::new(area.x, area.y, area.width, 1), base, ' ');
        let field = Self::text_area(area);
        if let Some(index) = self.chosen() {
            let shown = clip_display_prefix(self.options[index].as_ref(), field.width);
            cx.put_str(field.x, field.y, shown, base);
        } else {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            let shown = clip_display_prefix(&self.placeholder, field.width);
            cx.put_str(field.x, field.y, shown, dim);
        }
        if area.width >= MIN_WITH_INDICATOR {
            let chevron = Style::new()
                .fg(cx.token(Token::Accent))
                .bg(surface)
                .with_attrs(Attrs::new().set_dim(disabled || !focused).set_bold(focused));
            let at = area.x + area.width - INDICATOR_WIDTH;
            cx.put_str(at, area.y, if self.open { OPEN } else { CLOSED }, chevron);
        }
    }

    fn render_list(&self, area: Rect, rows: u16, cx: &mut RenderCx<'_>) {
        let list = Rect::new(area.x, area.y + 1, area.width, rows);
        let disabled = matches!(self.state, WidgetState::Disabled);
        let surface = cx.token(Token::Surface0);
        let base = Style::new()
            .fg(cx.token(Token::TextPrimary))
            .bg(surface)
            .with_attrs(Attrs::new().set_dim(disabled));
        if self.options.is_empty() {
            let dim = base.with_attrs(Attrs::new().set_dim(true));
            cx.fill(Rect::new(list.x, list.y, list.width, 1), base, ' ');
            let message = clip_display_prefix("No options", list.width.saturating_sub(2));
            cx.put_str(list.x + 1, list.y, message, dim);
            return;
        }
        let first = self.first_row(list.height);
        let accent = cx.token(Token::Accent);
        for row in 0..list.height {
            let position = first + usize::from(row);
            let y = list.y + row;
            let Some(option) = self.options.get(position) else {
                cx.fill(Rect::new(list.x, y, list.width, 1), base, ' ');
                continue;
            };
            let row_base = if self.active_index() == Some(position) {
                Style::new()
                    .fg(surface)
                    .bg(accent)
                    .with_attrs(Attrs::new().set_dim(disabled))
            } else {
                base
            };
            cx.fill(Rect::new(list.x, y, list.width, 1), row_base, ' ');
            if self.chosen() == Some(position) && list.width >= 2 {
                cx.put_str(list.x + 1, y, CHECK, row_base);
            }
            let shown = clip_display_prefix(option.as_ref(), list.width.saturating_sub(3));
            cx.put_str(list.x + 3.min(list.width), y, shown, row_base);
        }
    }
}

fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    let mut chars = text.chars();
    prefix.chars().all(|wanted| {
        chars
            .next()
            .is_some_and(|got| got.to_lowercase().eq(wanted.to_lowercase()))
    })
}

impl<S: AsRef<str>> Widget for Select<'_, S> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.render_field(area, cx);
        let rows = self.rows_for(area.height);
        if rows > 0 {
            self.render_list(area, rows, cx);
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        Size::new(avail.width, 1 + self.rows_for(avail.height))
    }
}
