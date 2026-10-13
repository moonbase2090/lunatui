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
    Event, Focus, KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseKind, MouseUnit,
    Scroll,
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
            if CrosstermEvent::Key(key).is_key_release() {
                return None;
            }
            let code = map_key_code(key.code)?;
            let modifiers = map_modifiers(key.modifiers);
            Some(Event::Key(KeyEvent::press(code).modifiers(modifiers)))
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
