// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{Cell, ClusterTable, Color, Grid, Rect, Style};
use lunatui_runtime::{FocusManager, HitId, LayerEvent, LayerStack};
use lunatui_term::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseKind};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    Dialog, DialogClosed, DialogFocus, DialogKind, DialogOutcome, DialogState, RenderCx, Widget,
};

const DELETE: &str = "Delete feature/login? Its 3 unmerged commits will be lost.";

fn draw(dialog: &Dialog<'_>, width: u16, height: u16) -> (Grid, ClusterTable) {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(width, height);
    let mut clusters = ClusterTable::new();
    let mut cx = RenderCx::new(&mut grid, &mut clusters, &theme);
    let dots = ".".repeat(usize::from(width));
    for y in 0..height {
        cx.put_str(0, y, &dots, Style::new().fg(Color::Rgb(200, 200, 200)));
    }
    dialog.render(Rect::new(0, 0, width, height), &mut cx);
    (grid, clusters)
}

fn rows(dialog: &Dialog<'_>, width: u16, height: u16) -> Vec<String> {
    let (grid, clusters) = draw(dialog, width, height);
    (0..height)
        .map(|y| grid.row_display(y, &clusters))
        .collect()
}

fn style_at(grid: &Grid, x: u16, y: u16) -> Style {
    grid.get(x, y).map(Cell::style).unwrap_or_default()
}

fn token(token: Token) -> Color {
    Theme::builtin(Variant::Dark).resolve(token)
}

fn delete() -> Dialog<'static> {
    Dialog::confirm("Delete branch", DELETE)
        .width(36)
        .confirm_label("Delete")
        .destructive(true)
}

fn opened(kind: DialogKind) -> DialogState {
    let mut state = DialogState::new();
    state.open(kind, &mut LayerStack::new(), None);
    state
}

const SCREEN: Rect = Rect::new(0, 0, 44, 12);
const OPENER: HitId = HitId(1);
const LIST: HitId = HitId(2);

struct Screen {
    layers: LayerStack,
    focus: FocusManager,
    state: DialogState,
    answers: Vec<DialogOutcome>,
    list_presses: u32,
}

impl Screen {
    fn with_open(kind: DialogKind) -> Self {
        let mut screen = Self {
            layers: LayerStack::new(),
            focus: FocusManager::new(),
            state: DialogState::new(),
            answers: Vec::new(),
            list_presses: 0,
        };
        screen.frame();
        screen.focus.focus(LIST);
        screen
            .state
            .open(kind, &mut screen.layers, screen.focus.focused());
        screen.frame();
        screen
    }

    fn frame(&mut self) {
        if self.state.is_open() {
            self.state.place(&mut self.layers, delete().area(SCREEN));
        }
        let base = vec![
            (OPENER, Rect::new(0, 0, 4, 1)),
            (LIST, Rect::new(0, 1, 44, 10)),
        ];
        self.focus.sync(self.layers.focus_entries(base));
    }

    fn answered(&mut self, closed: Option<DialogClosed>) {
        self.answers.extend(closed.map(|closed| closed.outcome()));
    }

    fn event(&mut self, event: &Event) {
        let route = self.layers.handle(event, &mut self.focus);
        let closed = self.state.sync(&self.layers);
        self.answered(closed);
        let deliver = match route {
            LayerEvent::Pass => true,
            LayerEvent::Consumed => false,
            LayerEvent::Dismissed { deliver, .. } => deliver,
        };
        if deliver && let Event::Mouse(mouse) = event {
            if self.state.is_open() {
                let target = delete().control_at(SCREEN, mouse.column, mouse.row);
                let closed = self.state.click(target, &mut self.layers, &mut self.focus);
                self.answered(closed);
            } else if mouse.row >= 1 {
                self.list_presses += 1;
            }
        }
        self.frame();
    }

    fn press(&mut self, column: u16, row: u16) {
        self.event(&Event::Mouse(MouseEvent::new(
            MouseKind::Down(MouseButton::Left),
            column,
            row,
        )));
    }
}

