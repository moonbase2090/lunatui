// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_button::{Button, Toggle, WidgetState};
use lunatui_core::{ClusterTable, Grid, Rect, Size};
use lunatui_field::{Field, Form};
use lunatui_runtime::{AccessibilityRole as Role, FocusManager, HitId, HitMap, LayerId};
use lunatui_term::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseKind};
use lunatui_theme::{Theme, Variant};
use lunatui_widgets::{
    ButtonState, NamedControl, RenderCx, ScrollView, ScrollbarVisibility, TextInput, Widget,
    WidgetList, WidgetListState,
};

fn draw(widget: &dyn Widget, hits: &mut HitMap, area: Rect) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(60, 14);
    let mut clusters = ClusterTable::new();
    widget.render(
        area,
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).hit_map(hits),
    );
    (grid, clusters)
}

#[test]
fn mixed_form_registers_names_and_body_roles_without_exporting_values_or_messages()
-> Result<(), Box<dyn std::error::Error>> {
    let input = TextInput::new("private-token-123", 17).show_cursor(false);
    let alerts = Toggle::new("By email", true).state(WidgetState::Disabled);
    let region = TextInput::new("private-region", 0);
    let form = Form::new([
        Field::new("Token").body(&input),
        Field::new("Notifications").body(&alerts),
        Field::new("Region")
            .name("Deployment region")
            .body(&region)
            .help("private helper")
            .error("private validation")
            .layer(Some(LayerId(8)))
            .z(200),
    ])
    .first_id(HitId(11));
    let mut hits = HitMap::new();
    let (grid, clusters) = draw(&form, &mut hits, Rect::new(2, 1, 50, 10));
    assert!(grid.row_display(1, &clusters).contains("private-token-123"));
    assert_eq!(
        hits.controls()
            .map(|c| (c.id, c.name, c.role, c.disabled, c.area, c.z, c.layer))
            .collect::<Vec<_>>(),
        [
            (
                HitId(11),
                "Token",
                Role::TextInput,
                false,
                Rect::new(17, 1, 35, 1),
                0,
                None
            ),
            (
                HitId(12),
                "Notifications",
                Role::Checkbox,
                true,
                Rect::new(17, 4, 35, 1),
                0,
                None
            ),
            (
                HitId(13),
                "Deployment region",
                Role::TextInput,
                false,
                Rect::new(17, 7, 35, 1),
                200,
                Some(LayerId(8))
            ),
        ]
    );
    assert_eq!(form.step(Some(0), true), Some(2));
    assert_eq!(hits.pick(18, 4), None);
    let mut focus = FocusManager::new();
    for control in hits.controls().filter(|c| !c.disabled) {
        focus.register(control.id, control.area);
    }
    focus.focus(HitId(11));
    let mut output = Vec::new();
    hits.write_accessibility_json(&mut output, focus.focused())?;
    let text = String::from_utf8(output)?;
    assert!(text.contains("\"name\":\"Token\""));
    assert!(text.contains("\"name\":\"Notifications\",\"disabled\":true"));
    assert!(text.contains("\"name\":\"Deployment region\""));
    for private in [
        "private-token-123",
        "private-region",
        "private helper",
        "private validation",
    ] {
        assert!(!text.contains(private));
    }
    assert!(text.contains("\"id\":11,\"role\":\"text_input\",\"name\":\"Token\",\"disabled\":false,\"layer\":null,\"area\":{\"x\":17,\"y\":1,\"width\":35,\"height\":1},\"z\":0,\"focused\":true"));
    let event = Event::Key(KeyEvent::press(KeyCode::Tab));
    if let Event::Key(key) = event
        && key.code == KeyCode::Tab
    {
        focus.tab(false);
    }
    assert_eq!(focus.focused(), Some(HitId(13)));
    let mut output = Vec::new();
    hits.write_accessibility_json(&mut output, focus.focused())?;
    assert!(String::from_utf8(output)?.contains("\"id\":13,\"role\":\"text_input\",\"name\":\"Deployment region\",\"disabled\":false,\"layer\":8,\"area\":{\"x\":17,\"y\":7,\"width\":35,\"height\":1},\"z\":200,\"focused\":true"));
    Ok(())
}

