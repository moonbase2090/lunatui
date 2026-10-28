//! Behavioral tests for TextArea editing and input handling.
#![cfg(feature = "text-area")]

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers, Paste};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{
    EditResult, KeymapMode, RenderCx, TextArea, TextPosition, VimMode, Widget, WidgetState,
};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::press(code))
}

fn shifted(code: KeyCode) -> Event {
    Event::Key(KeyEvent::press(code).modifiers(Modifiers::new().shift()))
}

fn typed(c: char) -> Event {
    Event::Key(KeyEvent::press(KeyCode::Char(c)).text(c.to_string()))
}

fn undo_key() -> Event {
    Event::Key(KeyEvent::press(KeyCode::Char('z')).modifiers(Modifiers::new().control()))
}

#[test]
fn inserts_multiline_text_and_undoes_as_one_edit() {
    let mut editor = TextArea::new("ab");
    editor.move_to(
        TextPosition {
            line: 0,
            grapheme: 1,
        },
        false,
    );
    assert_eq!(
        editor.handle_event(&key(KeyCode::Enter)),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "a\nb");
    assert_eq!(
        editor.handle_event(&undo_key()),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "ab");
    assert_eq!(editor.redo(), EditResult::DocumentChanged);
    assert_eq!(editor.text(), "a\nb");
    editor.move_to(
        TextPosition {
            line: 1,
            grapheme: 0,
        },
        false,
    );
    assert_eq!(
        editor.handle_event(&key(KeyCode::Backspace)),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "ab");
    assert_eq!(editor.cursor().line, 0);
}

#[test]
fn directed_selection_replaces_content_and_survives_recomposition() {
    let mut editor = TextArea::new("a\u{301}b");
    editor.move_to(
        TextPosition {
            line: 0,
            grapheme: 1,
        },
        false,
    );
    assert_eq!(
        editor.handle_event(&typed('x')),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "a\u{301}xb");
    assert_eq!(editor.cursor().grapheme, 2);
    assert_eq!(
        editor.handle_event(&key(KeyCode::Backspace)),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "a\u{301}b");
    assert_eq!(
        editor.handle_event(&undo_key()),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "a\u{301}xb");
}

#[test]
fn shift_movement_keeps_anchor_and_replaces_selected_range() {
    let mut editor = TextArea::new("abcd");
    editor.handle_event(&shifted(KeyCode::Right));
    editor.handle_event(&shifted(KeyCode::Right));
    assert_eq!(editor.selection().unwrap().anchor.grapheme, 0);
    assert_eq!(editor.selection().unwrap().head.grapheme, 2);
    editor.handle_event(&typed('x'));
    assert_eq!(editor.text(), "xcd");
    editor.handle_event(&undo_key());
    assert_eq!(editor.text(), "abcd");
    assert_eq!(editor.selection().unwrap().anchor.grapheme, 0);
    assert_eq!(editor.selection().unwrap().head.grapheme, 2);
    editor.redo();
    assert_eq!(editor.text(), "xcd");
}

#[test]
fn vertical_motion_preserves_display_column_across_short_lines() {
    let mut editor = TextArea::new("abcd\nx\nabcd");
    editor.move_to(
        TextPosition {
            line: 0,
            grapheme: 3,
        },
        false,
    );
    editor.handle_event(&key(KeyCode::Down));
    assert_eq!(
        editor.cursor(),
        TextPosition {
            line: 1,
            grapheme: 1
        }
    );
    editor.handle_event(&key(KeyCode::Down));
    assert_eq!(
        editor.cursor(),
        TextPosition {
            line: 2,
            grapheme: 3
        }
    );
}

#[test]
fn key_repeats_are_processed_by_the_widget() {
    let mut editor = TextArea::new("");
    editor.handle_event(&Event::Key(
        KeyEvent::press(KeyCode::Char('x'))
            .text("x")
            .kind(KeyKind::Press),
    ));
    editor.handle_event(&Event::Key(
        KeyEvent::press(KeyCode::Char('x'))
            .text("x")
            .kind(KeyKind::Repeat),
    ));
    assert_eq!(editor.text(), "xx");
}

