// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use lunatui_term::{Event, Parser};

pub(crate) const ESCAPE_TIMEOUT: Duration = Duration::from_millis(50);

struct Shared {
    state: Mutex<Held>,
    changed: Condvar,
}

struct Held {
    parser: Parser,
    generation: u64,
    closed: bool,
}

fn lock(shared: &Shared) -> MutexGuard<'_, Held> {
    shared
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn watch_escape(shared: &Shared, sender: &Sender<io::Result<Vec<Event>>>) {
    let mut held = lock(shared);
    loop {
        while !held.closed && !held.parser.holds_escape() {
            held = shared
                .changed
                .wait(held)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        if held.closed {
            return;
        }
        let seen = held.generation;
        let (next, timeout) = shared
            .changed
            .wait_timeout_while(held, ESCAPE_TIMEOUT, |held| {
                !held.closed && held.generation == seen
            })
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held = next;
        if held.closed {
            return;
        }
        if timeout.timed_out() && held.generation == seen {
            let events = held.parser.expire();
            held.generation += 1;
            if !events.is_empty() && sender.send(Ok(events)).is_err() {
                held.closed = true;
                return;
            }
        }
    }
}

/// Reads terminal input on a worker thread and groups available events into frames.
pub struct Input {
    receiver: Receiver<io::Result<Vec<Event>>>,
    pending_error: Option<io::Error>,
    finished: bool,
    #[cfg(windows)]
    worker: Option<lunatui_term::sys::windows::InputWorker>,
}

impl Input {
    /// Opens platform input (Unix stdin bytes or Windows crossterm events).
    ///
    /// # Errors
    ///
    /// Returns an error if the worker thread cannot be started.
    pub fn open() -> io::Result<Self> {
        #[cfg(unix)]
        {
            Self::from_reader(io::stdin())
        }
        #[cfg(windows)]
        {
            let (sender, receiver) = mpsc::channel();
            let worker = lunatui_term::sys::windows::spawn_input_reader(sender)?;
            Ok(Self {
                receiver,
                pending_error: None,
                finished: false,
                worker: Some(worker),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "lunatui input is only supported on Unix and Windows",
            ))
        }
    }

    #[cfg(feature = "image-presenter")]
    pub(crate) fn open_prefixed(bytes: Vec<u8>) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Self::from_reader(io::Cursor::new(bytes).chain(io::stdin()))
        }
        #[cfg(not(unix))]
        {
            drop(bytes);
            Self::open()
        }
    }

    /// Starts a worker that reads terminal bytes from `reader`.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker thread cannot be started.
    pub fn from_reader<R>(mut reader: R) -> io::Result<Self>
    where
        R: Read + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        let shared = Arc::new(Shared {
            state: Mutex::new(Held {
                parser: Parser::new(),
                generation: 0,
                closed: false,
            }),
            changed: Condvar::new(),
        });
        let watcher_shared = Arc::clone(&shared);
        let watcher_sender = sender.clone();
        thread::Builder::new()
            .name(String::from("lunatui-input-escape"))
            .spawn(move || watch_escape(&watcher_shared, &watcher_sender))?;
        thread::Builder::new()
            .name(String::from("lunatui-input"))
            .spawn(move || {
                let mut buffer = [0; 4096];
                loop {
                    let read = reader.read(&mut buffer);
                    let mut held = lock(&shared);
                    match read {
                        Ok(0) => {
                            let events = held.parser.flush();
                            if !events.is_empty() {
                                let _ = sender.send(Ok(events));
                            }
                            held.closed = true;
                            shared.changed.notify_all();
                            return;
                        }
                        Ok(length) => {
                            let events = held.parser.push(&buffer[..length]);
                            held.generation += 1;
                            if !events.is_empty() && sender.send(Ok(events)).is_err() {
                                held.closed = true;
                                shared.changed.notify_all();
                                return;
                            }
                            if held.parser.holds_escape() {
                                shared.changed.notify_all();
                            }
                        }
                        Err(error) => {
                            let _ = sender.send(Err(error));
                            held.closed = true;
                            shared.changed.notify_all();
                            return;
                        }
                    }
                }
            })?;
        Ok(Self {
            receiver,
            pending_error: None,
            finished: false,
            #[cfg(windows)]
            worker: None,
        })
    }

    /// Waits for available input and returns it as one frame.
    ///
    /// Events produced by the same reader burst are returned together. Events already queued
    /// when a frame is read are coalesced into that frame as well.
    ///
    /// # Errors
    ///
    /// Returns an input read error after any earlier events have been delivered.
    pub fn next_frame(&mut self) -> io::Result<Option<Vec<Event>>> {
        if let Some(error) = self.pending_error.take() {
            self.finished = true;
            return Err(error);
        }
        if self.finished {
            return Ok(None);
        }
        let mut events = match self.receiver.recv() {
            Ok(Ok(events)) => events,
            Ok(Err(error)) => {
                self.finished = true;
                return Err(error);
            }
            Err(_) => {
                self.finished = true;
                return Ok(None);
            }
        };
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(mut batch)) => events.append(&mut batch),
                Ok(Err(error)) => {
                    self.pending_error = Some(error);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        Ok(Some(events))
    }

    /// Returns coalesced input when a burst is already queued; otherwise `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns an input read error after any earlier events have been delivered.
    pub fn try_next_frame(&mut self) -> io::Result<Option<Vec<Event>>> {
        if let Some(error) = self.pending_error.take() {
            self.finished = true;
            return Err(error);
        }
        if self.finished {
            return Ok(None);
        }
        let mut events = match self.receiver.try_recv() {
            Ok(Ok(events)) => events,
            Ok(Err(error)) => {
                self.finished = true;
                return Err(error);
            }
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.finished = true;
                return Ok(None);
            }
        };
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(mut batch)) => events.append(&mut batch),
                Ok(Err(error)) => {
                    self.pending_error = Some(error);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        Ok(Some(events))
    }
}