#[test]
fn real_pointer_events_activate_the_named_button_only_when_its_widget_is_enabled()
-> Result<(), Box<dyn std::error::Error>> {
    for (disabled, expected) in [(true, 0), (false, 1)] {
        let button = Button::new("Deploy").state(if disabled {
            WidgetState::Disabled
        } else {
            WidgetState::Normal
        });
        let control = NamedControl::new(HitId(5), Role::Button, &button);
        let mut hits = HitMap::new();
        draw(&control, &mut hits, Rect::new(2, 2, 12, 1));
        let registered = hits
            .find_control(Role::Button, "Deploy", None)?
            .ok_or("expected registered control")?;
        assert_eq!(registered.disabled, disabled);
        let mut state = ButtonState::new();
        let mut clicks = 0;
        for event in [
            Event::Mouse(MouseEvent::new(MouseKind::Down(MouseButton::Left), 3, 2)),
            Event::Mouse(MouseEvent::new(MouseKind::Up(MouseButton::Left), 3, 2)),
        ] {
            if let Event::Mouse(mouse) = event
                && hits.pick(mouse.column, mouse.row) == Some(HitId(5))
            {
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) => {
                        state.pointer_down(mouse.column, mouse.row, registered.area);
                    }
                    MouseKind::Up(MouseButton::Left) => {
                        clicks +=
                            usize::from(state.pointer_up(mouse.column, mouse.row, registered.area));
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(clicks, expected);
    }
    Ok(())
}

#[test]
fn scrolled_control_geometry_matches_the_visible_cells_and_omits_hidden_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let input = TextInput::new("0123456789", 0);
    let field = Field::new("Filter")
        .id(HitId(22))
        .layer(Some(LayerId(8)))
        .z(300)
        .body(&input);
    let view = ScrollView::new(field, Size::new(20, 5))
        .scrollbars(ScrollbarVisibility::Never)
        .offset(3, 2);
    let mut hits = HitMap::new();
    let (grid, clusters) = draw(&view, &mut hits, Rect::new(4, 3, 6, 2));
    assert_eq!(
        grid.row_display(3, &clusters)
            .chars()
            .skip(4)
            .take(6)
            .collect::<String>(),
        "345678"
    );
    let control = hits
        .find_control(Role::TextInput, "Filter", Some(LayerId(8)))?
        .ok_or("expected registered control")?;
    assert_eq!(
        (control.id, control.area, control.z),
        (HitId(22), Rect::new(4, 3, 6, 2), 300)
    );
    assert_eq!(hits.pick(4, 3), Some(HitId(22)));
    hits.clear();
    draw(&view.offset(3, 4), &mut hits, Rect::new(4, 3, 6, 1));
    assert_eq!(hits.controls().count(), 0);
    Ok(())
}

#[test]
fn a_field_without_a_visible_label_uses_its_explicit_name() -> Result<(), Box<dyn std::error::Error>>
{
    let input = TextInput::new("value-is-not-a-name", 0);
    let field = Field::new("").name("Search").id(HitId(31)).body(&input);
    let mut hits = HitMap::new();
    draw(&field, &mut hits, Rect::new(2, 1, 30, 3));
    let control = hits
        .find_control(Role::TextInput, "Search", None)?
        .ok_or("expected registered control")?;
    assert_eq!(control.id, HitId(31));
    assert_eq!(control.area, Rect::new(2, 2, 30, 2));
    Ok(())
}

#[test]
fn a_partially_visible_list_item_keeps_its_name_and_screen_coordinates()
-> Result<(), Box<dyn std::error::Error>> {
    let list = WidgetList::new(
        1,
        |_, _| 3,
        |_, area, _, cx: &mut RenderCx<'_>| {
            let input = TextInput::new("value", 0).show_cursor(false);
            NamedControl::new(HitId(77), Role::TextInput, &input)
                .name("List entry")
                .z(41)
                .render(area, cx);
        },
    )
    .state(WidgetListState {
        selected: None,
        offset: 0,
        skip: 1,
    });
    let mut hits = HitMap::new();
    let (grid, clusters) = draw(&list, &mut hits, Rect::new(3, 2, 10, 1));
    assert!(grid.row_display(2, &clusters).contains("value"));
    let control = hits
        .find_control(Role::TextInput, "List entry", None)?
        .ok_or("missing list entry")?;
    assert_eq!(
        (control.id, control.area, control.z),
        (HitId(77), Rect::new(3, 2, 10, 1), 41)
    );
    assert_eq!(hits.pick(3, 2), Some(HitId(77)));
    Ok(())
}

#[test]
fn narrow_form_bodies_and_hits_stay_inside_the_form_rectangle()
-> Result<(), Box<dyn std::error::Error>> {
    let input = TextInput::new("value", 0).show_cursor(false);
    let form = Form::new([Field::new("Label").body(&input)]).first_id(HitId(7));
    let mut hits = HitMap::new();
    let (grid, clusters) = draw(&form, &mut hits, Rect::new(3, 1, 1, 2));
    assert_eq!(grid.row_display(1, &clusters).trim_end(), "   v");
    let control = hits
        .find_control(Role::TextInput, "Label", None)?
        .ok_or("missing narrow control")?;
    assert_eq!(control.area, Rect::new(3, 1, 1, 1));
    assert_eq!(hits.pick(3, 1), Some(HitId(7)));
    assert_eq!(hits.pick(5, 1), None);
    Ok(())
}

#[test]
#[should_panic(expected = "unlabeled controls require .name()")]
fn a_form_rejects_an_unnamed_control() {
    let _ = Form::new([Field::new(" ")]);
}

#[test]
#[should_panic(expected = "unlabeled controls require .name()")]
fn rendering_an_unnamed_input_control_is_a_hard_error() {
    let input = TextInput::new("a-value-cannot-label-itself", 0);
    let control = NamedControl::new(HitId(9), Role::TextInput, &input);
    draw(&control, &mut HitMap::new(), Rect::new(0, 0, 30, 1));
}
