// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_choice::{CheckState, Checkbox};
use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_field::Field;
use lunatui_runtime::{
    AccessibilityRole as Role, Action, Agent, ChangeContext, ChangeDetails, ChangeReason,
    ChangeSource, ControlChange, ControlMetadata, ControlQuery, Handle, HitId, HitMap, Selector,
    Subscription,
};
use lunatui_term::{Event, MouseButton, MouseKind};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RadioGroup, RadioState, RenderCx, Widget};

const PRIORITIES: [&str; 3] = ["Low", "Medium", "High"];
const PRIORITY: HitId = HitId(1);
const TERMS: HitId = HitId(2);
const TERMS_ROW: u16 = 4;

struct App {
    radio: RadioState,
    terms: bool,
    priority_change: Option<ChangeDetails>,
    terms_change: Option<ChangeDetails>,
    agent: Agent,
    hits: HitMap,
    subscription: Subscription,
    changes: Vec<ControlChange>,
}

impl App {
    fn new(query: ControlQuery<'_>) -> Self {
        let mut radio = RadioState::new();
        radio.sync(&RadioGroup::new(PRIORITIES));
        radio.select(0);
        Self {
            radio,
            terms: false,
            priority_change: None,
            terms_change: None,
            agent: Agent::new(),
            hits: HitMap::new(),
            subscription: Agent::subscribe(query),
            changes: Vec::new(),
        }
    }

    fn toggle_terms(&mut self, mut context: ChangeContext<'_>) {
        let terms = &mut self.terms;
        let change = context.apply(ChangeReason::Toggle, (), || *terms = !*terms);
        if change.value().is_some() {
            self.terms_change = change.details().copied();
        }
    }
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        if let Event::Mouse(mouse) = event
            && mouse.kind == MouseKind::Down(MouseButton::Left)
        {
            if mouse.row == TERMS_ROW {
                self.toggle_terms(ChangeContext::for_event(event));
            }
            return;
        }
        let change = self.radio.handle(event);
        if change.value().is_some() {
            self.priority_change = change.details().copied();
        }
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            if request.id == TERMS && matches!(request.action, Action::Toggle) {
                self.toggle_terms(ChangeContext::new(ChangeSource::Agent));
            }
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        {
            let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
            let group = RadioGroup::new(PRIORITIES).select(self.radio.selected());
            let radio_area = Rect::new(0, 0, area.width, 3);
            group.render(radio_area, &mut cx);
            let chosen = self
                .radio
                .selected()
                .and_then(|index| PRIORITIES.get(index))
                .copied()
                .unwrap_or("");
            let priority_change = self.priority_change.take();
            cx.register_control(Some(PRIORITY), radio_area, 0, || {
                let metadata = ControlMetadata::new("Priority", Role::ComboBox).value(chosen);
                match priority_change {
                    Some(details) => metadata.changed(details),
                    None => metadata,
                }
            });
            let checkbox = Checkbox::new("Accept terms").checked(if self.terms {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            });
            let field = Field::new("Terms")
                .name("Accept terms")
                .body(&checkbox)
                .id(TERMS);
            let field = match self.terms_change.take() {
                Some(details) => field.changed(details),
                None => field,
            };
            field.render(Rect::new(0, TERMS_ROW, area.width, 3), &mut cx);
        }
        self.changes
            .extend(self.subscription.observe(&self.hits, None));
    }
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn mount(query: ControlQuery<'_>) -> Harness<App> {
    Harness::mount(App::new(query), 40, 10)
}

fn take(harness: &mut Harness<App>) -> Vec<ControlChange> {
    std::mem::take(&mut harness.component_mut().changes)
}

#[test]
fn a_keyboard_choice_reports_its_reason_source_and_previous_value() -> Test {
    let mut harness = mount(ControlQuery::layer(None));
    assert_eq!(
        take(&mut harness),
        Vec::new(),
        "the first frame only primes"
    );
    harness.key("Down");
    let changes = take(&mut harness);
    let [change] = changes.as_slice() else {
        return Err(format!("one change, got {changes:?}").into());
    };
    assert_eq!((change.id, change.name.as_str()), (PRIORITY, "Priority"));
    assert_eq!(
        change
            .previous
            .as_ref()
            .and_then(|value| value.value.as_deref()),
        Some("Low")
    );
    assert_eq!(
        change
            .current
            .as_ref()
            .and_then(|value| value.value.as_deref()),
        Some("Medium")
    );
    assert_eq!(change.source, Some(ChangeSource::Keyboard));
    assert!(change.reason.is_some(), "the radio state reports why");
    Ok(())
}

#[test]
fn a_mouse_and_an_agent_toggle_report_their_own_source() -> Test {
    let mut harness = mount(ControlQuery::kind(Role::Checkbox));
    take(&mut harness);
    harness.mouse(MouseKind::Down(MouseButton::Left), 1, TERMS_ROW);
    let clicked = take(&mut harness);
    let [click] = clicked.as_slice() else {
        return Err(format!("one change from the click, got {clicked:?}").into());
    };
    assert_eq!(click.id, TERMS);
    assert_eq!(
        (click.reason, click.source),
        (Some(ChangeReason::Toggle), Some(ChangeSource::Mouse))
    );
    assert_eq!(
        click.previous.as_ref().and_then(|value| value.checked),
        Some(false)
    );
    assert_eq!(
        click.current.as_ref().and_then(|value| value.checked),
        Some(true)
    );

    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::Checkbox, "Accept terms"),
        Action::Toggle,
    )?;
    harness.advance(Duration::ZERO);
    let acted = take(&mut harness);
    let [act] = acted.as_slice() else {
        return Err(format!("one change from the agent, got {acted:?}").into());
    };
    assert_eq!(
        (act.reason, act.source),
        (Some(ChangeReason::Toggle), Some(ChangeSource::Agent))
    );
    assert_eq!(
        act.previous.as_ref().and_then(|value| value.checked),
        Some(true)
    );
    assert_eq!(
        act.current.as_ref().and_then(|value| value.checked),
        Some(false)
    );
    Ok(())
}

#[test]
fn an_unchanged_frame_reports_nothing_and_an_unrecorded_change_has_no_reason() -> Test {
    let mut harness = mount(ControlQuery::layer(None));
    take(&mut harness);
    harness.advance(Duration::from_millis(16));
    assert_eq!(take(&mut harness), Vec::new(), "nothing changed");
    harness.component_mut().radio.select(2);
    harness.advance(Duration::ZERO);
    let changes = take(&mut harness);
    let [change] = changes.as_slice() else {
        return Err(format!("one change, got {changes:?}").into());
    };
    assert_eq!(
        change
            .current
            .as_ref()
            .and_then(|value| value.value.as_deref()),
        Some("High")
    );
    assert_eq!((change.reason, change.source), (None, None));
    Ok(())
}

#[test]
fn a_subscription_watches_only_the_controls_its_query_matches() {
    let mut harness = mount(ControlQuery::text("terms"));
    take(&mut harness);
    harness.key("Down");
    assert_eq!(
        take(&mut harness),
        Vec::new(),
        "the priority is not watched"
    );
    harness.mouse(MouseKind::Down(MouseButton::Left), 1, TERMS_ROW);
    assert_eq!(
        take(&mut harness)
            .iter()
            .map(|change| change.id)
            .collect::<Vec<_>>(),
        vec![TERMS]
    );
}
