// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::borrow::Cow;

use lunatui_core::{Attrs, Rect, Size, Style, clip_display_prefix, width};
use lunatui_runtime::{AccessibilityRole, ControlMetadata, HitId, LayerId};
use lunatui_ui_tokens::{Role, surface, text};
use lunatui_widgets::{RenderCx, Widget};

mod form;
mod state;
pub use form::{FieldArea, Form};
pub use state::FormState;

const MARKER: &str = " *";

/// A label, a control widget, and a help or error line under it.
///
/// # Panics
///
/// Rendering panics when the selected accessible name is empty or whitespace,
/// in every build profile and even without an attached hit map. An explicit
/// [`name`](Self::name) overrides the label. Give blank labels a nonblank name
/// before rendering or passing the field to [`Form::new`]. Rendering with an
/// attached hit map also requires an explicit [`HitId`].
#[derive(Clone)]
pub struct Field<'a> {
    pub(crate) label: Cow<'a, str>,
    name: Option<Cow<'a, str>>,
    id: Option<HitId>,
    role: Option<AccessibilityRole>,
    layer: Option<LayerId>,
    z: u16,
    pub(crate) input_rows: u16,
    pub(crate) help: Cow<'a, str>,
    pub(crate) error: Option<Cow<'a, str>>,
    pub(crate) required: bool,
    focused: bool,
    pub(crate) disabled: bool,
    body: Option<&'a dyn Widget>,
    on: Role,
}

impl<'a> Field<'a> {
    /// A field with this label and no control, help or error.
    /// Name validation is deferred so [`name`](Self::name) can follow this call.
    ///
    /// # Panics
    ///
    /// Rendering the returned field or passing it to [`Form::new`] panics if
    /// its selected accessible name is empty or whitespace. This applies in
    /// every build profile, including rendering without an attached hit map.
    /// For a blank label, set a nonblank `.name()` before either operation.
    #[must_use]
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            name: None,
            id: None,
            role: None,
            layer: None,
            z: 0,
            input_rows: 1,
            help: Cow::Borrowed(""),
            error: None,
            required: false,
            focused: false,
            disabled: false,
            body: None,
            on: Role::Background,
        }
    }

    /// Muted help text under the control, shown while there is no error.
    #[must_use]
    pub fn help(mut self, help: impl Into<Cow<'a, str>>) -> Self {
        self.help = help.into();
        self
    }

    /// A validation message under the control in the destructive color. It replaces the help.
    /// An empty string clears the optional error and shows the help again.
    #[must_use]
    pub fn error(mut self, error: impl Into<Cow<'a, str>>) -> Self {
        let error = error.into();
        self.error = (!error.is_empty()).then_some(error);
        self
    }

    /// Marks the label with a `*`.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Draws the label in the ring color.
    #[must_use]
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Dims the label, the help and the error.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The control drawn between the label and the message.
    #[must_use]
    pub const fn body(mut self, body: &'a dyn Widget) -> Self {
        self.body = Some(body);
        self
    }

    /// The surface role the field sits on, `Role::Background` by default.
    #[must_use]
    pub const fn on(mut self, on: Role) -> Self {
        self.on = on;
        self
    }

    /// Accessible name override for a control with no visible label.
    #[must_use]
    pub fn name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Stable application identity used by the frame's `HitMap`.
    #[must_use]
    pub const fn id(mut self, id: HitId) -> Self {
        self.id = Some(id);
        self
    }

    /// Overrides the body widget's role. A body without a role defaults to text input.
    #[must_use]
    pub const fn role(mut self, role: AccessibilityRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Owning overlay layer, or `None` for the base layer.
    #[must_use]
    pub const fn layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = layer;
        self
    }

    /// Exact registration depth for this control.
    #[must_use]
    pub const fn z(mut self, z: u16) -> Self {
        self.z = z;
        self
    }

    /// Input height used by a `Form` when no body widget is supplied.
    #[must_use]
    pub fn input_rows(mut self, rows: u16) -> Self {
        self.input_rows = rows.max(1);
        self
    }

    /// Current validation error, never included in control metadata.
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the field or its enclosed control is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled || self.body.is_some_and(Widget::control_disabled)
    }

    /// Required nonempty accessible name, from the label unless overridden.
    ///
    /// # Panics
    ///
    /// Panics when the selected name is empty or whitespace. An explicit
    /// [`name`](Self::name) overrides the label, even when that name is blank.
    #[must_use]
    pub fn accessible_name(&self) -> &str {
        let name = self.name.as_deref().unwrap_or(&self.label);
        assert!(
            !name.trim().is_empty(),
            "unlabeled controls require .name()"
        );
        name
    }

    fn message(&self) -> &str {
        self.error.as_deref().unwrap_or(&self.help)
    }

    pub(crate) fn register(&self, area: Rect, cx: &mut RenderCx<'_>, fallback: Option<HitId>) {
        let name = self.accessible_name();
        cx.register_control(self.id.or(fallback), area, self.z, || {
            ControlMetadata::new(
                name,
                self.role
                    .or_else(|| self.body.and_then(Widget::control_role))
                    .unwrap_or(AccessibilityRole::TextInput),
            )
            .disabled(self.is_disabled())
            .layer(self.layer)
        });
    }

    pub(crate) fn render_body(&self, area: Rect, cx: &mut RenderCx<'_>) {
        if let Some(body) = self.body {
            body.render(area, cx);
        }
    }

    fn dim(&self, style: Style) -> Style {
        if self.is_disabled() {
            style.with_attrs(style.attrs.set_dim(true))
        } else {
            style
        }
    }

    fn body_rows(&self, avail: Size) -> u16 {
        self.body.map_or(0, |b| b.measure(avail).height.max(1))
    }
}

