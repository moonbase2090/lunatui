// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

use lunatui_core::{CellWidth, ClusterTable, Color, Grid, Rect, Style, UnderlineStyle};
use lunatui_term::{Event, KeyCode, KeyEvent, KeyKind, Modifiers, Paste, PtySize};
use lunatui_theme::{Theme, Token, Variant};
use lunatui_widgets::{PseudoTerminal, RenderCx, TerminalScreen, Widget, WidgetState};

type Test = Result<(), Box<dyn std::error::Error>>;

fn screen(cols: u16, rows: u16, bytes: &[u8]) -> Result<TerminalScreen, std::io::Error> {
    let mut screen = TerminalScreen::new(PtySize::new(cols, rows)?);
    screen.feed(bytes);
    Ok(screen)
}

fn draw(screen: &TerminalScreen, state: WidgetState, area: Rect) -> (Grid, ClusterTable) {
    let mut grid = Grid::new(20, 8);
    let mut clusters = ClusterTable::new();
    let theme = Theme::builtin(Variant::Dark);
    PseudoTerminal::new(screen)
        .state(state)
        .render(area, &mut RenderCx::new(&mut grid, &mut clusters, &theme));
    (grid, clusters)
}

#[test]
fn plain_text_cursor_moves_erasure_and_wrapping_are_emulated() -> Test {
    let mut screen = screen(8, 3, b"hello\r\nworld")?;
    assert_eq!(screen.row_text(0), "hello");
    assert_eq!(screen.cursor(), (5, 1));
    screen.feed(b"\x1b[1;2H!\x1b[K");
    assert_eq!(screen.row_text(0), "h!");
    screen.feed(b"\x1b[3;1H123456789");
    assert_eq!(screen.row_text(1), "12345678");
    assert_eq!(screen.row_text(2), "9");
    Ok(())
}

#[test]
fn fragmented_utf8_sgr_and_combining_marks_keep_their_cells() -> Test {
    let bytes = "\x1b[1;3;4;38;2;12;34;56;48;5;123mCafe\u{301}界";
    let mut screen = screen(12, 2, b"")?;
    for byte in bytes.as_bytes() {
        screen.feed(std::slice::from_ref(byte));
    }
    assert_eq!(screen.row_text(0), "Cafe\u{301}界");
    let cell = screen.cell(3, 0).ok_or("missing accented cell")?;
    assert_eq!(cell.text, "e\u{301}");
    assert_eq!(cell.style.fg, Color::Rgb(12, 34, 56));
    assert_eq!(cell.style.bg, Color::Indexed(123));
    assert!(cell.style.attrs.bold() && cell.style.attrs.italic());
    assert_eq!(cell.style.attrs.underline(), UnderlineStyle::Single);
    assert_eq!(
        screen.cell(4, 0).ok_or("missing wide cell")?.width,
        CellWidth::Two
    );
    assert_eq!(
        screen.cell(5, 0).ok_or("missing tail")?.width,
        CellWidth::Continuation
    );
    Ok(())
}

#[test]
fn alternate_screen_restores_main_content_and_cursor() -> Test {
    let mut screen = screen(10, 3, b"main")?;
    screen.feed(b"\x1b[?1049hother");
    assert!(screen.is_alternate());
    assert_eq!(screen.row_text(0), "other");
    screen.feed(b"\x1b[?1049l");
    assert!(!screen.is_alternate());
    assert_eq!(screen.row_text(0), "main");
    assert_eq!(screen.cursor(), (4, 0));
    Ok(())
}

#[test]
fn scrolling_and_resizing_have_bounded_geometry() -> Test {
    let mut screen = screen(10, 2, b"one\r\ntwo\r\nthree")?;
    screen.set_scrollback(1);
    assert_eq!(screen.row_text(0), "one");
    assert_eq!(screen.row_text(1), "two");
    screen.set_scrollback(0);
    assert_eq!(screen.row_text(0), "two");
    screen.resize(PtySize::new(5, 3)?);
    assert_eq!(screen.size(), PtySize::new(5, 3)?);
    assert_eq!(screen.row_text(0), "two");
    assert_eq!(screen.row_text(1), "three");
    assert_eq!(screen.cell(5, 0), None);
    Ok(())
}

