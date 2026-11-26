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
    AccessibilityRole, AmbiguousControl, Change, ChangeContext, ChangeReason, ChangeSource,
    Control, HitId, HitMap, LayerId, Outcome,
};

/// Most controls returned by one [`Agent::query`] call.
pub const CONTROL_QUERY_LIMIT: usize = 100;

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
            AccessibilityRole::Checkbox | AccessibilityRole::Disclosure => {
                &[Self::Focus, Self::Toggle]
            }
            AccessibilityRole::TagInput => &[Self::Focus, Self::SetValue, Self::Activate],
            AccessibilityRole::Calendar => &[Self::Focus, Self::SetValue],
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
    /// The application's [`Agent::allow`] hook refused the action.
    Denied,
    /// The request waits for the application to [`confirm`](Agent::confirm) or
    /// [`decline`](Agent::decline) it; nothing is queued yet.
    NeedsConfirmation(ConfirmId),
    /// [`Agent::plan`] was called while another plan is still running or waiting.
    Busy,
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
            Self::Denied => f.write_str("the application does not allow this action"),
            Self::NeedsConfirmation(_) => {
                f.write_str("the action waits for the application to confirm it")
            }
            Self::Busy => f.write_str("another plan is still running"),
        }
    }
}

impl std::error::Error for ActError {}

/// What the application lets an agent do with one action, as answered by the hook given to
/// [`Agent::allow`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Queue the request.
    Allow,
    /// Refuse it with [`ActError::Denied`].
    Deny,
    /// Hold it until the application confirms or declines it, and answer
    /// [`ActError::NeedsConfirmation`].
    Confirm,
}

impl Permission {
    /// The answer without a hook: [`Confirm`](Self::Confirm) for an action that would change a
    /// control marked [`destructive`](crate::ControlMetadata::destructive), and
    /// [`Allow`](Self::Allow) for everything else, focus included. A hook can call it for the
    /// controls it has no rule for.
    #[must_use]
    pub const fn default_for(control: &Control<'_>, action: &Action) -> Self {
        if control.is_destructive() && !matches!(action, Action::Focus) {
            Self::Confirm
        } else {
            Self::Allow
        }
    }
}

/// Names one request that waits for the application's confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConfirmId(u64);

/// A request held by [`Agent::act`] until the application confirms or declines it, as listed by
/// [`Agent::awaiting`]. It has what the application needs to ask the person.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AwaitingConfirmation {
    /// What [`Agent::confirm`] and [`Agent::decline`] take.
    pub id: ConfirmId,
    /// The control the request is for.
    pub control: HitId,
    /// Its accessible name.
    pub name: String,
    /// Its semantic role.
    pub role: AccessibilityRole,
    /// The overlay layer that owns it, or `None` for the base layer.
    pub layer: Option<LayerId>,
    /// What the agent asked for.
    pub action: Action,
    /// Who asked.
    pub source: Source,
}

/// One step of a plan given to [`Agent::plan`]: which control and what to do with it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Step {
    /// Semantic role of the control.
    pub role: AccessibilityRole,
    /// Its exact accessible name.
    pub name: String,
    /// The overlay layer that owns it, or `None` for the base layer.
    pub layer: Option<LayerId>,
    /// What to do.
    pub action: Action,
}

impl Step {
    /// Asks for `action` on the control `selector` names.
    #[must_use]
    pub fn new(selector: Selector<'_>, action: Action) -> Self {
        Self {
            role: selector.role,
            name: selector.name.to_owned(),
            layer: selector.layer,
            action,
        }
    }
}

/// Names one plan started with [`Agent::plan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlanId(u64);

/// Where a plan is, as answered by [`Agent::plan_status`]. `step` counts from 0 in the order
/// the steps were given.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStatus {
    /// The plan is on `step`: the next [`Agent::advance`] checks it, or its request waits for
    /// the app to [`take`](Agent::take) it.
    Running {
        /// The step the plan is on.
        step: usize,
    },
    /// `step` waits for the application to [`confirm`](Agent::confirm) or
    /// [`decline`](Agent::decline) the request `confirm`.
    Waiting {
        /// The step that waits.
        step: usize,
        /// The request listed in [`Agent::awaiting`].
        confirm: ConfirmId,
    },
    /// The app took the request of every step.
    Done,
    /// `step` failed with `error`. The steps before it were taken by the app; no step after
    /// it was queued.
    Failed {
        /// The step that failed.
        step: usize,
        /// Why it failed. A declined step fails with [`ActError::Denied`].
        error: ActError,
    },
    /// [`Agent::cancel_plan`] stopped the plan.
    Cancelled,
}