impl Widget for Field<'_> {
    fn render(&self, area: Rect, cx: &mut RenderCx<'_>) {
        let _ = self.accessible_name();
        if area.width == 0 || area.height == 0 {
            return;
        }
        let page = surface(cx, self.on);
        cx.fill(area, page, ' ');
        let label_color = if self.focused {
            cx.token(Role::Ring.token())
        } else {
            text(cx, Role::Foreground).fg
        };
        let label = self.dim(page.fg(label_color).with_attrs(Attrs::new().set_bold(true)));
        let shown = clip_display_prefix(&self.label, area.width);
        cx.put_str(area.x, area.y, shown, label);
        if self.required {
            let used = u16::try_from(width(shown)).unwrap_or(area.width);
            let marker = self.dim(page.fg(text(cx, Role::Destructive).fg));
            let room = area.width.saturating_sub(used);
            cx.put_str(
                area.x.saturating_add(used),
                area.y,
                clip_display_prefix(MARKER, room),
                marker,
            );
        }
        let rest = area.height - 1;
        let body_rows = self.body_rows(Size::new(area.width, rest)).min(rest);
        if let Some(body) = self.body
            && body_rows > 0
        {
            body.render(
                Rect::new(area.x, area.y.saturating_add(1), area.width, body_rows),
                cx,
            );
        }
        self.register(
            if body_rows > 0 {
                Rect::new(area.x, area.y.saturating_add(1), area.width, body_rows)
            } else {
                area
            },
            cx,
            None,
        );
        let message = self.message();
        if !message.is_empty() && rest > body_rows {
            let color = if self.error.is_none() {
                text(cx, Role::Muted).fg
            } else {
                text(cx, Role::Destructive).fg
            };
            let style = self.dim(page.fg(color));
            let shown = clip_display_prefix(message, area.width);
            cx.put_str(
                area.x,
                area.y.saturating_add(1).saturating_add(body_rows),
                shown,
                style,
            );
        }
    }

    fn measure(&self, avail: Size) -> Size {
        let label = width(&self.label) + if self.required { width(MARKER) } else { 0 };
        let body = self.body.map_or(Size::new(0, 0), |b| b.measure(avail));
        let widest = label.max(width(self.message()));
        let w = u16::try_from(widest).unwrap_or(u16::MAX).max(body.width);
        let h = 1_u16
            .saturating_add(self.body_rows(avail))
            .saturating_add(u16::from(!self.message().is_empty()));
        Size::new(w, h).clamp(avail)
    }
}
