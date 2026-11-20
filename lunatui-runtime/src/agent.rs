// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use lunatui_core::Rect;

use crate::hit::write_string;
use crate::{
    AccessibilityRole, AmbiguousControl, Change, ChangeContext, ChangeReason, ChangeSource, HitId,
    HitMap, LayerId, Outcome,
};

/// What a control can be asked to do, as listed in a [`Snapshot`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    /// Move focus to the control.
    Focus,
    /// Press a button, open a list or choose the item.
    Activate,
    /// Flip an on/off control.
    Toggle,
    /// Choose this item in its group.
    Select,
    /// Replace the control's value.
    SetValue,
}

impl ActionKind {
    /// The name an agent reads in a snapshot.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Activate => "activate",
            Self::Toggle => "toggle",
            Self::Select => "select",
            Self::SetValue => "set_value",
        }
    }

    const fn for_role(role: AccessibilityRole) -> &'static [Self] {
        match role {
            AccessibilityRole::Button => &[Self::Focus, Self::Activate],
            AccessibilityRole::Tab | AccessibilityRole::RadioButton => &[Self::Focus, Self::Select],
            AccessibilityRole::ListItem | AccessibilityRole::MenuItem => {
                &[Self::Focus, Self::Select, Self::Activate]
            }
            AccessibilityRole::TextInput | AccessibilityRole::Slider => {
                &[Self::Focus, Self::SetValue]
            }
            AccessibilityRole::Checkbox => &[Self::Focus, Self::Toggle],
            AccessibilityRole::ComboBox => &[Self::Focus, Self::Activate, Self::SetValue],
            _ => &[],
        }
    }
}

/// One thing an agent asks a control to do.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Move focus to the control.
    Focus,
    /// Press a button, open a list or choose the item.
    Activate,
    /// Flip an on/off control.
    Toggle,
    /// Choose this item in its group.
    Select,
    /// Replace the control's value.
    SetValue(String),
}

impl Action {
    /// The operation a change guard is asked to approve for this action.
    #[must_use]
    pub const fn reason(&self) -> ChangeReason {
        match self {
            Self::Focus => ChangeReason::Navigate,
            Self::Activate => ChangeReason::Activate,
            Self::Toggle => ChangeReason::Toggle,
            Self::Select => ChangeReason::Select,
            Self::SetValue(_) => ChangeReason::Edit,
        }
    }

    /// The kind of this action.
    #[must_use]
    pub const fn kind(&self) -> ActionKind {
        match self {
            Self::Focus => ActionKind::Focus,
            Self::Activate => ActionKind::Activate,
            Self::Toggle => ActionKind::Toggle,
            Self::Select => ActionKind::Select,
            Self::SetValue(_) => ActionKind::SetValue,
        }
    }
}

/// A condition of a control in a [`Snapshot`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlState {
    /// The control cannot be activated.
    Disabled,
    /// The control has focus.
    Focused,
    /// The item is the chosen one in its group.
    Selected,
    /// The control shows a validation error.
    Invalid,
}

impl ControlState {
    /// The name an agent reads in a snapshot.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Focused => "focused",
            Self::Selected => "selected",
            Self::Invalid => "invalid",
        }
    }
}

/// Who caused a [`Request`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// An agent called [`Agent::act`].
    Agent,
}

impl Source {
    /// The provenance a change guard sees for a request from this source.
    #[must_use]
    pub const fn change_source(self) -> ChangeSource {
        match self {
            Self::Agent => ChangeSource::Agent,
        }
    }
}

/// Which control to act on: its role, exact accessible name and owning layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selector<'a> {
    /// Semantic role.
    pub role: AccessibilityRole,
    /// Exact accessible name.
    pub name: &'a str,
    /// Owning overlay, or `None` for the base layer.
    pub layer: Option<LayerId>,
}

impl<'a> Selector<'a> {
    /// A control on the base layer.
    #[must_use]
    pub const fn new(role: AccessibilityRole, name: &'a str) -> Self {
        Self {
            role,
            name,
            layer: None,
        }
    }

    /// Looks in an overlay layer instead of the base layer.
    #[must_use]
    pub const fn layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = layer;
        self
    }
}

/// Why [`Agent::act`] did not queue a request.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActError {
    /// No control matches the selector.
    NotFound,
    /// Several controls match the selector.
    Ambiguous(AmbiguousControl),
    /// The control is disabled.
    Disabled,
    /// The control does not take this kind of action.
    Unsupported(ActionKind),
}