struct Plan {
    id: PlanId,
    steps: Vec<Step>,
    step: usize,
    phase: Phase,
}

enum Phase {
    Ready,
    Queued(Ticket),
    Waiting(ConfirmId),
    Finished(PlanStatus),
}

impl Plan {
    const fn status(&self) -> PlanStatus {
        match self.phase {
            Phase::Ready | Phase::Queued(_) => PlanStatus::Running { step: self.step },
            Phase::Waiting(confirm) => PlanStatus::Waiting {
                step: self.step,
                confirm,
            },
            Phase::Finished(status) => status,
        }
    }

    fn fail(&mut self, error: ActError) {
        self.phase = Phase::Finished(PlanStatus::Failed {
            step: self.step,
            error,
        });
    }
}

enum Checked {
    Queued(Ticket),
    Waiting(ConfirmId),
}

type AllowHook = Box<dyn FnMut(&Control<'_>, &Action) -> Permission + Send + Sync>;

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
    /// The keyboard shortcut a person sees for the control, such as `Ctrl+S`.
    pub shortcut: Option<String>,
    /// The letter that chooses the control while its menu is open.
    pub mnemonic: Option<char>,
    /// Whether acting on the control destroys work. By default [`Agent::act`] holds a request
    /// that would change it until the application confirms it.
    pub destructive: bool,
    /// Disabled, focused, selected and invalid, in that order, for those that hold.
    pub states: Vec<ControlState>,
    /// What [`Agent::act`] accepts. Empty when the control is disabled.
    pub actions: Vec<ActionKind>,
    /// The overlay layer that owns the control, or `None` for the base layer.
    pub parent: Option<LayerId>,
    /// The control's cells.
    pub area: Rect,
}

/// Filters controls by accessible name, semantic kind, owning layer, or a combination.
///
/// Constructing a query requires at least one filter. Additional filters are combined with it;
/// a control must match all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlQuery<'a> {
    text: Option<&'a str>,
    kind: Option<AccessibilityRole>,
    layer: LayerFilter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayerFilter {
    Any,
    Exact(Option<LayerId>),
}

impl<'a> ControlQuery<'a> {
    /// Finds controls whose accessible name contains `text`, ignoring Unicode case.
    #[must_use]
    pub const fn text(text: &'a str) -> Self {
        Self {
            text: Some(text),
            kind: None,
            layer: LayerFilter::Any,
        }
    }

    /// Finds controls with this semantic role.
    #[must_use]
    pub const fn kind(kind: AccessibilityRole) -> Self {
        Self {
            text: None,
            kind: Some(kind),
            layer: LayerFilter::Any,
        }
    }

    /// Finds controls owned by `layer`. `None` selects the base layer.
    #[must_use]
    pub const fn layer(layer: Option<LayerId>) -> Self {
        Self {
            text: None,
            kind: None,
            layer: LayerFilter::Exact(layer),
        }
    }

    /// Also requires the accessible name to contain `text`, ignoring Unicode case.
    #[must_use]
    pub const fn with_text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    /// Also requires this semantic role.
    #[must_use]
    pub const fn with_kind(mut self, kind: AccessibilityRole) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Also requires ownership by `layer`. `None` selects the base layer.
    #[must_use]
    pub const fn in_layer(mut self, layer: Option<LayerId>) -> Self {
        self.layer = LayerFilter::Exact(layer);
        self
    }

    fn matches(self, control: Control<'_>, text: Option<&str>) -> bool {
        text.is_none_or(|text| control.name.to_lowercase().contains(text))
            && self.kind.is_none_or(|kind| control.role == kind)
            && match self.layer {
                LayerFilter::Any => true,
                LayerFilter::Exact(layer) => control.layer == layer,
            }
    }
}

/// A bounded set of controls returned by [`Agent::query`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlQueryResult {
    controls: Vec<AgentControl>,
    more: usize,
}

impl ControlQueryResult {
    /// Matching controls in registration order, up to [`CONTROL_QUERY_LIMIT`].
    #[must_use]
    pub fn controls(&self) -> &[AgentControl] {
        &self.controls
    }

    /// Matching controls omitted after [`CONTROL_QUERY_LIMIT`] was reached.
    #[must_use]
    pub const fn more(&self) -> usize {
        self.more
    }