#[test]
fn osc_titles_are_local_and_oversized_control_strings_are_discarded() -> Test {
    let mut screen = screen(12, 2, b"\x1b]2;build log\x07ok")?;
    assert_eq!(screen.title(), "build log");
    screen.feed(b"\x1b]2;");
    for _ in 0..100 {
        screen.feed(&[b'x'; 1024]);
    }
    screen.feed(b"\x1b\\!");
    assert_eq!(screen.title(), "build log");
    assert_eq!(screen.row_text(0), "ok!");
    screen.feed(b"\x1bPignored\x1b\\still");
    assert_eq!(screen.row_text(0), "ok!still");
    Ok(())
}

#[test]
fn device_queries_produce_bounded_child_replies() -> Test {
    let mut screen = screen(10, 3, b"hi\x1b[6n\x1b[5n\x1b[c")?;
    assert_eq!(screen.take_replies(), b"\x1b[1;3R\x1b[0n\x1b[?1;2c");
    assert_eq!(screen.take_replies(), b"");
    for _ in 0..1000 {
        screen.feed(b"\x1b[6n");
    }
    assert!(screen.take_replies().len() <= 4096);
    Ok(())
}

#[test]
fn focus_gates_input_and_disabled_output_is_dim() -> Test {
    let screen = screen(10, 2, b"hello\x1b[?25l")?;
    let event = Event::Key(KeyEvent::press(KeyCode::Char('q')));
    assert_eq!(PseudoTerminal::new(&screen).input(&event), None);
    assert_eq!(
        PseudoTerminal::new(&screen)
            .state(WidgetState::Disabled)
            .input(&event),
        None
    );
    assert_eq!(
        PseudoTerminal::new(&screen)
            .state(WidgetState::Focused)
            .input(&event),
        Some(b"q".to_vec())
    );
    let (grid, _) = draw(&screen, WidgetState::Disabled, Rect::new(2, 2, 10, 2));
    assert!(grid.get(2, 2).ok_or("missing cell")?.style().attrs.dim());
    Ok(())
}

#[test]
fn input_uses_child_cursor_modes_control_bytes_and_paste_modes() -> Test {
    let mut screen = screen(10, 2, b"")?;
    let key = |code| Event::Key(KeyEvent::press(code));
    let input = |screen: &TerminalScreen, event: &Event| {
        PseudoTerminal::new(screen)
            .state(WidgetState::Focused)
            .input(event)
    };
    assert_eq!(input(&screen, &key(KeyCode::Up)), Some(b"\x1b[A".to_vec()));
    let ctrl_c =
        Event::Key(KeyEvent::press(KeyCode::Char('c')).modifiers(Modifiers::new().control()));
    assert_eq!(input(&screen, &ctrl_c), Some(vec![3]));
    let alt = Event::Key(KeyEvent::press(KeyCode::Char('x')).modifiers(Modifiers::new().alt()));
    assert_eq!(input(&screen, &alt), Some(b"\x1bx".to_vec()));
    assert_eq!(input(&screen, &key(KeyCode::Enter)), Some(b"\r".to_vec()));
    assert_eq!(input(&screen, &Event::Paste(Paste::Start)), None);
    assert_eq!(
        input(&screen, &Event::Paste(Paste::Chunk("one\ntwo".into()))),
        Some(b"one\ntwo".to_vec())
    );
    screen.feed(b"\x1b[?1h\x1b[?2004h");
    assert_eq!(input(&screen, &key(KeyCode::Up)), Some(b"\x1bOA".to_vec()));
    assert_eq!(
        input(&screen, &Event::Paste(Paste::Start)),
        Some(b"\x1b[200~".to_vec())
    );
    assert_eq!(
        input(&screen, &Event::Paste(Paste::End)),
        Some(b"\x1b[201~".to_vec())
    );
    let release = Event::Key(KeyEvent::press(KeyCode::Char('x')).kind(KeyKind::Release));
    assert_eq!(input(&screen, &release), None);
    Ok(())
}