#[test]
fn confirm_wraps_the_message_and_right_aligns_its_buttons() {
    assert_eq!(
        rows(&delete(), 44, 12),
        [
            "............................................",
            "............................................",
            "....╭Delete branch─────────────────────╮....",
            "....│                                  │....",
            "....│  Delete feature/login? Its 3     │....",
            "....│  unmerged commits will be lost.  │....",
            "....│                                  │....",
            "....│            Cancel      Delete    │....",
            "....│                                  │....",
            "....╰──────────────────────────────────╯....",
            "............................................",
            "............................................",
        ]
    );
    assert_eq!(
        delete().area(Rect::new(0, 0, 44, 12)),
        Rect::new(4, 2, 36, 8)
    );
}

#[test]
fn prompt_shows_its_field_between_the_message_and_buttons() {
    let state = opened(DialogKind::Prompt);
    let prompt = Dialog::prompt("Rename", "New name for the branch:", "login-form", 10)
        .width(30)
        .state(&state);
    assert_eq!(
        rows(&prompt, 30, 9),
        [
            "╭Rename──────────────────────╮",
            "│                            │",
            "│  New name for the branch:  │",
            "│                            │",
            "│  ▏login-form               │",
            "│                            │",
            "│          Cancel      OK    │",
            "│                            │",
            "╰────────────────────────────╯",
        ]
    );
}

#[test]
fn alert_has_one_button() {
    assert_eq!(
        rows(
            &Dialog::alert("Saved", "All 12 files were written.").width(32),
            32,
            7
        ),
        [
            "╭Saved─────────────────────────╮",
            "│                              │",
            "│  All 12 files were written.  │",
            "│                              │",
            "│                        OK    │",
            "│                              │",
            "╰──────────────────────────────╯",
        ]
    );
}

#[test]
fn breaks_words_wider_than_the_message_and_keeps_line_breaks() {
    let dialog = Dialog::alert("Path", "cache/abcdefghijklmnop\nok").width(16);
    let drawn = rows(&dialog, 16, 12);
    assert_eq!(drawn[3], "│  cache/abcd  │");
    assert_eq!(drawn[4], "│  efghijklmn  │");
    assert_eq!(drawn[5], "│  op          │");
    assert_eq!(drawn[6], "│  ok          │");
}

#[test]
fn focused_and_destructive_buttons_take_their_tokens() {
    let (grid, _) = draw(&delete(), 44, 12);
    let cancel = style_at(&grid, 17, 7);
    assert_eq!(cancel.fg, token(Token::TextPrimary));
    assert_eq!(cancel.bg, token(Token::Surface0));
    let confirm = style_at(&grid, 29, 7);
    assert_eq!(confirm.bg, token(Token::Fail));
    assert_eq!(confirm.fg, token(Token::Surface0));
    assert!(confirm.attrs.bold());
    assert_eq!(style_at(&grid, 27, 7).bg, token(Token::Fail));
    assert_eq!(style_at(&grid, 26, 7).bg, token(Token::Surface1));

    let mut state = opened(DialogKind::Confirm);
    state.previous();
    let (grid, _) = draw(&delete().state(&state), 44, 12);
    assert_eq!(style_at(&grid, 17, 7).bg, token(Token::FocusRing));
    let resting = style_at(&grid, 29, 7);
    assert_eq!(resting.fg, token(Token::Fail));
    assert_eq!(resting.bg, token(Token::Surface0));
    assert!(!resting.attrs.bold());

    let plain = Dialog::confirm("Save", "Save changes?").width(30);
    let (grid, clusters) = draw(&plain, 30, 7);
    assert_eq!(
        grid.row_display(4, &clusters),
        "│          Cancel      OK    │"
    );
    assert_eq!(style_at(&grid, 23, 4).bg, token(Token::FocusRing));
}

