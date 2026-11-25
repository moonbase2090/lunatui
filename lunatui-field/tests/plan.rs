// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use std::time::Duration;

use lunatui_button::Button;
use lunatui_choice::{CheckState, Checkbox};
use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_field::Field;
use lunatui_runtime::{
    AccessibilityRole as Role, Act, ActError, Action, ActionKind, Agent, HitId, HitMap, PlanId,
    PlanStatus, Selector, Step,
};
use lunatui_term::{Event, KeyCode};
use lunatui_test::{Component, Harness};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{RenderCx, TextInput, Widget};

const NAME: HitId = HitId(1);
const TERMS: HitId = HitId(2);
const SUBMIT: HitId = HitId(3);
const MORE: HitId = HitId(4);
const ARCHIVE: HitId = HitId(5);
const DELETE: HitId = HitId(6);
const PROMPT_ROW: u16 = 18;

/// A sign-up form whose Submit enables only once a name is typed and the terms are accepted,
/// a More button that shows an Archive button, and a destructive Delete draft.
#[derive(Default)]
struct App {
    name: String,
    terms: bool,
    submitted: u32,
    more_open: bool,
    archived: u32,
    deleted: u32,
    agent: Agent,
    hits: HitMap,
}

impl Component for App {
    fn update(&mut self, event: &Event) {
        let Event::Key(key) = event else {
            return;
        };
        let Some(waiting) = self.agent.awaiting().first().map(|waiting| waiting.id) else {
            return;
        };
        match key.code {
            KeyCode::Char('y') => {
                let _ = self.agent.confirm(&self.hits, waiting);
            }
            KeyCode::Char('n') => {
                self.agent.decline(waiting);
            }
            _ => {}
        }
    }

    fn tick(&mut self, _: Duration) {
        while let Some(request) = self.agent.take() {
            match (request.id, &request.action) {
                (NAME, action) => {
                    self.name.act(action);
                }
                (TERMS, action) => {
                    self.terms.act(action);
                }
                (SUBMIT, Action::Activate) => self.submitted += 1,
                (MORE, Action::Activate) => self.more_open = true,
                (ARCHIVE, Action::Activate) => self.archived += 1,
                (DELETE, Action::Activate) => self.deleted += 1,
                _ => {}
            }
        }
    }

    fn render(&mut self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        let theme = Theme::builtin(Variant::Dark);
        self.hits = HitMap::new();
        {
            let mut cx = RenderCx::new(grid, clusters, &theme).hit_map(&mut self.hits);
            let row = |index: u16| Rect::new(0, index * 3, area.width, 3);
            let name = TextInput::new(self.name.clone(), self.name.len()).show_cursor(false);
            Field::new("Name")
                .body(&name)
                .id(NAME)
                .render(row(0), &mut cx);
            let terms = Checkbox::new("Accept terms").checked(if self.terms {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            });
            Field::new("Terms")
                .name("Accept terms")
                .body(&terms)
                .id(TERMS)
                .render(row(1), &mut cx);
            let submit = Button::new("Submit");
            Field::new("Submit")
                .body(&submit)
                .id(SUBMIT)
                .disabled(self.name.is_empty() || !self.terms)
                .render(row(2), &mut cx);
            let more = Button::new("More");
            Field::new("More")
                .body(&more)
                .id(MORE)
                .render(row(3), &mut cx);
            if self.more_open {
                let archive = Button::new("Archive");
                Field::new("Archive")
                    .body(&archive)
                    .id(ARCHIVE)
                    .render(row(4), &mut cx);
            }
            let delete = Button::new("Delete");
            Field::new("Delete draft")
                .body(&delete)
                .id(DELETE)
                .destructive(true)
                .render(row(5), &mut cx);
            if let Some(waiting) = self.agent.awaiting().first() {
                let prompt = format!("Agent: {:?} {}? y/n", waiting.action, waiting.name);
                cx.put_str(0, PROMPT_ROW, &prompt, Style::default());
            }
        }
        self.agent.advance(&self.hits);
    }
}