impl fmt::Display for ActError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("no control matches the selector"),
            Self::Ambiguous(error) => error.fmt(f),
            Self::Disabled => f.write_str("the control is disabled"),
            Self::Unsupported(kind) => {
                write!(f, "the control does not take the {} action", kind.name())
            }
        }
    }
}

impl std::error::Error for ActError {}

/// Cancels a queued [`Request`] before the app applies it.
#[derive(Clone, Debug)]
pub struct Ticket {
    cancelled: Arc<AtomicBool>,
}

impl Ticket {
    /// Drops the request if the app has not taken it yet.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Whether [`cancel`](Self::cancel) was called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// One queued action, taken from [`Agent::take`] and applied by the app to the state behind
/// control [`id`](Self::id).
#[derive(Clone, Debug)]
pub struct Request {
    /// The control the request is for.
    pub id: HitId,
    /// What to do.
    pub action: Action,
    /// Who asked.
    pub source: Source,
    ticket: Ticket,
}

impl Request {
    /// Applies this request to `state` once the guard in `context` approves it. The guard runs
    /// before `state` is touched, whatever the implementation of [`Act`], sees the reason for the
    /// action and the source of the request, and cannot change either. A refusal leaves `state`
    /// unchanged and the result has no accepted payload. An approved result carries the
    /// state's own [`Outcome`], which may be [`Outcome::Ignored`] or [`Outcome::Handled`].
    pub fn apply<A: Act + ?Sized>(
        &self,
        state: &mut A,
        mut context: ChangeContext<'_>,
    ) -> Change<Outcome> {
        context.perform(self.action.reason(), || state.act(&self.action))
    }

    /// A change context attributed to this request's source, with no guard attached.
    #[must_use]
    pub const fn context<'a>(&self) -> ChangeContext<'a> {
        ChangeContext::new(self.source.change_source())
    }

    /// The ticket that cancels this request, for an app that defers it.
    #[must_use]
    pub fn ticket(&self) -> &Ticket {
        &self.ticket
    }
}

/// One control as an agent sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentControl {
    /// Application-defined identity.
    pub id: HitId,
    /// Accessible name.
    pub name: String,
    /// Semantic role.
    pub role: AccessibilityRole,
    /// The current value, when the control has one that is not secret.
    pub value: Option<String>,
    /// Whether an on/off control is on, or `None` for a control that is not one.
    pub checked: Option<bool>,
    /// Disabled, focused, selected and invalid, in that order, for those that hold.
    pub states: Vec<ControlState>,
    /// What [`Agent::act`] accepts. Empty when the control is disabled.
    pub actions: Vec<ActionKind>,
    /// The overlay layer that owns the control, or `None` for the base layer.
    pub parent: Option<LayerId>,
    /// The control's cells.
    pub area: Rect,
}

/// Every named control of one frame, in registration order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    controls: Vec<AgentControl>,
}

impl Snapshot {
    /// The controls in registration order.
    #[must_use]
    pub fn controls(&self) -> &[AgentControl] {
        &self.controls
    }

    /// Finds one control by selector.
    ///
    /// # Errors
    ///
    /// [`AmbiguousControl`] when several controls match.
    pub fn find(&self, selector: Selector<'_>) -> Result<Option<&AgentControl>, AmbiguousControl> {
        let mut matching = self.controls.iter().filter(|c| {
            c.role == selector.role && c.name == selector.name && c.parent == selector.layer
        });
        let first = matching.next();
        let extra = matching.count();
        if extra == 0 {
            Ok(first)
        } else {
            Err(AmbiguousControl { matches: extra + 1 })
        }
    }

    /// Writes the snapshot as one JSON object and a newline. Unlike the accessibility export it
    /// carries values, so keep it off any public surface.
    ///
    /// # Errors
    ///
    /// Propagates writer errors.
    pub fn write_json(&self, mut writer: impl Write) -> io::Result<()> {
        writer.write_all(b"{\"version\":1,\"controls\":[")?;
        for (index, control) in self.controls.iter().enumerate() {
            if index > 0 {
                writer.write_all(b",")?;
            }
            write_control(&mut writer, control)?;
        }
        writer.write_all(b"]}\n")
    }
}

