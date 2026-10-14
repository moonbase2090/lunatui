// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, MouseButton as CrosstermMouseButton};

use crate::input::{
    Event, Focus, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
    MouseUnit, Scroll,
};

/// Joins the crossterm input worker on drop after setting [`shutdown`].
pub struct InputWorker {
    shutdown: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Drop for InputWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Maps a crossterm event into a lunatui [`Event`].
#[must_use]
pub fn map_event(raw: CrosstermEvent) -> Option<Event> {
    match raw {
        CrosstermEvent::Key(key) => {
            // Console hosts report a release for every key. The VT parser only
            // emits releases for kitty event types, so a console release is dropped.
            if key.kind == event::KeyEventKind::Release {
                return None;
            }
            let code = map_key_code(key.code)?;
            let mut modifiers = map_modifiers(key.modifiers);
            // Conhost reports Shift+Tab as BackTab with no shift bit. VT CSI Z is Tab+shift.
            if matches!(key.code, event::KeyCode::BackTab) {
                modifiers = modifiers.shift();
            }
            let kind = if key.kind == event::KeyEventKind::Repeat {
                KeyKind::Repeat
            } else {
                KeyKind::Press
            };
            Some(Event::Key(
                KeyEvent::press(code).modifiers(modifiers).kind(kind),
            ))
        }
        CrosstermEvent::Mouse(mouse) => Some(Event::Mouse(map_mouse(mouse))),
        CrosstermEvent::Paste(text) => Some(Event::Paste(crate::input::Paste::Chunk(text))),
        CrosstermEvent::FocusGained => Some(Event::Focus(Focus::In)),
        CrosstermEvent::FocusLost => Some(Event::Focus(Focus::Out)),
        CrosstermEvent::Resize(_, _) => None,
    }
}

fn map_modifiers(raw: event::KeyModifiers) -> Modifiers {
    let mut mods = Modifiers::new();
    if raw.contains(event::KeyModifiers::SHIFT) {
        mods = mods.shift();
    }
    if raw.contains(event::KeyModifiers::ALT) {
        mods = mods.alt();
    }
    if raw.contains(event::KeyModifiers::CONTROL) {
        mods = mods.control();
    }
    if raw.contains(event::KeyModifiers::SUPER) {
        mods = mods.super_key();
    }
    if raw.contains(event::KeyModifiers::HYPER) {
        mods = mods.hyper();
    }
    if raw.contains(event::KeyModifiers::META) {
        mods = mods.meta();
    }
    mods
}

fn map_key_code(raw: event::KeyCode) -> Option<KeyCode> {
    match raw {
        event::KeyCode::Char(ch) => Some(KeyCode::Char(ch)),
        event::KeyCode::Enter => Some(KeyCode::Enter),
        event::KeyCode::Left => Some(KeyCode::Left),
        event::KeyCode::Right => Some(KeyCode::Right),
        event::KeyCode::Up => Some(KeyCode::Up),
        event::KeyCode::Down => Some(KeyCode::Down),
        event::KeyCode::Home => Some(KeyCode::Home),
        event::KeyCode::End => Some(KeyCode::End),
        event::KeyCode::PageUp => Some(KeyCode::PageUp),
        event::KeyCode::PageDown => Some(KeyCode::PageDown),
        event::KeyCode::Tab => Some(KeyCode::Tab),
        event::KeyCode::Backspace => Some(KeyCode::Backspace),
        event::KeyCode::Delete => Some(KeyCode::Delete),
        event::KeyCode::Insert => Some(KeyCode::Insert),
        event::KeyCode::Esc => Some(KeyCode::Esc),
        event::KeyCode::F(n) => Some(KeyCode::F(n)),
        event::KeyCode::Null => None,
        event::KeyCode::BackTab => Some(KeyCode::Tab),
        event::KeyCode::Media(_)
        | event::KeyCode::Modifier(_)
        | event::KeyCode::CapsLock
        | event::KeyCode::ScrollLock
        | event::KeyCode::NumLock
        | event::KeyCode::PrintScreen
        | event::KeyCode::Pause => None,
        _ => None,
    }
}

fn map_mouse(raw: event::MouseEvent) -> MouseEvent {
    let kind = match raw.kind {
        event::MouseEventKind::Down(btn) => MouseKind::Down(map_mouse_button(btn)),
        event::MouseEventKind::Up(btn) => MouseKind::Up(map_mouse_button(btn)),
        event::MouseEventKind::Drag(btn) => MouseKind::Drag(map_mouse_button(btn)),
        event::MouseEventKind::Moved => MouseKind::Move,
        event::MouseEventKind::ScrollUp => MouseKind::Scroll(Scroll::Up),
        event::MouseEventKind::ScrollDown => MouseKind::Scroll(Scroll::Down),
        event::MouseEventKind::ScrollLeft => MouseKind::Scroll(Scroll::Left),
        event::MouseEventKind::ScrollRight => MouseKind::Scroll(Scroll::Right),
    };
    MouseEvent {
        kind,
        column: raw.column,
        row: raw.row,
        modifiers: map_modifiers(raw.modifiers),
        unit: MouseUnit::Cells,
    }
}

fn map_mouse_button(raw: CrosstermMouseButton) -> MouseButton {
    match raw {
        CrosstermMouseButton::Left => MouseButton::Left,
        CrosstermMouseButton::Right => MouseButton::Right,
        CrosstermMouseButton::Middle => MouseButton::Middle,
    }
}

/// Starts a worker that polls crossterm console events (D3).
///
/// # Errors
///
/// Returns an error when the worker thread cannot be spawned.
pub fn spawn_input_reader(sender: Sender<io::Result<Vec<Event>>>) -> io::Result<InputWorker> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&shutdown);
    let join = thread::Builder::new()
        .name(String::from("lunatui-windows-input"))
        .spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match event::poll(Duration::from_millis(50)) {
                    Ok(true) => match event::read() {
                        Ok(raw) => {
                            let Some(mapped) = map_event(raw) else {
                                continue;
                            };
                            if sender.send(Ok(vec![mapped])).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = sender.send(Err(error));
                            return;
                        }
                    },
                    Ok(false) => {}
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        return;
                    }
                }
            }
        })
        .map_err(io::Error::other)?;
    Ok(InputWorker {
        shutdown,
        join: Some(join),
    })
}