#[cfg(windows)]
impl Drop for Input {
    fn drop(&mut self) {
        drop(self.worker.take());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{self, Read};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::thread;
    use std::time::{Duration, Instant};

    use lunatui_term::{Event, KeyCode, KeyEvent, Modifiers, Paste};

    use super::ESCAPE_TIMEOUT;
    use crate::Input;

    struct FakeSource {
        reads: VecDeque<Vec<u8>>,
        ready: Sender<()>,
        release: Receiver<()>,
    }

    impl FakeSource {
        fn from_reads(
            reads: Vec<Vec<u8>>,
            barrier_sender: Sender<()>,
            continue_receiver: Receiver<()>,
        ) -> Self {
            Self {
                reads: reads.into(),
                ready: barrier_sender,
                release: continue_receiver,
            }
        }
    }

    impl Read for FakeSource {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if let Some(bytes) = self.reads.pop_front() {
                buffer[..bytes.len()].copy_from_slice(&bytes);
                return Ok(bytes.len());
            }
            self.ready
                .send(())
                .map_err(|_| io::Error::other("test receiver closed"))?;
            self.release
                .recv()
                .map_err(|_| io::Error::other("test release closed"))?;
            Ok(0)
        }
    }

    #[test]
    fn bursts_coalesce_to_one_frame() -> Result<(), Box<dyn std::error::Error>> {
        let (barrier_sender, barrier_receiver) = mpsc::channel();
        let (continue_sender, continue_receiver) = mpsc::channel();
        let source = FakeSource::from_reads(
            vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
            barrier_sender,
            continue_receiver,
        );
        let mut input = Input::from_reader(source)?;
        barrier_receiver.recv()?;

        let frame = input.next_frame()?.ok_or("source ended before its burst")?;
        continue_sender.send(())?;

        assert_eq!(
            frame,
            vec![
                Event::Key(KeyEvent::press(KeyCode::Char('a'))),
                Event::Key(KeyEvent::press(KeyCode::Char('b'))),
                Event::Key(KeyEvent::press(KeyCode::Char('c'))),
            ]
        );
        assert_eq!(input.next_frame()?, None);
        Ok(())
    }

    struct ChannelSource {
        bytes: Receiver<Vec<u8>>,
    }