fn mount() -> Harness<App> {
    Harness::mount(App::default(), 40, 20)
}

fn step(role: Role, name: &str, action: Action) -> Step {
    Step::new(Selector::new(role, name), action)
}

fn sign_up() -> [Step; 3] {
    [
        step(Role::TextInput, "Name", Action::SetValue("Ada".to_owned())),
        step(Role::Checkbox, "Accept terms", Action::Toggle),
        step(Role::Button, "Submit", Action::Activate),
    ]
}

type Test = Result<(), Box<dyn std::error::Error>>;

fn start(
    harness: &mut Harness<App>,
    steps: impl IntoIterator<Item = Step>,
) -> Result<PlanId, ActError> {
    harness.component_mut().agent.plan(steps)
}

fn frame(harness: &mut Harness<App>) {
    harness.advance(Duration::ZERO);
}

fn status(harness: &Harness<App>, plan: PlanId) -> Option<PlanStatus> {
    harness.component().agent.plan_status(plan)
}

#[test]
fn each_step_is_checked_against_the_frame_its_earlier_steps_drew() -> Test {
    let mut harness = mount();
    let plan = start(&mut harness, sign_up())?;
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Running { step: 0 })
    );
    let app = harness.component_mut();
    assert_eq!(
        app.agent
            .act(
                &app.hits,
                Selector::new(Role::Button, "Submit"),
                Action::Activate
            )
            .map(|_| ()),
        Err(ActError::Disabled),
        "Submit is disabled in the frame the plan started from"
    );
    assert_eq!(app.agent.pending(), 0);

    let mut seen = Vec::new();
    for _ in 0..4 {
        frame(&mut harness);
        let app = harness.component();
        seen.push((
            status(&harness, plan),
            app.name.clone(),
            app.terms,
            app.submitted,
        ));
    }
    assert_eq!(
        seen,
        [
            (
                Some(PlanStatus::Running { step: 0 }),
                String::new(),
                false,
                0
            ),
            (
                Some(PlanStatus::Running { step: 1 }),
                "Ada".to_owned(),
                false,
                0
            ),
            (
                Some(PlanStatus::Running { step: 2 }),
                "Ada".to_owned(),
                true,
                0
            ),
            (Some(PlanStatus::Done), "Ada".to_owned(), true, 1),
        ],
        "one step per frame, and Submit goes out once the frame shows it enabled"
    );
    frame(&mut harness);
    assert_eq!(status(&harness, plan), Some(PlanStatus::Done));
    assert_eq!(
        harness.component().submitted,
        1,
        "a done plan queues nothing more"
    );
    Ok(())
}

#[test]
fn a_step_can_name_a_control_an_earlier_step_made_appear() -> Test {
    let mut harness = mount();
    let plan = start(
        &mut harness,
        [
            step(Role::Button, "More", Action::Activate),
            step(Role::Button, "Archive", Action::Activate),
        ],
    )?;
    for _ in 0..3 {
        frame(&mut harness);
    }
    assert_eq!(status(&harness, plan), Some(PlanStatus::Done));
    assert_eq!(harness.component().archived, 1);
    Ok(())
}

#[test]
fn the_plan_stops_at_the_first_failing_step_and_names_it() -> Test {
    let mut harness = mount();
    let plan = start(
        &mut harness,
        [
            step(Role::Checkbox, "Accept terms", Action::Toggle),
            step(Role::Button, "Publish", Action::Activate),
            step(Role::Button, "More", Action::Activate),
        ],
    )?;
    for _ in 0..4 {
        frame(&mut harness);
    }
    let app = harness.component();
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Failed {
            step: 1,
            error: ActError::NotFound
        })
    );
    assert!(app.terms, "the step before the failure was applied");
    assert!(!app.more_open, "no step after the failure was queued");
    assert_eq!(app.agent.pending(), 0);

    let mut harness = mount();
    let plan = start(
        &mut harness,
        [
            step(Role::TextInput, "Name", Action::SetValue("Ada".to_owned())),
            step(Role::Button, "Submit", Action::Activate),
            step(Role::Button, "More", Action::Activate),
        ],
    )?;
    for _ in 0..4 {
        frame(&mut harness);
    }
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Failed {
            step: 1,
            error: ActError::Disabled
        }),
        "Submit stays disabled without the terms"
    );
    assert_eq!(harness.component().submitted, 0);
    assert!(!harness.component().more_open);

    let mut harness = mount();
    let plan = start(&mut harness, [step(Role::Button, "More", Action::Toggle)])?;
    frame(&mut harness);
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Failed {
            step: 0,
            error: ActError::Unsupported(ActionKind::Toggle)
        })
    );
    Ok(())
}