#[test]
fn prompt_field_draws_its_caret_placeholder_and_scrolls_to_the_caret() {
    let state = opened(DialogKind::Prompt);
    let field = |text: &'static str, caret: usize| {
        Dialog::prompt("Rename", "Name:", text, caret)
            .placeholder("branch name")
            .width(20)
            .state(&state)
    };
    let (grid, clusters) = draw(&field("login", 2), 20, 9);
    assert_eq!(grid.row_display(4, &clusters), "│  ▏login          │");
    assert_eq!(style_at(&grid, 3, 4).fg, token(Token::FocusRing));
    let caret = style_at(&grid, 6, 4);
    assert_eq!(
        (caret.fg, caret.bg),
        (token(Token::Surface0), token(Token::FocusRing))
    );
    assert_eq!(style_at(&grid, 5, 4).bg, token(Token::Surface0));
    assert_eq!(style_at(&grid, 7, 4).bg, token(Token::Surface0));

    let (grid, clusters) = draw(&field("", 0), 20, 9);
    assert_eq!(grid.row_display(4, &clusters), "│  ▏branch name    │");
    assert!(style_at(&grid, 5, 4).attrs.dim());

    let long = "feature/login-form-v2";
    let (grid, clusters) = draw(&field(long, long.len()), 20, 9);
    assert_eq!(grid.row_display(4, &clusters), "│  ▏gin-form-v2    │");
    let end = style_at(&grid, 15, 4);
    assert_eq!(end.bg, token(Token::FocusRing));

    let mut away = opened(DialogKind::Prompt);
    away.next();
    let unfocused = Dialog::prompt("Rename", "Name:", "login", 2)
        .width(20)
        .state(&away);
    let (grid, _) = draw(&unfocused, 20, 9);
    assert_eq!(style_at(&grid, 6, 4).bg, token(Token::Surface0));
    assert_eq!(style_at(&grid, 3, 4).fg, token(Token::Accent));
}

#[test]
fn fades_the_screen_behind_and_stays_inside_a_small_area() {
    let (grid, _) = draw(&delete(), 44, 12);
    assert_ne!(style_at(&grid, 0, 0).fg, Color::Rgb(200, 200, 200));
    assert_eq!(style_at(&grid, 6, 3).bg, token(Token::Surface1));

    let small = rows(&delete(), 20, 5);
    assert_eq!(small[0], "╭Delete branch─────╮");
    assert_eq!(small[4], "╰──────────────────╯");
    assert_eq!(
        delete().area(Rect::new(0, 0, 20, 5)),
        Rect::new(0, 0, 20, 5)
    );
    assert!(
        rows(&delete(), 2, 2)
            .iter()
            .all(|row| row.chars().count() == 2)
    );
}

#[test]
fn control_at_finds_buttons_and_the_field() {
    let screen = Rect::new(0, 0, 44, 12);
    let dialog = delete();
    assert_eq!(dialog.control_at(screen, 15, 7), Some(DialogFocus::Cancel));
    assert_eq!(dialog.control_at(screen, 24, 7), Some(DialogFocus::Cancel));
    assert_eq!(dialog.control_at(screen, 25, 7), None);
    assert_eq!(dialog.control_at(screen, 27, 7), Some(DialogFocus::Confirm));
    assert_eq!(dialog.control_at(screen, 36, 7), Some(DialogFocus::Confirm));
    assert_eq!(dialog.control_at(screen, 37, 7), None);
    assert_eq!(dialog.control_at(screen, 20, 4), None);
    assert_eq!(dialog.control_at(screen, 30, 8), None);
    assert_eq!(dialog.control_at(screen, 30, 6), None);
    assert_eq!(dialog.control_at(screen, 1, 1), None);

    let prompt = Dialog::prompt("Rename", "Name:", "x", 1).width(30);
    let area = Rect::new(0, 0, 30, 9);
    assert_eq!(prompt.control_at(area, 3, 4), Some(DialogFocus::Input));
    assert_eq!(prompt.control_at(area, 26, 4), Some(DialogFocus::Input));
    assert_eq!(prompt.control_at(area, 2, 4), None);
    assert_eq!(prompt.kind(), DialogKind::Prompt);
}

#[test]
fn open_focuses_the_field_or_the_confirm_button() {
    assert_eq!(opened(DialogKind::Prompt).focus(), DialogFocus::Input);
    assert_eq!(opened(DialogKind::Confirm).focus(), DialogFocus::Confirm);
    assert_eq!(opened(DialogKind::Alert).focus(), DialogFocus::Confirm);
    assert_eq!(opened(DialogKind::Prompt).kind(), Some(DialogKind::Prompt));
    let closed = DialogState::new();
    assert!(!closed.is_open());
    assert_eq!(closed.kind(), None);
}