    /// Writes the result as one JSON object and a newline. Like a [`Snapshot`], it may carry
    /// control values, so keep it off any public surface.
    ///
    /// # Errors
    ///
    /// Propagates writer errors.
    pub fn write_json(&self, mut writer: impl Write) -> io::Result<()> {
        writer.write_all(b"{\"version\":1,\"controls\":[")?;
        write_controls(&mut writer, &self.controls)?;
        writeln!(writer, "],\"more\":{}}}", self.more)
    }
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
        write_controls(&mut writer, &self.controls)?;
        writer.write_all(b"]}\n")
    }
}

fn write_controls(writer: &mut impl Write, controls: &[AgentControl]) -> io::Result<()> {
    for (index, control) in controls.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        write_control(writer, control)?;
    }
    Ok(())
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
    writer.write_all(b",\"shortcut\":")?;
    match &control.shortcut {
        Some(shortcut) => write_string(writer, shortcut)?,
        None => writer.write_all(b"null")?,
    }
    writer.write_all(b",\"mnemonic\":")?;
    match control.mnemonic {
        Some(mnemonic) => write_string(writer, mnemonic.encode_utf8(&mut [0; 4]))?,
        None => writer.write_all(b"null")?,
    }
    write!(writer, ",\"destructive\":{}", control.destructive)?;
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
///
/// Before queueing, [`act`](Self::act) asks the hook set with [`allow`](Self::allow), or
/// [`Permission::default_for`] when there is none, whether the action may go ahead. A request
/// that needs confirmation waits in [`awaiting`](Self::awaiting) until the application, on the
/// person's word, calls [`confirm`](Self::confirm) or [`decline`](Self::decline); an agent has
/// no way to confirm its own request.
///
/// [`plan`](Self::plan) gives several steps at once. The app calls [`advance`](Self::advance)
/// after each render, and the agent checks and queues one step per frame, so a step can name a
/// control that an earlier step made appear or enabled. The plan stops at the first step that
/// fails and [`plan_status`](Self::plan_status) says which one and why.
#[derive(Default)]
pub struct Agent {
    queue: VecDeque<Request>,
    allow: Option<AllowHook>,
    awaiting: Vec<AwaitingConfirmation>,
    next_confirm: u64,
    plan: Option<Plan>,
    next_plan: u64,
}

impl fmt::Debug for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Agent")
            .field("queue", &self.queue)
            .field("allow", &self.allow.as_ref().map(|_| "hook"))
            .field("awaiting", &self.awaiting)
            .field(
                "plan",
                &self.plan.as_ref().map(|plan| (plan.id, plan.status())),
            )
            .finish_non_exhaustive()
    }
}

