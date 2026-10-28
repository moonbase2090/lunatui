// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_theme::Token;

use crate::{RenderCx, Widget, WidgetState};

const BUTTON: u16 = 3;
const MIN_WITH_BUTTONS: u16 = 9;
const MAX_TEXT: usize = 20;

/// A number field with minus and plus buttons: a spin box.
///
/// The widget shows a value with an optional prefix and suffix, right-aligned between a minus and a
/// plus button. While the user types, [`NumberInput::edit`] shows their text and a caret instead of
/// the formatted value, in the fail color when the text cannot be committed. The app owns the
/// value and the text; [`nudge`](NumberInput::nudge), [`parse`](NumberInput::parse),
/// [`accepts`](NumberInput::accepts), and [`format`](NumberInput::format) are the rules to apply in
/// its key handling, and [`minus_area`](NumberInput::minus_area),
/// [`plus_area`](NumberInput::plus_area), and [`field_area`](NumberInput::field_area) are the cells
/// to hit test. The widget is one row high, and buttons are left out below nine columns.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberInput<'a> {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: u8,
    prefix: Cow<'a, str>,
    suffix: Cow<'a, str>,
    edit: Option<(Cow<'a, str>, usize)>,
    state: WidgetState,
    style: Option<Style>,
}

impl<'a> NumberInput<'a> {
    /// A field showing `value`, unbounded, stepping by one, with no decimals.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self {
            value,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            step: 1.0,
            decimals: 0,
            prefix: Cow::Borrowed(""),
            suffix: Cow::Borrowed(""),
            edit: None,
            state: WidgetState::Normal,
            style: None,
        }
    }

    /// Limits the value to `min..=max`. The bounds are put in order if they are given backwards.
    #[must_use]
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min.min(max);
        self.max = min.max(max);
        self
    }

    /// How far one [`nudge`](NumberInput::nudge) step moves the value. A step that is not a
    /// positive number counts as one.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = if step > 0.0 { step } else { 1.0 };
        self
    }

    /// Digits shown after the decimal point; the value is rounded to them.
    #[must_use]
    pub const fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// Text before the number, such as a currency sign.
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<Cow<'a, str>>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Text after the number, such as a unit.
    #[must_use]
    pub fn suffix(mut self, suffix: impl Into<Cow<'a, str>>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Shows the text being typed with a caret before its `caret`th character, instead of the
    /// formatted value.
    #[must_use]
    pub fn edit(mut self, text: impl Into<Cow<'a, str>>, caret: usize) -> Self {
        self.edit = Some((text.into(), caret));
        self
    }

    /// Interaction state: focused draws the number in the accent color, disabled dims everything.
    #[must_use]
    pub const fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Draws the field's text in `style` instead of theme colors.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    fn clamp(&self, value: f64) -> f64 {
        if value.is_nan() {
            self.min.max(0.0).min(self.max)
        } else {
            value.clamp(self.min, self.max)
        }
    }

    fn round(&self, value: f64) -> f64 {
        let scale = 10_f64.powi(i32::from(self.decimals));
        (value * scale).round() / scale
    }

    /// The value moved by `steps` steps (negative to go down), rounded to the decimals and kept
    /// within the range.
    #[must_use]
    pub fn nudge(&self, steps: i32) -> f64 {
        let moved = self.clamp(self.value) + f64::from(steps) * self.step;
        self.clamp(self.round(moved))
    }

    /// The value shown with this widget's decimals, never as a negative zero.
    #[must_use]
    pub fn format(&self, value: f64) -> String {
        let value = self.clamp(self.round(value));
        let places = usize::from(self.decimals);
        let text = format!("{value:.places$}");
        match text.strip_prefix('-') {
            Some(rest) if rest.bytes().all(|byte| matches!(byte, b'0' | b'.')) => rest.to_owned(),
            _ => text,
        }
    }

    /// Whether `text` could still become a number as the user types: it may be empty or end in a
    /// sign or a decimal point. A minus sign needs a negative lower bound, a point needs decimals,
    /// and digits after the point are limited to the decimals.
    #[must_use]
    pub fn accepts(&self, text: &str) -> bool {
        if text.len() > MAX_TEXT {
            return false;
        }
        let digits = text
            .strip_prefix('-')
            .map_or(text, |rest| if self.min < 0.0 { rest } else { text });
        let mut parts = digits.splitn(2, '.');
        let whole = parts.next().unwrap_or("");
        let fraction = parts.next();
        whole.bytes().all(|byte| byte.is_ascii_digit())
            && fraction.is_none_or(|fraction| {
                self.decimals > 0
                    && fraction.len() <= usize::from(self.decimals)
                    && fraction.bytes().all(|byte| byte.is_ascii_digit())
            })
    }

    /// The number in `text`, rounded to the decimals and kept within the range, or `None` when
    /// the text is empty or only a sign or point.
    #[must_use]
    pub fn parse(&self, text: &str) -> Option<f64> {
        if !self.accepts(text) || !text.bytes().any(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let value: f64 = text.parse().ok()?;
        Some(self.clamp(self.round(value)))
    }

    /// Whether the value is at the lower bound, so the minus button is spent.
    #[must_use]
    pub fn at_min(&self) -> bool {
        self.value <= self.min
    }

    /// Whether the value is at the upper bound, so the plus button is spent.
    #[must_use]
    pub fn at_max(&self) -> bool {
        self.value >= self.max
    }

    fn button_width(area: Rect) -> u16 {
        if area.width >= MIN_WITH_BUTTONS {
            BUTTON
        } else {
            0
        }
    }

    /// The cells of the minus button in `area` (empty below nine columns).
    #[must_use]
    pub fn minus_area(&self, area: Rect) -> Rect {
        Rect::new(area.x, area.y, Self::button_width(area), area.height.min(1))
    }

    /// The cells of the plus button in `area` (empty below nine columns).
    #[must_use]
    pub fn plus_area(&self, area: Rect) -> Rect {
        let button = Self::button_width(area);
        Rect::new(
            area.x + area.width - button,
            area.y,
            button,
            area.height.min(1),
        )
    }

    /// The cells of the number field in `area`, between the buttons.
    #[must_use]
    pub fn field_area(&self, area: Rect) -> Rect {
        let button = Self::button_width(area);
        Rect::new(
            area.x + button,
            area.y,
            area.width - button * 2,
            area.height.min(1),
        )
    }

    fn shown(&self) -> String {
        let number = self
            .edit
            .as_ref()
            .map_or_else(|| self.format(self.value), |(text, _)| text.to_string());
        format!("{}{number}{}", self.prefix, self.suffix)
    }

    fn field_style(&self, cx: &RenderCx<'_>) -> Style {
        let disabled = matches!(self.state, WidgetState::Disabled);
        let focused = matches!(self.state, WidgetState::Focused);
        let invalid = self
            .edit
            .as_ref()
            .is_some_and(|(text, _)| self.parse(text).is_none());
        let fg = match self.style {
            Some(style) => style.fg,
            None if invalid && !disabled => cx.token(Token::Fail),
            None if focused => cx.token(Token::Accent),
            None => cx.token(Token::TextPrimary),
        };
        Style::new()
            .fg(fg)
            .bg(cx.token(Token::Surface1))
            .with_attrs(Attrs::new().set_dim(disabled).set_bold(focused))
    }

    fn draw_button(&self, cx: &mut RenderCx<'_>, area: Rect, glyph: &str, spent: bool) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let disabled = matches!(self.state, WidgetState::Disabled);
        let style = Style::new()
            .fg(cx.token(Token::Accent))
            .bg(cx.token(Token::Surface1))
            .with_attrs(Attrs::new().set_dim(disabled || spent));
        cx.put_str(area.x, area.y, &format!(" {glyph} "), style);
    }
}