/// Non-blocking poll for already-queued crossterm events.
///
/// # Errors
///
/// Returns an I/O error from the console driver.
pub fn try_read_events() -> io::Result<Vec<Event>> {
    let mut out = Vec::new();
    while event::poll(Duration::from_millis(0))? {
        let raw = event::read()?;
        if let Some(mapped) = map_event(raw) {
            out.push(mapped);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crossterm::event::{
        Event as CrosstermEvent, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
        KeyEventKind, KeyModifiers, MouseButton as CrosstermMouseButton,
        MouseEvent as CrosstermMouseEvent, MouseEventKind,
    };

    use crate::input::{
        Event, Focus, KeyCode, KeyEvent, KeyKind, Modifiers, MouseButton, MouseEvent, MouseKind,
        MouseUnit, Parser, Paste, Scroll,
    };

    use super::map_event;

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
    fn console_backtab_matches_vt_shift_tab() {
        let mapped = map_event(key(
            CrosstermKeyCode::BackTab,
            KeyModifiers::SHIFT,
            KeyEventKind::Press,
        ))
        .expect("backtab");
        assert_eq!(parse(b"\x1b[Z"), vec![mapped]);
    }

    #[test]
    fn console_backtab_without_shift_bit_still_matches_vt() {
        let mapped = map_event(key(
            CrosstermKeyCode::BackTab,
            KeyModifiers::empty(),
            KeyEventKind::Press,
        ))
        .expect("backtab");
        assert_eq!(
            mapped,
            Event::Key(KeyEvent::press(KeyCode::Tab).modifiers(Modifiers::new().shift()))
        );
    }

    #[test]
    fn console_repeat_matches_vt_repeat() {
        let mapped = map_event(key(
            CrosstermKeyCode::Char('a'),
            KeyModifiers::empty(),
            KeyEventKind::Repeat,
        ))
        .expect("repeat");
        assert_eq!(parse(b"\x1b[97;1:2u"), vec![mapped]);
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
    fn console_lock_and_media_keys_are_dropped() {
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
        let mapped = map_event(key(
            CrosstermKeyCode::Char('c'),
            KeyModifiers::SHIFT
                | KeyModifiers::ALT
                | KeyModifiers::CONTROL
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
            KeyEventKind::Press,
        ))
        .expect("modified key");
        assert_eq!(
            mapped,
            Event::Key(
                KeyEvent::press(KeyCode::Char('c')).modifiers(
                    Modifiers::new()
                        .shift()
                        .alt()
                        .control()
                        .super_key()
                        .hyper()
                        .meta()
                )
            )
        );
    }

    #[test]
    fn console_mouse_buttons_map_to_cell_events() {
        let down = map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
            kind: MouseEventKind::Down(CrosstermMouseButton::Left),
            column: 3,
            row: 4,
            modifiers: KeyModifiers::SHIFT,
        }))
        .expect("down");
        assert_eq!(
            down,
            Event::Mouse(MouseEvent {
                kind: MouseKind::Down(MouseButton::Left),
                column: 3,
                row: 4,
                modifiers: Modifiers::new().shift(),
                unit: MouseUnit::Cells,
            })
        );
        let up = map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
            kind: MouseEventKind::Up(CrosstermMouseButton::Right),
            column: 1,
            row: 2,
            modifiers: KeyModifiers::empty(),
        }))
        .expect("up");
        assert_eq!(
            up,
            Event::Mouse(MouseEvent {
                kind: MouseKind::Up(MouseButton::Right),
                column: 1,
                row: 2,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })
        );
        let drag = map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
            kind: MouseEventKind::Drag(CrosstermMouseButton::Middle),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        }))
        .expect("drag");
        assert_eq!(
            drag,
            Event::Mouse(MouseEvent {
                kind: MouseKind::Drag(MouseButton::Middle),
                column: 0,
                row: 0,
                modifiers: Modifiers::new(),
                unit: MouseUnit::Cells,
            })
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
            let mapped = map_event(CrosstermEvent::Mouse(CrosstermMouseEvent {
                kind: raw_kind,
                column: 8,
                row: 1,
                modifiers: KeyModifiers::empty(),
            }))
            .expect("mouse");
            assert_eq!(
                mapped,
                Event::Mouse(MouseEvent {
                    kind,
                    column: 8,
                    row: 1,
                    modifiers: Modifiers::new(),
                    unit: MouseUnit::Cells,
                })
            );
        }
    }

    #[test]
    fn console_paste_is_a_chunk() {
        let mapped = map_event(CrosstermEvent::Paste(String::from("hello"))).expect("paste");
        assert_eq!(mapped, Event::Paste(Paste::Chunk(String::from("hello"))));
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