#[test]
fn printable_key_without_associated_text_still_inserts() {
    let mut editor = TextArea::new("");
    assert_eq!(
        editor.handle_event(&key(KeyCode::Char('x'))),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "x");
}

#[test]
fn bracketed_paste_is_one_undoable_edit_and_normalizes_newlines() {
    let mut editor = TextArea::new("start");
    editor.handle_event(&Event::Paste(Paste::Start));
    editor.handle_event(&Event::Paste(Paste::Chunk("a\r".into())));
    editor.handle_event(&Event::Paste(Paste::Chunk("\nb".into())));
    assert_eq!(editor.text(), "start");
    assert_eq!(
        editor.handle_event(&Event::Paste(Paste::End)),
        EditResult::DocumentChanged
    );
    assert_eq!(editor.text(), "a\nbstart");
    editor.handle_event(&undo_key());
    assert_eq!(editor.text(), "start");
}

#[test]
fn bracketed_paste_enters_search_without_editing_the_document() {
    let mut editor = TextArea::new("alpha beta");
    editor.start_search();
    editor.handle_event(&Event::Paste(Paste::Start));
    editor.handle_event(&Event::Paste(Paste::Chunk("be".into())));
    editor.handle_event(&Event::Paste(Paste::Chunk("ta".into())));
    editor.handle_event(&Event::Paste(Paste::End));
    assert_eq!(editor.search_query(), Some("beta"));
    assert_eq!(
        editor.cursor(),
        TextPosition {
            line: 0,
            grapheme: 6
        }
    );
    assert_eq!(editor.text(), "alpha beta");
}

#[test]
fn rejected_bracketed_paste_discards_the_rest_of_its_chunks() {
    let mut editor = TextArea::new("unchanged");
    editor.handle_event(&Event::Paste(Paste::Start));
    let too_large = "x".repeat(256 * 1024 + 1);
    assert_eq!(
        editor.handle_event(&Event::Paste(Paste::Chunk(too_large))),
        EditResult::PasteRejected
    );
    assert_eq!(
        editor.handle_event(&Event::Paste(Paste::Chunk("tail".into()))),
        EditResult::Ignored
    );
    assert_eq!(
        editor.handle_event(&Event::Paste(Paste::End)),
        EditResult::Ignored
    );
    assert_eq!(editor.text(), "unchanged");
}

#[test]
fn empty_yank_does_not_delete_a_selection() {
    let mut editor = TextArea::new("ab");
    editor.handle_event(&shifted(KeyCode::Right));
    let paste =
        Event::Key(KeyEvent::press(KeyCode::Char('v')).modifiers(Modifiers::new().control()));
    assert_eq!(editor.handle_event(&paste), EditResult::Ignored);
    assert_eq!(editor.text(), "ab");
}

#[test]
fn disabled_editor_ignores_input_and_vim_exposes_mode() {
    let mut editor = TextArea::new("").keymap_mode(KeymapMode::Vim);
    assert_eq!(editor.vim_mode(), VimMode::Normal);
    editor.handle_event(&typed('i'));
    assert_eq!(editor.vim_mode(), VimMode::Insert);
    editor.set_widget_state(WidgetState::Disabled);
    assert_eq!(editor.handle_event(&typed('x')), EditResult::Ignored);
    assert_eq!(editor.text(), "");
}

#[test]
fn vim_insert_mode_routes_text_and_escape_through_its_registry() {
    let mut editor = TextArea::new("").keymap_mode(KeymapMode::Vim);
    assert_eq!(
        editor.handle_event(&Event::Key(
            KeyEvent::press(KeyCode::Char('H')).modifiers(Modifiers::new().shift()),
        )),
        EditResult::Ignored
    );
    editor.handle_event(&key(KeyCode::Char('i')));
    assert_eq!(editor.vim_mode(), VimMode::Insert);
    editor.handle_event(&key(KeyCode::Char('i')));
    editor.handle_event(&key(KeyCode::Char('a')));
    editor.handle_event(&key(KeyCode::Char('/')));
    assert_eq!(editor.text(), "ia/");
    editor.handle_event(&key(KeyCode::Esc));
    assert_eq!(editor.vim_mode(), VimMode::Normal);
}