impl Widget for NumberInput<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.draw_button(cx, self.minus_area(area), "−", self.at_min());
        self.draw_button(cx, self.plus_area(area), "+", self.at_max());
        let field = self.field_area(area);
        let style = self.field_style(cx);
        cx.put_str(
            field.x,
            field.y,
            &" ".repeat(usize::from(field.width)),
            style,
        );
        let inner = field.width.saturating_sub(2);
        let shown = self.shown();
        let clipped = clip_display_prefix(&shown, inner);
        let used = u16::try_from(width(clipped)).unwrap_or(inner);
        let text_x = field.x + 1 + (inner - used);
        cx.put_str(text_x, field.y, clipped, style);
        let editing = matches!(self.state, WidgetState::Focused);
        if let (Some((text, caret)), true) = (&self.edit, editing) {
            let before: String = text.chars().take(*caret).collect();
            let offset = width(&self.prefix) + width(&before);
            let caret_x = text_x.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            if caret_x < field.x + field.width {
                let under = text.chars().nth(*caret).unwrap_or(' ');
                let inverted = Style::new().fg(cx.token(Token::Surface1)).bg(style.fg);
                cx.put_str(caret_x, field.y, &under.to_string(), inverted);
            }
        }
    }

    fn measure(&self, avail: Size) -> Size {
        if avail.width == 0 || avail.height == 0 {
            return Size::new(0, 0);
        }
        let longest = [self.min, self.max, self.value]
            .into_iter()
            .filter(|value| value.is_finite())
            .map(|value| width(&self.format(value)))
            .max()
            .unwrap_or(1)
            + width(&self.prefix)
            + width(&self.suffix);
        let natural = u16::try_from(longest + 2 + usize::from(BUTTON) * 2).unwrap_or(u16::MAX);
        Size::new(natural.min(avail.width), 1)
    }
}