impl Agent {
    /// An agent with nothing queued.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            allow: None,
            awaiting: Vec::new(),
            next_confirm: 0,
            plan: None,
            next_plan: 0,
        }
    }

    /// Decides what [`act`](Self::act) does with each action on an enabled control that takes
    /// it, in place of [`Permission::default_for`]. The hook sees the control as registered in
    /// the frame and the action, and replaces any earlier hook.
    pub fn allow(
        &mut self,
        hook: impl FnMut(&Control<'_>, &Action) -> Permission + Send + Sync + 'static,
    ) {
        self.allow = Some(Box::new(hook));
    }

    /// Every named control in `hits`, with `focused` marking the one with focus.
    #[must_use]
    pub fn snapshot(hits: &HitMap, focused: Option<HitId>) -> Snapshot {
        let controls = hits
            .controls()
            .map(|control| agent_control(control, focused))
            .collect();
        Snapshot { controls }
    }

    /// Finds controls by accessible-name text, semantic kind, owning layer, or a combination.
    ///
    /// Matches keep registration order. Disabled controls are included, as in [`Self::snapshot`].
    /// At most [`CONTROL_QUERY_LIMIT`] controls are returned; [`ControlQueryResult::more`]
    /// reports how many additional controls matched.
    #[must_use]
    pub fn query(
        hits: &HitMap,
        focused: Option<HitId>,
        query: ControlQuery<'_>,
    ) -> ControlQueryResult {
        let text = query.text.map(str::to_lowercase);
        let mut controls = Vec::new();
        let mut more = 0;
        for control in hits
            .controls()
            .filter(|control| query.matches(*control, text.as_deref()))
        {
            if controls.len() < CONTROL_QUERY_LIMIT {
                controls.push(agent_control(control, focused));
            } else {
                more += 1;
            }
        }
        ControlQueryResult { controls, more }
    }

    /// Queues `action` for the control `selector` names in `hits`.
    ///
    /// # Errors
    ///
    /// [`ActError::NotFound`] and [`ActError::Ambiguous`] when the selector does not name exactly
    /// one control, [`ActError::Disabled`] for a disabled control,
    /// [`ActError::Unsupported`] when its role does not take the action, [`ActError::Denied`]
    /// when the application refuses it, and [`ActError::NeedsConfirmation`] when the request
    /// waits for the application's confirmation.
    pub fn act(
        &mut self,
        hits: &HitMap,
        selector: Selector<'_>,
        action: Action,
    ) -> Result<Ticket, ActError> {
        match self.check(hits, selector, action)? {
            Checked::Queued(ticket) => Ok(ticket),
            Checked::Waiting(id) => Err(ActError::NeedsConfirmation(id)),
        }
    }

    fn check(
        &mut self,
        hits: &HitMap,
        selector: Selector<'_>,
        action: Action,
    ) -> Result<Checked, ActError> {
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
        let permission = match &mut self.allow {
            Some(hook) => hook(&control, &action),
            None => Permission::default_for(&control, &action),
        };
        match permission {
            Permission::Allow => Ok(Checked::Queued(self.queue_request(control.id, action))),
            Permission::Deny => Err(ActError::Denied),
            Permission::Confirm => {
                let id = ConfirmId(self.next_confirm);
                self.next_confirm += 1;
                self.awaiting.push(AwaitingConfirmation {
                    id,
                    control: control.id,
                    name: control.name.to_owned(),
                    role: control.role,
                    layer: control.layer,
                    action,
                    source: Source::Agent,
                });
                Ok(Checked::Waiting(id))
            }
        }
    }

    /// Starts a plan of `steps`. Nothing is checked or queued until the next
    /// [`advance`](Self::advance); a plan with no steps is [`PlanStatus::Done`] at once.
    ///
    /// # Errors
    ///
    /// [`ActError::Busy`] while an earlier plan is still running or waiting.
    pub fn plan(&mut self, steps: impl IntoIterator<Item = Step>) -> Result<PlanId, ActError> {
        if self
            .plan
            .as_ref()
            .is_some_and(|plan| !matches!(plan.phase, Phase::Finished(_)))
        {
            return Err(ActError::Busy);
        }
        let id = PlanId(self.next_plan);
        self.next_plan += 1;
        let steps: Vec<Step> = steps.into_iter().collect();
        let phase = if steps.is_empty() {
            Phase::Finished(PlanStatus::Done)
        } else {
            Phase::Ready
        };
        self.plan = Some(Plan {
            id,
            steps,
            step: 0,
            phase,
        });
        Ok(id)
    }

    /// Checks the plan's current step against `hits` and queues it, as [`act`](Self::act)
    /// would. Call it once after each render. It does nothing while the step's request waits
    /// for the app to [`take`](Self::take) it or for the application's confirmation, so one
    /// step goes out per frame. A step that fails ends the plan.
    pub fn advance(&mut self, hits: &HitMap) {
        let Some(mut plan) = self.plan.take() else {
            return;
        };
        if matches!(plan.phase, Phase::Ready) {
            let step = &plan.steps[plan.step];
            let selector = Selector::new(step.role, &step.name).layer(step.layer);
            match self.check(hits, selector, step.action.clone()) {
                Ok(Checked::Queued(ticket)) => plan.phase = Phase::Queued(ticket),
                Ok(Checked::Waiting(id)) => plan.phase = Phase::Waiting(id),
                Err(error) => plan.fail(error),
            }
        }
        self.plan = Some(plan);
    }

    /// Where the plan `id` is, or `None` when `id` is not the latest plan.
    #[must_use]
    pub fn plan_status(&self, id: PlanId) -> Option<PlanStatus> {
        self.plan
            .as_ref()
            .filter(|plan| plan.id == id)
            .map(Plan::status)
    }

    /// Stops the plan `id`: its queued request is cancelled, its waiting one leaves
    /// [`awaiting`](Self::awaiting), and no later step is queued. Answers whether the plan was
    /// still running or waiting.
    pub fn cancel_plan(&mut self, id: PlanId) -> bool {
        let Some(plan) = self.plan.as_mut().filter(|plan| plan.id == id) else {
            return false;
        };
        match &plan.phase {
            Phase::Finished(_) => return false,
            Phase::Ready => {}
            Phase::Queued(ticket) => ticket.cancel(),
            Phase::Waiting(confirm) => {
                let confirm = *confirm;
                self.awaiting.retain(|waiting| waiting.id != confirm);
            }
        }
        plan.phase = Phase::Finished(PlanStatus::Cancelled);
        true
    }

    /// Requests that wait for the application's confirmation, oldest first.
    #[must_use]
    pub fn awaiting(&self) -> &[AwaitingConfirmation] {
        &self.awaiting
    }

    /// Queues the waiting request `id` once the person agrees. Call it from the application's
    /// own confirmation, never on an agent's behalf. The request leaves
    /// [`awaiting`](Self::awaiting) whatever the result. A plan step that waited on it goes on
    /// when it is queued and fails with the error when it is not.
    ///
    /// # Errors
    ///
    /// [`ActError::NotFound`] when `id` is not waiting, or when `hits` no longer has its control
    /// under the same name, role, layer and identity; [`ActError::Ambiguous`] when several
    /// controls now match; [`ActError::Disabled`] when the control became disabled.
    pub fn confirm(&mut self, hits: &HitMap, id: ConfirmId) -> Result<Ticket, ActError> {
        let result = self.confirm_waiting(hits, id);
        if let Some(plan) = self.waiting_plan(id) {
            match &result {
                Ok(ticket) => plan.phase = Phase::Queued(ticket.clone()),
                Err(error) => plan.fail(*error),
            }
        }
        result
    }

    fn confirm_waiting(&mut self, hits: &HitMap, id: ConfirmId) -> Result<Ticket, ActError> {
        let index = self
            .awaiting
            .iter()
            .position(|waiting| waiting.id == id)
            .ok_or(ActError::NotFound)?;
        let waiting = self.awaiting.remove(index);
        let control = hits
            .find_control(waiting.role, &waiting.name, waiting.layer)
            .map_err(ActError::Ambiguous)?
            .filter(|control| control.id == waiting.control)
            .ok_or(ActError::NotFound)?;
        if control.disabled {
            return Err(ActError::Disabled);
        }
        Ok(self.queue_request(control.id, waiting.action))
    }

    /// Drops the waiting request `id`, answering whether it was waiting. A plan step that
    /// waited on it fails with [`ActError::Denied`].
    pub fn decline(&mut self, id: ConfirmId) -> bool {
        let before = self.awaiting.len();
        self.awaiting.retain(|waiting| waiting.id != id);
        if let Some(plan) = self.waiting_plan(id) {
            plan.fail(ActError::Denied);
        }
        self.awaiting.len() != before
    }

    fn waiting_plan(&mut self, id: ConfirmId) -> Option<&mut Plan> {
        self.plan
            .as_mut()
            .filter(|plan| matches!(plan.phase, Phase::Waiting(waiting) if waiting == id))
    }

    fn queue_request(&mut self, id: HitId, action: Action) -> Ticket {
        let ticket = Ticket {
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        self.queue.push_back(Request {
            id,
            action,
            source: Source::Agent,
            ticket: ticket.clone(),
        });
        ticket
    }

    /// Watches the controls `query` matches; see [`Subscription`].
    #[must_use]
    pub fn subscribe(query: ControlQuery<'_>) -> Subscription {
        Subscription {
            text: query.text.map(str::to_owned),
            kind: query.kind,
            layer: query.layer,
            seen: None,
        }
    }

    /// The oldest request that was not cancelled.
    pub fn take(&mut self) -> Option<Request> {
        while let Some(request) = self.queue.pop_front() {
            if !request.ticket.is_cancelled() {
                if let Some(plan) = self.plan.as_mut().filter(|plan| {
                    matches!(&plan.phase, Phase::Queued(ticket)
                        if Arc::ptr_eq(&ticket.cancelled, &request.ticket.cancelled))
                }) {
                    plan.step += 1;
                    plan.phase = if plan.step == plan.steps.len() {
                        Phase::Finished(PlanStatus::Done)
                    } else {
                        Phase::Ready
                    };
                }
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

/// What a [`Subscription`] compares for one control between frames.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ControlValue {
    /// The value, when the control has one that is not secret.
    pub value: Option<String>,
    /// Whether an on/off control is on.
    pub checked: Option<bool>,
    /// Disabled, focused, selected and invalid, in that order, for those that hold.
    pub states: Vec<ControlState>,
}

/// One subscribed control that changed between two observed frames.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ControlChange {
    /// Application-defined identity.
    pub id: HitId,
    /// Accessible name in the newer frame, or the last one seen when the control went away.
    pub name: String,
    /// Semantic role.
    pub role: AccessibilityRole,
    /// The overlay layer that owns the control, or `None` for the base layer.
    pub parent: Option<LayerId>,
    /// The control before the change, or `None` when it appeared.
    pub previous: Option<ControlValue>,
    /// The control after the change, or `None` when it went away.
    pub current: Option<ControlValue>,
    /// Why it changed, when the application recorded the change with
    /// [`ControlMetadata::changed`](crate::ControlMetadata::changed).
    pub reason: Option<ChangeReason>,
    /// Where the change came from, under the same condition as [`reason`](Self::reason).
    pub source: Option<ChangeSource>,
}

/// Watches the controls a [`ControlQuery`] matches and reports what changed between frames.
///
/// The first [`observe`](Self::observe) only records the controls; each later one returns the
/// controls whose value, checked state or states differ from the previous frame, and those that
/// appeared or went away, in registration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    text: Option<String>,
    kind: Option<AccessibilityRole>,
    layer: LayerFilter,
    seen: Option<Vec<Seen>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    id: HitId,
    name: String,
    role: AccessibilityRole,
    parent: Option<LayerId>,
    value: ControlValue,
}

impl Subscription {
    /// Compares the controls in `hits` with those of the previous call and returns the changes.
    pub fn observe(&mut self, hits: &HitMap, focused: Option<HitId>) -> Vec<ControlChange> {
        let query = ControlQuery {
            text: self.text.as_deref(),
            kind: self.kind,
            layer: self.layer,
        };
        let text = self.text.as_deref().map(str::to_lowercase);
        let mut now = Vec::new();
        let mut changes = Vec::new();
        for control in hits
            .controls()
            .filter(|control| query.matches(*control, text.as_deref()))
        {
            let agent = agent_control(control, focused);
            let value = ControlValue {
                value: agent.value,
                checked: agent.checked,
                states: agent.states,
            };
            if let Some(seen) = &self.seen {
                let before = seen.iter().find(|old| old.id == control.id);
                if before.is_none_or(|old| old.value != value) {
                    changes.push(ControlChange {
                        id: control.id,
                        name: agent.name.clone(),
                        role: control.role,
                        parent: control.layer,
                        previous: before.map(|old| old.value.clone()),
                        current: Some(value.clone()),
                        reason: control.change.map(|change| change.reason()),
                        source: control.change.map(|change| change.source()),
                    });
                }
            }
            now.push(Seen {
                id: control.id,
                name: agent.name,
                role: control.role,
                parent: control.layer,
                value,
            });
        }
        if let Some(seen) = &self.seen {
            for gone in seen
                .iter()
                .filter(|old| now.iter().all(|new| new.id != old.id))
            {
                changes.push(ControlChange {
                    id: gone.id,
                    name: gone.name.clone(),
                    role: gone.role,
                    parent: gone.parent,
                    previous: Some(gone.value.clone()),
                    current: None,
                    reason: None,
                    source: None,
                });
            }
        }
        self.seen = Some(now);
        changes
    }
}

fn agent_control(control: Control<'_>, focused: Option<HitId>) -> AgentControl {
    AgentControl {
        id: control.id,
        name: control.name.to_owned(),
        role: control.role,
        value: control.value.map(str::to_owned),
        checked: control.checked,
        shortcut: control.shortcut.map(str::to_owned),
        mnemonic: control.mnemonic,
        destructive: control.is_destructive(),
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
    }
}

/// State an agent's [`Action`] can change directly.
pub trait Act {
    /// Applies `action`, answering [`Outcome::Ignored`] for one this state does not take.
    fn act(&mut self, action: &Action) -> Outcome;
}

/// Lends [`Act`] to a closure, for state whose rules live beside it rather than in it, such as
/// a number held as a plain value and bounded by a widget: `request.apply(&mut ActWith(|action|
/// slider.act(&mut value, action)), context)`.
pub struct ActWith<F>(pub F);

impl<F: FnMut(&Action) -> Outcome> Act for ActWith<F> {
    fn act(&mut self, action: &Action) -> Outcome {
        (self.0)(action)
    }
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