#[test]
fn drawing_copies_text_and_styles_and_rewrites_every_cell_in_its_area() -> Test {
    let screen = screen(10, 2, b"\x1b[1;31mred\x1b[0m")?;
    let (grid, clusters) = draw(&screen, WidgetState::Normal, Rect::new(2, 2, 10, 2));
    assert_eq!(grid.row_display(2, &clusters), "  red               ");
    let cell = grid.get(2, 2).ok_or("missing red cell")?;
    assert_eq!(cell.style().fg, Color::Indexed(1));
    assert!(cell.style().attrs.bold());
    let theme = Theme::builtin(Variant::Dark);
    assert_eq!(
        grid.get(8, 2).ok_or("missing blank")?.style().fg,
        theme.resolve(Token::TextPrimary)
    );
    assert_eq!(
        grid.get(8, 2).ok_or("missing blank")?.style().bg,
        theme.resolve(Token::Surface0)
    );
    assert_eq!(grid.get(1, 2).ok_or("outside cell")?.style(), Style::new());
    Ok(())
}

#[test]
fn wide_characters_are_blank_when_cut_and_never_overwrite_a_neighbour() -> Test {
    let screen = screen(4, 1, "界!".as_bytes())?;
    let (grid, clusters) = draw(&screen, WidgetState::Normal, Rect::new(2, 2, 1, 1));
    assert_eq!(grid.row_display(2, &clusters), "                    ");
    assert_eq!(grid.get(3, 2).ok_or("outside")?.style(), Style::new());
    Ok(())
}

#[test]
fn cursor_is_local_visible_only_when_focused_and_hidden_by_child_mode() -> Test {
    let mut screen = screen(8, 2, b"hi")?;
    let (grid, _) = draw(&screen, WidgetState::Focused, Rect::new(2, 2, 8, 2));
    assert_eq!(
        grid.get(4, 2).ok_or("cursor")?.style().bg,
        Theme::builtin(Variant::Dark).resolve(Token::FocusRing)
    );
    screen.feed(b"\x1b[?25l");
    let (grid, _) = draw(&screen, WidgetState::Focused, Rect::new(2, 2, 8, 2));
    assert_ne!(
        grid.get(4, 2).ok_or("hidden cursor")?.style().bg,
        Theme::builtin(Variant::Dark).resolve(Token::FocusRing)
    );
    Ok(())
}

#[test]
fn malformed_and_fragmented_bytes_do_not_change_the_view_bounds() -> Test {
    let mut screen = screen(8, 3, b"")?;
    let mut value = 7_u32;
    for _ in 0..2000 {
        value = value.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        screen.feed(&value.to_le_bytes());
        let (x, y) = screen.cursor();
        assert!(x < 8 && y < 3);
    }
    let (grid, _) = draw(&screen, WidgetState::Normal, Rect::new(2, 2, 8, 3));
    assert_eq!(grid.get(10, 2).ok_or("neighbour")?.style(), Style::new());
    Ok(())
}

#[test]
fn exact_control_string_limit_and_cancelled_strings_recover_without_host_effects() -> Test {
    let mut screen = screen(12, 2, b"")?;
    let accepted = format!("\x1b]2;{}\x07", "x".repeat(4091));
    assert_eq!(accepted.len(), 4096);
    screen.feed(accepted.as_bytes());
    assert_eq!(screen.title(), "x".repeat(256));
    screen.feed(format!("\x1b]2;{}\x07", "y".repeat(4092)).as_bytes());
    assert_eq!(screen.title(), "x".repeat(256));
    screen.feed(b"\x1b]52;c;ignored\x18safe\x1b]2;local\x1b\\!");
    assert_eq!(screen.title(), "local");
    assert_eq!(screen.row_text(0), "safe!");
    Ok(())
}

