// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(windows)]
mod smoke {
    use crossterm::event::{
        Event as CrosstermEvent, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    };
    use lunatui_core::{ClusterTable, Color, Grid, Rect, Style};
    use lunatui_term::{ColorMode, Encoder, Event, KeyCode, KeyEvent, sys::windows::map_event};

    #[test]
    fn map_event_translates_press() {
        let raw = CrosstermEvent::Key(CrosstermKeyEvent::new(
            CrosstermKeyCode::Char('q'),
            crossterm::event::KeyModifiers::empty(),
        ));
        let mapped = map_event(raw).expect("key event");
        assert_eq!(mapped, Event::Key(KeyEvent::press(KeyCode::Char('q'))));
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
        assert!(payload.contains(b"38;2;1;2;3"));
    }
}

#[cfg(not(windows))]
#[test]
fn windows_terminal_tests_run_on_windows_only() {}
