// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_ui_tokens::{Role, surface, text};
use lunatui_widgets::{RadioFlow, RadioGroup, RenderCx, Widget};

pub use lunatui_widgets::{ButtonState, WidgetState};

const BOX_WIDTH: u16 = 3;
const TRACK_WIDTH: u16 = 3;

/// Whether a [`Checkbox`] is checked.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckState {
    /// `[ ]`
    #[default]
    Unchecked,
    /// `[x]`
    Checked,
    /// `[-]`, for a parent whose children are only partly checked.
    Mixed,
}

impl CheckState {
    /// The state a click leads to: a mixed or unchecked box becomes checked.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Checked => Self::Unchecked,
            Self::Unchecked | Self::Mixed => Self::Checked,
        }
    }

    const fn glyph(self) -> &'static str {
        match self {
            Self::Unchecked => "[ ]",
            Self::Checked => "[x]",
            Self::Mixed => "[-]",
        }
    }
}

fn label_width(label: &str) -> u16 {
    u16::try_from(width(label)).unwrap_or(u16::MAX)
}

fn styles(cx: &RenderCx<'_>, state: WidgetState, active: bool, on: Role) -> (Style, Style) {
    let page = surface(cx, on);
    let mut mark = page.fg(if active {
        text(cx, Role::Primary).fg
    } else {
        text(cx, Role::Border).fg
    });
    let mut label = page;
    match state {
        WidgetState::Disabled => {
            mark = mark.with_attrs(Attrs::new().set_dim(true));
            label = page
                .fg(text(cx, Role::Muted).fg)
                .with_attrs(Attrs::new().set_dim(true));
        }
        WidgetState::Focused => {
            let bold = Attrs::new().set_bold(true);
            mark = page.fg(text(cx, Role::Ring).fg).with_attrs(bold);
            label = label.with_attrs(bold);
        }
        _ => {
            if active {
                mark = mark.with_attrs(Attrs::new().set_bold(true));
            }
        }
    }
    (mark, label)
}

fn paint(
    area: Rect,
    glyph: &str,
    glyph_width: u16,
    label: &str,
    styles: (Style, Style),
    cx: &mut RenderCx<'_>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    cx.fill(area, styles.1, ' ');
    cx.put_str(
        area.x,
        area.y,
        clip_display_prefix(glyph, area.width),
        styles.0,
    );
    let room = area.width.saturating_sub(glyph_width + 1);
    if room > 0 {
        cx.put_str(
            area.x.saturating_add(glyph_width).saturating_add(1),
            area.y,
            clip_display_prefix(label, room),
            styles.1,
        );
    }
}

/// A `[x]` box followed by a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkbox<'a> {
    label: &'a str,
    checked: CheckState,
    state: WidgetState,
    on: Role,
}

impl<'a> Checkbox<'a> {
    /// An unchecked box with this label.
    #[must_use]
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            checked: CheckState::Unchecked,
            state: WidgetState::Normal,
            on: Role::Background,
        }
    }

    /// Sets whether the box is checked, unchecked or mixed.
    #[must_use]
    pub const fn checked(mut self, checked: CheckState) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the interaction state.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Names the surface the box sits on, [`Role::Background`] by default.
    #[must_use]
    pub const fn on(mut self, role: Role) -> Self {
        self.on = role;
        self
    }

    /// The cells a click should reach in `area`: all of it, or none when disabled.
    #[must_use]
    pub const fn hit_area(&self, area: Rect) -> Rect {
        if matches!(self.state, WidgetState::Disabled) {
            return Rect::new(area.x, area.y, 0, 0);
        }
        area
    }
}

impl Widget for Checkbox<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let active = !matches!(self.checked, CheckState::Unchecked);
        let style = styles(cx, self.state, active, self.on);
        paint(area, self.checked.glyph(), BOX_WIDTH, self.label, style, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(
            BOX_WIDTH
                .saturating_add(1)
                .saturating_add(label_width(self.label)),
            1,
        )
        .clamp(avail)
    }
}

/// A three-cell track with a knob at the on or off end, followed by a label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Switch<'a> {
    label: &'a str,
    on: bool,
    state: WidgetState,
    surface: Role,
}