fn write_control(writer: &mut impl Write, control: &AgentControl) -> io::Result<()> {
    write!(writer, "{{\"id\":{},\"name\":", control.id.0)?;
    write_string(writer, &control.name)?;
    write!(
        writer,
        ",\"role\":\"{}\",\"value\":",
        control.role.wire_name()
    )?;
    match &control.value {
        Some(value) => write_string(writer, value)?,
        None => writer.write_all(b"null")?,
    }
    writer.write_all(b",\"checked\":")?;
    match control.checked {
        Some(checked) => write!(writer, "{checked}")?,
        None => writer.write_all(b"null")?,
    }
    writer.write_all(b",\"states\":[")?;
    for (index, state) in control.states.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        write!(writer, "\"{}\"", state.name())?;
    }
    writer.write_all(b"],\"actions\":[")?;
    for (index, action) in control.actions.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        write!(writer, "\"{}\"", action.name())?;
    }
    writer.write_all(b"],\"parent\":")?;
    match control.parent {
        Some(layer) => write!(writer, "{}", layer.0)?,
        None => writer.write_all(b"null")?,
    }
    write!(
        writer,
        ",\"area\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}",
        control.area.x, control.area.y, control.area.width, control.area.height
    )
}

/// Lets an agent read the named controls of the last frame and change them by name.
///
/// [`snapshot`](Self::snapshot) reads a frame's [`HitMap`]. [`act`](Self::act) finds a control by
/// role, name and layer, checks that it is enabled and takes the action, and queues a
/// [`Request`]. The app takes requests with [`take`](Self::take) in its update step, applies each
/// to the state behind [`Request::id`] and redraws, so an agent never types keys and nothing
/// waits on time. The returned [`Ticket`] cancels a request the app has not taken yet.
#[derive(Debug, Default)]
pub struct Agent {
    queue: VecDeque<Request>,
}

impl Agent {
    /// An agent with nothing queued.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Every named control in `hits`, with `focused` marking the one with focus.
    #[must_use]
    pub fn snapshot(hits: &HitMap, focused: Option<HitId>) -> Snapshot {
        let controls = hits
            .controls()
            .map(|control| AgentControl {
                id: control.id,
                name: control.name.to_owned(),
                role: control.role,
                value: control.value.map(str::to_owned),
                checked: control.checked,
                states: [
                    (ControlState::Disabled, control.disabled),
                    (ControlState::Focused, focused == Some(control.id)),
                    (ControlState::Selected, control.selected),
                    (ControlState::Invalid, control.invalid),
                ]
                .into_iter()
                .filter_map(|(state, holds)| holds.then_some(state))
                .collect(),
                actions: if control.disabled {
                    Vec::new()
                } else {
                    ActionKind::for_role(control.role).to_vec()
                },
                parent: control.layer,
                area: control.area,
            })
            .collect();
        Snapshot { controls }
    }

    /// Queues `action` for the control `selector` names in `hits`.
    ///
    /// # Errors
    ///
    /// [`ActError::NotFound`] and [`ActError::Ambiguous`] when the selector does not name exactly
    /// one control, [`ActError::Disabled`] for a disabled control, and
    /// [`ActError::Unsupported`] when its role does not take the action.
    pub fn act(
        &mut self,
        hits: &HitMap,
        selector: Selector<'_>,
        action: Action,
    ) -> Result<Ticket, ActError> {
        let control = hits
            .find_control(selector.role, selector.name, selector.layer)
            .map_err(ActError::Ambiguous)?
            .ok_or(ActError::NotFound)?;
        if control.disabled {
            return Err(ActError::Disabled);
        }
        let kind = action.kind();
        if !ActionKind::for_role(control.role).contains(&kind) {
            return Err(ActError::Unsupported(kind));
        }
        let ticket = Ticket {
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        self.queue.push_back(Request {
            id: control.id,
            action,
            source: Source::Agent,
            ticket: ticket.clone(),
        });
        Ok(ticket)
    }

    /// The oldest request that was not cancelled.
    pub fn take(&mut self) -> Option<Request> {
        while let Some(request) = self.queue.pop_front() {
            if !request.ticket.is_cancelled() {
                return Some(request);
            }
        }
        None
    }

    /// Requests waiting, cancelled ones included until [`take`](Self::take) skips them.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

/// State an agent's [`Action`] can change directly.
pub trait Act {
    /// Applies `action`, answering [`Outcome::Ignored`] for one this state does not take.
    fn act(&mut self, action: &Action) -> Outcome;
}

impl Act for bool {
    fn act(&mut self, action: &Action) -> Outcome {
        if matches!(action, Action::Toggle) {
            *self = !*self;
            Outcome::Changed
        } else {
            Outcome::Ignored
        }
    }
}

impl Act for String {
    fn act(&mut self, action: &Action) -> Outcome {
        let Action::SetValue(value) = action else {
            return Outcome::Ignored;
        };
        if self == value {
            Outcome::Handled
        } else {
            value.clone_into(self);
            Outcome::Changed
        }
    }
}