#[test]
fn tab_cycles_inside_the_dialog_in_both_directions() {
    let mut state = opened(DialogKind::Prompt);
    let mut forward = Vec::new();
    for _ in 0..4 {
        state.next();
        forward.push(state.focus());
    }
    assert_eq!(
        forward,
        [
            DialogFocus::Cancel,
            DialogFocus::Confirm,
            DialogFocus::Input,
            DialogFocus::Cancel
        ]
    );
    let mut backward = Vec::new();
    for _ in 0..3 {
        state.previous();
        backward.push(state.focus());
    }
    assert_eq!(
        backward,
        [
            DialogFocus::Input,
            DialogFocus::Confirm,
            DialogFocus::Cancel
        ]
    );

    let mut confirm = opened(DialogKind::Confirm);
    confirm.next();
    assert_eq!(confirm.focus(), DialogFocus::Cancel);
    confirm.next();
    assert_eq!(confirm.focus(), DialogFocus::Confirm);

    let mut alert = opened(DialogKind::Alert);
    alert.previous();
    assert_eq!(alert.focus(), DialogFocus::Confirm);

    let mut closed = DialogState::new();
    closed.next();
    assert_eq!(closed.focus(), DialogFocus::Confirm);
}

#[test]
fn set_focus_only_reaches_controls_the_dialog_has() {
    let mut confirm = opened(DialogKind::Confirm);
    assert!(!confirm.set_focus(DialogFocus::Input));
    assert_eq!(confirm.focus(), DialogFocus::Confirm);
    assert!(confirm.set_focus(DialogFocus::Cancel));
    assert_eq!(confirm.focus(), DialogFocus::Cancel);
    assert!(!DialogState::new().set_focus(DialogFocus::Cancel));
}

#[test]
fn answering_reports_the_outcome_once() -> Result<(), Box<dyn std::error::Error>> {
    let (mut layers, mut focus) = (LayerStack::new(), FocusManager::new());
    let mut state = opened(DialogKind::Confirm);
    let closed = state
        .activate(&mut layers, &mut focus)
        .ok_or("open dialog closes")?;
    assert_eq!(closed.outcome(), DialogOutcome::Confirmed);
    assert!(!state.is_open());
    assert_eq!(state.activate(&mut layers, &mut focus), None);

    let mut cancel = opened(DialogKind::Confirm);
    cancel.next();
    let closed = cancel
        .activate(&mut layers, &mut focus)
        .ok_or("open dialog closes")?;
    assert_eq!(closed.outcome(), DialogOutcome::Cancelled);

    let mut field = opened(DialogKind::Prompt);
    let closed = field
        .activate(&mut layers, &mut focus)
        .ok_or("enter in the field confirms")?;
    assert_eq!(closed.outcome(), DialogOutcome::Confirmed);
    Ok(())
}

#[test]
fn esc_cancels_through_the_layer_stack_and_focus_returns_to_the_opener() {
    let mut screen = Screen::with_open(DialogKind::Confirm);
    assert_eq!(screen.layers.len(), 1);
    assert_ne!(
        screen.focus.focused(),
        Some(LIST),
        "Tab stays inside the dialog"
    );
    screen.event(&Event::Key(KeyEvent::press(KeyCode::Esc)));
    assert!(!screen.state.is_open());
    assert!(screen.layers.is_empty());
    assert_eq!(screen.answers, vec![DialogOutcome::Cancelled]);
    assert_eq!(screen.focus.focused(), Some(LIST));
    screen.event(&Event::Key(KeyEvent::press(KeyCode::Esc)));
    assert_eq!(
        screen.answers.len(),
        1,
        "a second Esc has nothing to cancel"
    );
}

#[test]
fn a_press_outside_leaves_the_dialog_open_and_never_reaches_the_screen() {
    let mut screen = Screen::with_open(DialogKind::Confirm);
    screen.press(1, 11);
    assert!(screen.state.is_open());
    assert_eq!(screen.list_presses, 0);
    assert_eq!(screen.answers, Vec::new());
}

#[test]
fn a_press_on_a_button_answers_and_gives_focus_back() -> Result<(), &'static str> {
    let mut screen = Screen::with_open(DialogKind::Confirm);
    let panel = delete().area(SCREEN);
    let (column, row) = (panel.y..panel.y + panel.height)
        .flat_map(|row| (panel.x..panel.x + panel.width).map(move |column| (column, row)))
        .find(|(column, row)| {
            delete().control_at(SCREEN, *column, *row) == Some(DialogFocus::Confirm)
        })
        .ok_or("the confirm button is drawn")?;
    screen.press(column, row);
    assert_eq!(screen.answers, vec![DialogOutcome::Confirmed]);
    assert!(screen.layers.is_empty());
    assert_eq!(screen.focus.focused(), Some(LIST));
    screen.press(1, 5);
    assert_eq!(screen.list_presses, 1, "the screen takes presses again");
    Ok(())
}