#[test]
fn navigation_function_and_modified_keys_use_literal_xterm_sequences() -> Test {
    let screen = screen(10, 2, b"")?;
    let view = PseudoTerminal::new(&screen).state(WidgetState::Focused);
    for (code, modifiers, expected) in [
        (KeyCode::F(1), Modifiers::new(), "\x1bOP"),
        (KeyCode::F(4), Modifiers::new(), "\x1bOS"),
        (KeyCode::F(5), Modifiers::new(), "\x1b[15~"),
        (KeyCode::F(12), Modifiers::new(), "\x1b[24~"),
        (KeyCode::Delete, Modifiers::new().control(), "\x1b[3;5~"),
        (KeyCode::Left, Modifiers::new().alt(), "\x1b[1;3D"),
        (KeyCode::Home, Modifiers::new(), "\x1b[H"),
        (KeyCode::Tab, Modifiers::new().shift(), "\x1b[Z"),
        (KeyCode::Backspace, Modifiers::new(), "\x7f"),
        (KeyCode::Char('['), Modifiers::new().control(), "\x1b"),
        (KeyCode::Char(']'), Modifiers::new().control(), "\x1d"),
        (KeyCode::Char('?'), Modifiers::new().control(), "\x7f"),
        (KeyCode::Char(' '), Modifiers::new().control(), "\0"),
    ] {
        let event = Event::Key(KeyEvent::press(code).modifiers(modifiers));
        assert_eq!(
            view.input(&event),
            Some(expected.as_bytes().to_vec()),
            "{code:?}"
        );
    }
    let event = Event::Key(KeyEvent::press(KeyCode::Char('x')).text("界"));
    assert_eq!(view.input(&event), Some("界".as_bytes().to_vec()));
    let event =
        Event::Key(KeyEvent::press(KeyCode::Char('界')).modifiers(Modifiers::new().control()));
    assert_eq!(view.input(&event), None);
    Ok(())
}

#[test]
fn history_and_child_resize_requests_cannot_expand_the_validated_screen() -> Test {
    let mut screen = screen(8, 2, b"")?;
    for _ in 0..1000 {
        screen.feed(b"line\r\n");
    }
    screen.set_scrollback(usize::MAX);
    assert_eq!(screen.scrollback(), 256);
    screen.feed(b"\x1b[8;65535;65535t");
    assert_eq!(screen.size(), PtySize::new(8, 2)?);
    Ok(())
}

#[cfg(feature = "color-profiles")]
#[test]
fn a_prepared_profile_downsamples_child_sgr_colors_without_losing_attributes() -> Test {
    let screen = screen(8, 1, b"\x1b[1;38;2;255;0;0;48;2;0;0;255mR")?;
    let theme = Theme::builtin(Variant::Dark);
    let palette = theme.prepare(lunatui_theme::Profile::Ansi16);
    let mut grid = Grid::new(8, 1);
    let mut clusters = ClusterTable::new();
    PseudoTerminal::new(&screen).render(
        Rect::new(0, 0, 8, 1),
        &mut RenderCx::new(&mut grid, &mut clusters, &theme).palette(&palette),
    );
    let style = grid.get(0, 0).ok_or("missing profiled cell")?.style();
    assert_eq!(style.fg, Color::Indexed(9));
    assert_eq!(style.bg, Color::Indexed(4));
    assert!(style.attrs.bold());
    Ok(())
}

#[test]
fn rendered_cells_keep_child_italic_underline_and_inverse_attributes() -> Test {
    let screen = screen(8, 2, b"\x1b[3;4;7mX")?;
    let (grid, _) = draw(&screen, WidgetState::Normal, Rect::new(2, 2, 8, 2));
    let style = grid.get(2, 2).ok_or("missing styled cell")?.style();
    assert!(style.attrs.italic());
    assert_eq!(style.attrs.underline(), UnderlineStyle::Single);
    assert!(style.attrs.reverse());
    Ok(())
}