impl<'a> Switch<'a> {
    /// A switch showing `label`, on or off.
    #[must_use]
    pub const fn new(label: &'a str, on: bool) -> Self {
        Self {
            label,
            on,
            state: WidgetState::Normal,
            surface: Role::Background,
        }
    }

    /// Sets the interaction state.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Names the surface the switch sits on, [`Role::Background`] by default.
    #[must_use]
    pub const fn on_surface(mut self, role: Role) -> Self {
        self.surface = role;
        self
    }

    /// Whether the switch is on.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.on
    }

    /// The cells a click should reach in `area`: all of it, or none when disabled.
    #[must_use]
    pub const fn hit_area(&self, area: Rect) -> Rect {
        if matches!(self.state, WidgetState::Disabled) {
            return Rect::new(area.x, area.y, 0, 0);
        }
        area
    }
}

impl Widget for Switch<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let glyph = if self.on { "━━●" } else { "●━━" };
        let style = styles(cx, self.state, self.on, self.surface);
        paint(area, glyph, TRACK_WIDTH, self.label, style, cx);
    }

    fn measure(&self, avail: Size) -> Size {
        Size::new(
            TRACK_WIDTH
                .saturating_add(1)
                .saturating_add(label_width(self.label)),
            1,
        )
        .clamp(avail)
    }
}

/// A group of options of which one is selected: `◉ Selected`, `○ Other`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Radio {
    options: Vec<String>,
    selected: Option<usize>,
    focused: Option<usize>,
    disabled: Vec<usize>,
    flow: RadioFlow,
    gap: u16,
    state: WidgetState,
    on: Role,
}

impl Radio {
    /// A group with these option labels and nothing selected.
    #[must_use]
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            focused: None,
            disabled: Vec::new(),
            flow: RadioFlow::Vertical,
            gap: 0,
            state: WidgetState::Normal,
            on: Role::Background,
        }
    }

    /// The selected option, if any.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// The option with the focus ring, if any.
    #[must_use]
    pub fn focused(mut self, index: Option<usize>) -> Self {
        self.focused = index;
        self
    }

    /// Options that dim and cannot be stepped onto.
    #[must_use]
    pub fn disabled_options(mut self, indexes: impl IntoIterator<Item = usize>) -> Self {
        self.disabled = indexes.into_iter().collect();
        self
    }

    /// Lays the options out in a column or a row.
    #[must_use]
    pub const fn flow(mut self, flow: RadioFlow) -> Self {
        self.flow = flow;
        self
    }

    /// Blank rows or cells between options.
    #[must_use]
    pub const fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }

    /// Sets the state of the whole group; a disabled group dims every option.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Names the surface the group sits on, [`Role::Background`] by default.
    #[must_use]
    pub const fn on(mut self, role: Role) -> Self {
        self.on = role;
        self
    }

    fn group(&self) -> RadioGroup {
        RadioGroup::new(self.options.iter().cloned())
            .flow(self.flow)
            .gap(self.gap)
            .disabled_options(self.disabled.iter().copied())
    }

    /// The next enabled option after `from`, wrapping around.
    #[must_use]
    pub fn step(&self, from: Option<usize>, forward: bool) -> Option<usize> {
        self.group().step(from, forward)
    }

    /// The cells each option covers in `area`, for hit testing.
    #[must_use]
    pub fn option_areas(&self, area: Rect) -> Vec<(usize, Rect)> {
        self.group().option_areas(area)
    }

    fn option_state(&self, index: usize) -> WidgetState {
        if matches!(self.state, WidgetState::Disabled) || self.disabled.contains(&index) {
            WidgetState::Disabled
        } else if self.focused == Some(index) {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        }
    }
}

impl Widget for Radio {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        cx.fill(area, surface(cx, self.on), ' ');
        for (index, rect) in self.option_areas(area) {
            let selected = self.selected == Some(index);
            let style = styles(cx, self.option_state(index), selected, self.on);
            let glyph = if selected { "◉" } else { "○" };
            cx.put_str(rect.x, rect.y, glyph, style.0);
            let room = rect.width.saturating_sub(2);
            if room > 0 {
                let label = clip_display_prefix(&self.options[index], room);
                cx.put_str(rect.x + 2, rect.y, label, style.1);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        self.group().measure(avail)
    }
}