#[test]
fn clicks_answer_with_buttons_focus_the_field_and_ignore_the_outside()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut layers, mut focus) = (LayerStack::new(), FocusManager::new());
    let mut state = opened(DialogKind::Prompt);
    state.next();
    assert_eq!(state.click(None, &mut layers, &mut focus), None);
    assert!(state.is_open());
    assert_eq!(
        state.click(Some(DialogFocus::Input), &mut layers, &mut focus),
        None
    );
    assert_eq!(state.focus(), DialogFocus::Input);
    let closed = state
        .click(Some(DialogFocus::Cancel), &mut layers, &mut focus)
        .ok_or("cancel closes")?;
    assert_eq!(closed.outcome(), DialogOutcome::Cancelled);

    let mut confirm = opened(DialogKind::Confirm);
    confirm.next();
    let closed = confirm
        .click(Some(DialogFocus::Confirm), &mut layers, &mut focus)
        .ok_or("confirm closes")?;
    assert_eq!(closed.outcome(), DialogOutcome::Confirmed);

    let mut alert = opened(DialogKind::Alert);
    assert_eq!(
        alert.click(Some(DialogFocus::Cancel), &mut layers, &mut focus),
        None
    );
    assert!(alert.is_open());
    Ok(())
}

#[test]
fn state_shows_only_focus_the_dialog_has() {
    let confirm = opened(DialogKind::Prompt);
    let (grid, _) = draw(&delete().state(&confirm), 44, 12);
    assert_eq!(style_at(&grid, 29, 7).bg, token(Token::Fail));
    assert_eq!(style_at(&grid, 17, 7).bg, token(Token::Surface0));
}

#[test]
fn wraps_short_words_by_their_place_in_the_line() {
    let dialog = Dialog::alert("T", "a b c d e f g h i j k l").width(14);
    let drawn = rows(&dialog, 14, 9);
    assert_eq!(drawn[2], "│  a b c d   │");
    assert_eq!(drawn[3], "│  e f g h   │");
    assert_eq!(drawn[4], "│  i j k l   │");
}

#[test]
fn panel_height_follows_the_wrapped_message() {
    let dialog = Dialog::alert("T", "aaaa bbbb ccccc").width(20);
    assert_eq!(dialog.area(Rect::new(0, 0, 20, 10)), Rect::new(0, 1, 20, 8));
    let drawn = rows(&dialog, 20, 10);
    assert_eq!(drawn[3], "│  aaaa bbbb       │");
    assert_eq!(drawn[4], "│  ccccc           │");
}

#[test]
fn rows_that_reach_the_bottom_border_are_not_drawn() {
    let border = format!("╰{}╯", "─".repeat(28));
    let confirm = Dialog::confirm("Save", "Save changes?").width(30);
    let area = Rect::new(0, 0, 30, 5);
    assert_eq!(rows(&confirm, 30, 5)[4], border);
    assert_eq!(confirm.control_at(area, 23, 4), None);
    let prompt = Dialog::prompt("Save", "Save as:", "notes.md", 8).width(30);
    assert_eq!(rows(&prompt, 30, 5)[4], border);
    assert_eq!(prompt.control_at(area, 10, 4), None);
}

#[test]
fn clips_a_button_label_inside_its_padding() {
    let dialog = Dialog::alert("T", "Done.")
        .width(14)
        .confirm_label("Acknowledge");
    assert_eq!(rows(&dialog, 14, 7)[4], "│    Ackn    │");
}

#[test]
fn caret_inside_a_character_moves_to_its_start() {
    let state = opened(DialogKind::Prompt);
    let prompt = Dialog::prompt("Rename", "Name:", "née", 2)
        .width(20)
        .state(&state);
    let (grid, _) = draw(&prompt, 20, 9);
    assert_eq!(style_at(&grid, 5, 4).bg, token(Token::FocusRing));
    assert_eq!(style_at(&grid, 6, 4).bg, token(Token::Surface0));
}