    impl Read for ChannelSource {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            match self.bytes.recv() {
                Ok(bytes) => {
                    buffer[..bytes.len()].copy_from_slice(&bytes);
                    Ok(bytes.len())
                }
                Err(_) => Ok(0),
            }
        }
    }

    type Checked = Result<(), Box<dyn std::error::Error>>;

    fn live_input() -> Result<(Input, Sender<Vec<u8>>), io::Error> {
        let (sender, bytes) = mpsc::channel();
        let input = Input::from_reader(ChannelSource { bytes })?;
        Ok((input, sender))
    }

    fn frame_within(
        input: Input,
        limit: Duration,
    ) -> Result<(Input, Option<Vec<Event>>, Duration), String> {
        let (done, result) = mpsc::channel();
        let started = Instant::now();
        thread::spawn(move || {
            let mut input = input;
            let frame = input.next_frame().map_err(|error| error.to_string());
            let _ = done.send((input, frame));
        });
        match result.recv_timeout(limit) {
            Ok((input, frame)) => Ok((input, frame?, started.elapsed())),
            Err(error) => Err(format!("no frame within {limit:?}: {error}")),
        }
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::press(code))
    }

    const LIMIT: Duration = Duration::from_secs(3);

    #[test]
    fn a_lone_escape_is_delivered_without_any_further_input() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b".to_vec())?;
        let (mut input, frame, waited) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Esc)]));
        assert!(waited < Duration::from_secs(1), "{waited:?}");
        assert_eq!(input.try_next_frame()?, None);
        Ok(())
    }

    #[test]
    fn the_escape_waits_out_the_timeout_instead_of_firing_at_once() -> Checked {
        let (input, sender) = live_input()?;
        let sent = Instant::now();
        sender.send(b"\x1b".to_vec())?;
        let (_input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Esc)]));
        assert!(sent.elapsed() >= ESCAPE_TIMEOUT, "{:?}", sent.elapsed());
        Ok(())
    }

    #[test]
    fn whole_csi_and_ss3_sequences_parse_and_add_no_stray_escape() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b[A".to_vec())?;
        let (input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Up)]));
        sender.send(b"\x1bOP".to_vec())?;
        let (input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::F(1))]));
        sender.send(b"\x1b[1;5C".to_vec())?;
        let (mut input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(
            frame,
            Some(vec![Event::Key(
                KeyEvent::press(KeyCode::Right).modifiers(Modifiers::new().control())
            )])
        );
        thread::sleep(ESCAPE_TIMEOUT * 4);
        assert_eq!(input.try_next_frame()?, None);
        Ok(())
    }

    #[test]
    fn a_sequence_split_inside_the_timeout_is_one_key_not_escape_plus_text() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b".to_vec())?;
        thread::sleep(Duration::from_millis(10));
        sender.send(b"[".to_vec())?;
        thread::sleep(Duration::from_millis(10));
        sender.send(b"A".to_vec())?;
        let (mut input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Up)]));
        thread::sleep(ESCAPE_TIMEOUT * 4);
        assert_eq!(input.try_next_frame()?, None);
        Ok(())
    }

    #[test]
    fn every_arriving_chunk_restarts_the_escape_timeout() -> Checked {
        let (input, sender) = live_input()?;
        let gap = ESCAPE_TIMEOUT * 7 / 10;
        sender.send(b"\x1b".to_vec())?;
        thread::sleep(gap);
        sender.send(b"[".to_vec())?;
        thread::sleep(gap);
        sender.send(b"A".to_vec())?;
        let (mut input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Up)]));
        thread::sleep(ESCAPE_TIMEOUT * 4);
        assert_eq!(input.try_next_frame()?, None);
        Ok(())
    }

    #[test]
    fn a_key_after_an_expired_escape_is_a_plain_key_in_order() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b".to_vec())?;
        let (input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Esc)]));
        sender.send(b"x".to_vec())?;
        let (_input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Char('x'))]));
        Ok(())
    }

    #[test]
    fn an_escape_followed_at_once_by_a_key_is_still_alt() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1bx".to_vec())?;
        let (mut input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(
            frame,
            Some(vec![Event::Key(
                KeyEvent::press(KeyCode::Char('x')).modifiers(Modifiers::new().alt())
            )])
        );
        thread::sleep(ESCAPE_TIMEOUT * 4);
        assert_eq!(input.try_next_frame()?, None);
        Ok(())
    }

    #[test]
    fn two_escapes_far_apart_are_two_events() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b".to_vec())?;
        let (input, first, _) = frame_within(input, LIMIT)?;
        sender.send(b"\x1b".to_vec())?;
        let (_input, second, _) = frame_within(input, LIMIT)?;
        assert_eq!(first, Some(vec![key(KeyCode::Esc)]));
        assert_eq!(second, Some(vec![key(KeyCode::Esc)]));
        Ok(())
    }

    #[test]
    fn a_paste_that_outlasts_the_timeout_is_not_cut_short() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b[200~\x1b".to_vec())?;
        let (input, first, _) = frame_within(input, LIMIT)?;
        thread::sleep(ESCAPE_TIMEOUT * 4);
        sender.send(b"[201~".to_vec())?;
        let (_input, rest, _) = frame_within(input, LIMIT)?;
        let all: Vec<Event> = first.into_iter().chain(rest).flatten().collect();
        let starts = all
            .iter()
            .filter(|e| matches!(e, Event::Paste(Paste::Start)))
            .count();
        let ends = all
            .iter()
            .filter(|e| matches!(e, Event::Paste(Paste::End)))
            .count();
        assert_eq!((starts, ends), (1, 1), "{all:?}");
        assert!(!all.contains(&key(KeyCode::Esc)), "{all:?}");
        Ok(())
    }

    #[test]
    fn end_of_input_still_flushes_a_held_escape() -> Checked {
        let (input, sender) = live_input()?;
        sender.send(b"\x1b".to_vec())?;
        drop(sender);
        let (mut input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Esc)]));
        assert_eq!(input.next_frame()?, None);
        Ok(())
    }
}
