// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(windows)]
mod smoke {
    use crossterm::event::{
        Event as CrosstermEvent, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
        KeyEventKind, KeyModifiers, MouseButton as CrosstermMouseButton,
        MouseEvent as CrosstermMouseEvent, MouseEventKind,
    };
    use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
    use lunatui_term::{
        ColorMode, Encoder, Event, Focus, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent,
        MouseKind, MouseUnit, Parser, Paste, Scroll, sys::windows::map_event,
    };

    fn parse(bytes: &[u8]) -> Vec<Event> {
        let mut parser = Parser::new();
        let mut events = parser.push(bytes);
        events.extend(parser.flush());
        events
    }

    fn key(code: CrosstermKeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> CrosstermEvent {
        CrosstermEvent::Key(CrosstermKeyEvent::new_with_kind(code, modifiers, kind))
    }

    #[test]
    fn map_event_translates_press() {
        let raw = CrosstermEvent::Key(CrosstermKeyEvent::new(
            CrosstermKeyCode::Char('q'),
            crossterm::event::KeyModifiers::empty(),
        ));
        assert_eq!(
            map_event(raw),
            Some(Event::Key(KeyEvent::press(KeyCode::Char('q'))))
        );
    }

    #[test]
    fn encoder_emits_vt_for_windows_terminal() {
        let prev = Grid::new(3, 2);
        let mut next = Grid::new(3, 2);
        let mut clusters = ClusterTable::new();
        let area = Rect::new(0, 0, 3, 2);
        next.fill(
            area,
            Style::new().fg(Color::Rgb(1, 2, 3)),
            'x',
            &mut clusters,
        );
        let mut encoder = Encoder::new();
        let payload = encoder.encode_diff(&prev, &next, &clusters, ColorMode::TrueColor);
        let rgb_sgr: &[u8] = b"38;2;1;2;3";
        assert!(
            payload
                .windows(rgb_sgr.len())
                .any(|window| window == rgb_sgr)
        );
    }

    #[test]
    fn console_backtab_matches_vt_shift_tab() {
        let mapped = map_event(key(
            CrosstermKeyCode::BackTab,
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        ));
        assert_eq!(parse(b"\x1b[Z"), mapped.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn console_backtab_without_shift_bit_still_matches_vt() {
        assert_eq!(
            map_event(key(
                CrosstermKeyCode::BackTab,
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
            Some(Event::Key(
                KeyEvent::press(KeyCode::Tab).modifiers(Modifiers::new().shift())
            ))
        );
    }

    #[test]
    fn console_repeat_matches_vt_repeat() {
        let mapped = map_event(key(
            CrosstermKeyCode::Char('a'),
            KeyModifiers::empty(),
            KeyEventKind::Repeat,
        ));
        assert_eq!(
            parse(b"\x1b[97;1:2u"),
            mapped.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn console_release_is_dropped() {
        assert_eq!(
            map_event(key(
                CrosstermKeyCode::Char('a'),
                KeyModifiers::empty(),
                KeyEventKind::Release,
            )),
            None
        );
    }

    #[test]
    fn console_resize_is_dropped() {
        assert_eq!(map_event(CrosstermEvent::Resize(80, 24)), None);
    }

    #[test]
    fn console_lock_and_null_keys_are_dropped() {
        assert_eq!(
            map_event(key(
                CrosstermKeyCode::CapsLock,
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
            None
        );
        assert_eq!(
            map_event(key(
                CrosstermKeyCode::Null,
                KeyModifiers::empty(),
                KeyEventKind::Press,
            )),
            None
        );
    }

    #[test]
    fn console_modifiers_match_named_bits() {
        assert_eq!(
            map_event(key(
                CrosstermKeyCode::Char('c'),
                KeyModifiers::SHIFT
                    | KeyModifiers::ALT
                    | KeyModifiers::CONTROL
                    | KeyModifiers::SUPER
                    | KeyModifiers::HYPER
                    | KeyModifiers::META,
                KeyEventKind::Press,
            )),
            Some(Event::Key(
                KeyEvent::press(KeyCode::Char('c')).modifiers(
                    Modifiers::new()
                        .shift()
                        .alt()
                        .control()
                        .super_key()
                        .hyper()
                        .meta()
                )
            ))
        );
    }

    #[test]
    fn console_mouse_buttons_map_to_cell_events() {
        assert_eq!(
            map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
                kind: MouseEventKind::Down(CrosstermMouseButton::Left),
                column: 3,
                row: 4,
                modifiers: KeyModifiers::SHIFT,
            })),
            Some(Event::Mouse(MouseEvent {
                kind: MouseKind::Down(MouseButton::Left),
                column: 3,
                row: 4,
                modifiers: Modifiers::new().shift(),
                unit: MouseUnit::Cells,
            }))
        );
        assert_eq!(
            map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
                kind: MouseEventKind::Up(CrosstermMouseButton::Right),
                column: 1,
                row: 2,
                modifiers: KeyModifiers::empty(),
            })),
            Some(Event::Mouse(MouseEvent {
                kind: MouseKind::Up(MouseButton::Right),
                column: 1,
                row: 2,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            }))
        );
        assert_eq!(
            map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
                kind: MouseEventKind::Drag(CrosstermMouseButton::Middle),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::empty(),
            })),
            Some(Event::Mouse(MouseEvent {
                kind: MouseKind::Drag(MouseButton::Middle),
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            }))
        );
    }

    #[test]
    fn console_mouse_scroll_and_move_map() {
        let cases = [
            (MouseEventKind::Moved, MouseKind::Move),
            (MouseEventKind::ScrollUp, MouseKind::Scroll(Scroll::Up)),
            (MouseEventKind::ScrollDown, MouseKind::Scroll(Scroll::Down)),
            (MouseEventKind::ScrollLeft, MouseKind::Scroll(Scroll::Left)),
            (
                MouseEventKind::ScrollRight,
                MouseKind::Scroll(Scroll::Right),
            ),
        ];
        for (raw_kind, kind) in cases {
            assert_eq!(
                map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
                    kind: raw_kind,
                    column: 8,
                    row: 1,
                    modifiers: KeyModifiers::empty(),
                })),
                Some(Event::Mouse(MouseEvent {
                    kind,
                    column: 8,
                    row: 1,
                    modifiers: Modifiers::new(),
                    unit: MouseUnit::Cells,
                }))
            );
        }
    }

    #[test]
    fn console_paste_is_a_chunk() {
        assert_eq!(
            map_event(CrosstermEvent::Paste(String::from("hello"))),
            Some(Event::Paste(Paste::Chunk(String::from("hello"))))
        );
    }

    #[test]
    fn console_focus_maps_in_and_out() {
        assert_eq!(
            map_event(CrosstermEvent::FocusGained),
            Some(Event::Focus(Focus::In))
        );
        assert_eq!(
            map_event(CrosstermEvent::FocusLost),
            Some(Event::Focus(Focus::Out))
        );
    }
}

#[cfg(not(windows))]
#[test]
fn windows_terminal_tests_run_on_windows_only() {}