#[test]
fn a_second_plan_is_busy_until_the_first_ends() -> Test {
    let mut harness = mount();
    let first = start(&mut harness, sign_up())?;
    assert_eq!(
        harness.component_mut().agent.plan(sign_up()),
        Err(ActError::Busy)
    );
    for _ in 0..4 {
        frame(&mut harness);
    }
    assert_eq!(status(&harness, first), Some(PlanStatus::Done));
    let second = start(
        &mut harness,
        [step(Role::Button, "Submit", Action::Activate)],
    )?;
    assert_ne!(first, second);
    assert_eq!(
        status(&harness, first),
        None,
        "only the latest plan is kept"
    );
    frame(&mut harness);
    frame(&mut harness);
    assert_eq!(status(&harness, second), Some(PlanStatus::Done));
    assert_eq!(harness.component().submitted, 2);

    let mut harness = mount();
    let waiting = start(
        &mut harness,
        [step(Role::Button, "Delete draft", Action::Activate)],
    )?;
    frame(&mut harness);
    assert!(matches!(
        status(&harness, waiting),
        Some(PlanStatus::Waiting { step: 0, .. })
    ));
    assert_eq!(
        harness.component_mut().agent.plan([]),
        Err(ActError::Busy),
        "a plan waiting for the person is still running"
    );
    Ok(())
}

#[test]
fn a_destructive_step_waits_for_the_person() -> Test {
    let mut harness = mount();
    let plan = start(
        &mut harness,
        [
            step(Role::Button, "Delete draft", Action::Activate),
            step(Role::Button, "More", Action::Activate),
        ],
    )?;
    frame(&mut harness);
    let confirm = match status(&harness, plan) {
        Some(PlanStatus::Waiting { step: 0, confirm }) => confirm,
        other => return Err(format!("the first step waits, got {other:?}").into()),
    };
    assert_eq!(
        harness
            .component()
            .agent
            .awaiting()
            .first()
            .map(|waiting| waiting.id),
        Some(confirm)
    );
    frame(&mut harness);
    frame(&mut harness);
    let app = harness.component();
    assert_eq!(
        app.deleted, 0,
        "nothing is deleted before the person agrees"
    );
    assert!(!app.more_open, "the plan does not move past a waiting step");
    assert_eq!(
        harness.rows()[usize::from(PROMPT_ROW)],
        "Agent: Activate Delete draft? y/n"
    );

    harness.key("y");
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Running { step: 0 })
    );
    for _ in 0..2 {
        frame(&mut harness);
    }
    assert_eq!(status(&harness, plan), Some(PlanStatus::Done));
    let app = harness.component();
    assert_eq!(app.deleted, 1);
    assert!(app.more_open, "the plan went on after the confirmation");
    assert_eq!(harness.rows()[usize::from(PROMPT_ROW)], "");
    Ok(())
}

#[test]
fn a_declined_step_fails_the_plan() -> Test {
    let mut harness = mount();
    let plan = start(
        &mut harness,
        [
            step(Role::Button, "Delete draft", Action::Activate),
            step(Role::Button, "More", Action::Activate),
        ],
    )?;
    frame(&mut harness);
    harness.key("n");
    for _ in 0..2 {
        frame(&mut harness);
    }
    let app = harness.component();
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Failed {
            step: 0,
            error: ActError::Denied
        })
    );
    assert_eq!(app.deleted, 0);
    assert!(!app.more_open);
    assert_eq!(app.agent.awaiting(), []);
    Ok(())
}