#[test]
fn emacs_search_finds_text_and_escape_restores_cursor() {
    let mut editor = TextArea::new("alpha beta alpha");
    editor.handle_event(&Event::Key(
        KeyEvent::press(KeyCode::Char('s')).modifiers(Modifiers::new().control()),
    ));
    editor.handle_event(&typed('b'));
    assert_eq!(editor.cursor().grapheme, 6);
    editor.handle_event(&key(KeyCode::Esc));
    assert_eq!(editor.cursor().grapheme, 0);
}

#[test]
fn search_is_safe_from_a_multibyte_cursor_boundary() {
    let mut editor = TextArea::new("éclair");
    editor.handle_event(&Event::Key(
        KeyEvent::press(KeyCode::Char('s')).modifiers(Modifiers::new().control()),
    ));
    editor.handle_event(&typed('c'));
    assert_eq!(
        editor.cursor(),
        TextPosition {
            line: 0,
            grapheme: 1
        }
    );
    editor.set_text("a\u{301}b");
    assert_eq!(editor.cursor().grapheme, 1);
}

#[test]
fn keymap_registry_lists_commands_for_the_active_mode() {
    let mut emacs = TextArea::new("");
    assert!(
        emacs
            .keymap_registry()
            .commands()
            .iter()
            .any(|command| command.id() == "search")
    );
    let search =
        Event::Key(KeyEvent::press(KeyCode::Char('s')).modifiers(Modifiers::new().control()));
    assert_eq!(emacs.keymap_registry().dispatch(&search), Some("search"));
    assert_eq!(emacs.execute_command("unknown"), EditResult::Ignored);
    let repeated = Event::Key(
        KeyEvent::press(KeyCode::Char('s'))
            .modifiers(Modifiers::new().control())
            .kind(KeyKind::Repeat),
    );
    assert_eq!(emacs.keymap_registry().dispatch(&repeated), None);
    let vim = TextArea::new("").keymap_mode(KeymapMode::Vim);
    assert!(
        vim.keymap_registry()
            .commands()
            .iter()
            .any(|command| command.id() == "insert-mode")
    );
}

#[test]
fn focused_and_disabled_states_use_theme_tokens() {
    let theme = Theme::builtin(Variant::Dark);
    let mut grid = Grid::new(2, 1);
    let mut clusters = ClusterTable::new();
    let mut focused = TextArea::new("x");
    focused.set_widget_state(WidgetState::Focused);
    focused.render(
        Rect::new(0, 0, 2, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    assert_eq!(
        grid.get(0, 0).unwrap().style().bg,
        theme.resolve(Token::FocusFill)
    );

    grid.fill(Rect::new(0, 0, 2, 1), Style::new(), ' ', &mut clusters);
    let mut disabled = TextArea::new("x");
    disabled.move_to(
        TextPosition {
            line: 0,
            grapheme: 1,
        },
        false,
    );
    disabled.set_widget_state(WidgetState::Disabled);
    disabled.render(
        Rect::new(0, 0, 2, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    assert_eq!(
        grid.get(0, 0).unwrap().style().fg,
        theme.resolve(Token::TextMuted)
    );
}

#[test]
fn render_follows_cursor_and_keeps_a_cursor_before_an_unfittable_wide_glyph() {
    let theme = Theme::builtin(Variant::Dark);
    let mut editor = TextArea::new("a\nb\nc");
    editor.move_to(
        TextPosition {
            line: 2,
            grapheme: 0,
        },
        false,
    );
    let mut grid = Grid::new(1, 1);
    let mut clusters = ClusterTable::new();
    editor.render(
        Rect::new(0, 0, 1, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme),
    );
    assert_eq!(
        grid.get(0, 0).unwrap().style().bg,
        theme.resolve(Token::SurfaceRaised)
    );

    let wide = TextArea::new("界");
    let mut narrow_grid = Grid::new(1, 1);
    wide.render(
        Rect::new(0, 0, 1, 1),
        &mut RenderCx::new(&mut narrow_grid, &mut clusters, &theme),
    );
    assert_eq!(
        narrow_grid.get(0, 0).unwrap().style().bg,
        theme.resolve(Token::SurfaceRaised)
    );
}