#[test]
fn confirming_after_the_control_went_disabled_fails_the_step() -> Test {
    let mut harness = mount();
    let plan = start(
        &mut harness,
        [
            step(Role::TextInput, "Name", Action::SetValue("Ada".to_owned())),
            step(Role::Checkbox, "Accept terms", Action::Toggle),
            step(Role::Button, "Submit", Action::Activate),
        ],
    )?;
    harness.component_mut().agent.allow(|control, _| {
        if control.name == "Submit" {
            lunatui_runtime::Permission::Confirm
        } else {
            lunatui_runtime::Permission::Allow
        }
    });
    for _ in 0..3 {
        frame(&mut harness);
    }
    assert!(matches!(
        status(&harness, plan),
        Some(PlanStatus::Waiting { step: 2, .. })
    ));
    harness.component_mut().terms = false;
    frame(&mut harness);
    harness.key("y");
    assert_eq!(
        status(&harness, plan),
        Some(PlanStatus::Failed {
            step: 2,
            error: ActError::Disabled
        })
    );
    frame(&mut harness);
    assert_eq!(harness.component().submitted, 0);
    Ok(())
}

#[test]
fn cancelling_drops_the_queued_or_waiting_step_and_everything_after_it() -> Test {
    let mut harness = mount();
    let plan = start(&mut harness, sign_up())?;
    frame(&mut harness);
    assert_eq!(
        harness.component().agent.pending(),
        1,
        "the first step is queued"
    );
    assert!(harness.component_mut().agent.cancel_plan(plan));
    assert!(
        !harness.component_mut().agent.cancel_plan(plan),
        "a stopped plan cannot be cancelled again"
    );
    for _ in 0..3 {
        frame(&mut harness);
    }
    let app = harness.component();
    assert_eq!(status(&harness, plan), Some(PlanStatus::Cancelled));
    assert_eq!(app.name, "", "the queued step was cancelled");
    assert!(!app.terms);

    let mut harness = mount();
    let plan = start(
        &mut harness,
        [step(Role::Button, "Delete draft", Action::Activate)],
    )?;
    frame(&mut harness);
    assert_eq!(harness.component().agent.awaiting().len(), 1);
    assert!(harness.component_mut().agent.cancel_plan(plan));
    assert_eq!(harness.component().agent.awaiting(), []);
    frame(&mut harness);
    assert_eq!(harness.component().deleted, 0);
    assert_eq!(status(&harness, plan), Some(PlanStatus::Cancelled));
    Ok(())
}

#[test]
fn an_empty_plan_is_done_at_once() -> Test {
    let mut harness = mount();
    let plan = start(&mut harness, [])?;
    assert_eq!(status(&harness, plan), Some(PlanStatus::Done));
    assert!(!harness.component_mut().agent.cancel_plan(plan));
    assert_eq!(ActError::Busy.to_string(), "another plan is still running");
    Ok(())
}

#[test]
fn only_the_plan_step_moves_the_plan_on() -> Test {
    let mut harness = mount();
    let plan = start(&mut harness, sign_up())?;
    let app = harness.component_mut();
    app.agent.act(
        &app.hits,
        Selector::new(Role::Button, "More"),
        Action::Activate,
    )?;
    app.agent.advance(&app.hits);
    assert_eq!(
        app.agent.pending(),
        2,
        "the act request, then the first step"
    );
    let first = app.agent.take().map(|request| request.id);
    assert_eq!(first, Some(MORE));
    assert_eq!(
        app.agent.plan_status(plan),
        Some(PlanStatus::Running { step: 0 }),
        "taking another request leaves the plan on its step"
    );
    let second = app.agent.take().map(|request| request.id);
    assert_eq!(second, Some(NAME));
    assert_eq!(
        app.agent.plan_status(plan),
        Some(PlanStatus::Running { step: 1 })
    );
    Ok(())
}
