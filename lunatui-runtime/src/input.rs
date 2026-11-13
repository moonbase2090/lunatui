// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use lunatui_term::{Event, Parser, watch_size};
#[cfg(unix)]
use lunatui_term::{SIZE_POLL, terminal_size};

type SizeReader = Box<dyn FnMut() -> Option<(u16, u16)> + Send>;

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

fn close(shared: &Shared) {
    lock(shared).closed = true;
    shared.changed.notify_all();
}

trait Timer: Send + Sync {
    fn wait<'a>(
        &self,
        shared: &'a Shared,
        held: MutexGuard<'a, Held>,
        seen: u64,
    ) -> (MutexGuard<'a, Held>, bool);
}

struct Clock(Duration);

impl Timer for Clock {
    fn wait<'a>(
        &self,
        shared: &'a Shared,
        held: MutexGuard<'a, Held>,
        seen: u64,
    ) -> (MutexGuard<'a, Held>, bool) {
        let (next, result) = shared
            .changed
            .wait_timeout_while(held, self.0, |held| !held.closed && held.generation == seen)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (next, result.timed_out())
    }
}

fn watch_escape(shared: &Shared, sender: &Sender<io::Result<Vec<Event>>>, timer: &dyn Timer) {
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
        let (next, timed_out) = timer.wait(shared, held, seen);
        held = next;
        if held.closed {
            return;
        }
        if timed_out && held.generation == seen {
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
            Self::from_reader_sized(io::stdin(), Box::new(terminal_size), SIZE_POLL)
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
            Self::from_reader_sized(
                io::Cursor::new(bytes).chain(io::stdin()),
                Box::new(terminal_size),
                SIZE_POLL,
            )
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
    pub fn from_reader<R>(reader: R) -> io::Result<Self>
    where
        R: Read + Send + 'static,
    {
        Self::from_reader_with(reader, ESCAPE_TIMEOUT)
    }

    fn from_reader_with<R>(reader: R, escape_timeout: Duration) -> io::Result<Self>
    where
        R: Read + Send + 'static,
    {
        Self::from_reader_timed(reader, Arc::new(Clock(escape_timeout)), None)
    }

    /// Starts a worker that reads terminal bytes from `reader` and also delivers an [`Event::Resize`] first and whenever `size` reports a new value, checked every `poll`.
    ///
    /// # Errors
    ///
    /// Returns an error if a worker thread cannot be started.
    pub fn from_reader_sized<R>(reader: R, size: SizeReader, poll: Duration) -> io::Result<Self>
    where
        R: Read + Send + 'static,
    {
        Self::from_reader_timed(reader, Arc::new(Clock(ESCAPE_TIMEOUT)), Some((size, poll)))
    }

    fn from_reader_timed<R>(
        mut reader: R,
        timer: Arc<dyn Timer>,
        size: Option<(SizeReader, Duration)>,
    ) -> io::Result<Self>
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
            .spawn(move || {
                watch_escape(&watcher_shared, &watcher_sender, timer.as_ref());
            })?;
        if let Some((read, poll)) = size {
            let size_sender = sender.clone();
            let size_shared = Arc::clone(&shared);
            watch_size(
                read,
                poll,
                move |columns, rows| {
                    size_sender
                        .send(Ok(vec![Event::Resize(columns, rows)]))
                        .is_ok()
                },
                move || lock(&size_shared).closed,
            )?;
        }
        let failed_shared = Arc::clone(&shared);
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
            })
            .inspect_err(|_| close(&failed_shared))?;
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

    use super::{
        Clock, Condvar, ESCAPE_TIMEOUT, Held, Mutex, MutexGuard, Shared, Timer, close, lock,
        watch_escape,
    };
    use crate::Input;
    use lunatui_term::Parser;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

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
        live_input_with(ESCAPE_TIMEOUT)
    }

    fn live_input_with(timeout: Duration) -> Result<(Input, Sender<Vec<u8>>), io::Error> {
        let (sender, bytes) = mpsc::channel();
        let input = Input::from_reader_with(ChannelSource { bytes }, timeout)?;
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
        let timeout = Duration::from_millis(200);
        let (input, sender) = live_input_with(timeout)?;
        let sent = Instant::now();
        sender.send(b"\x1b".to_vec())?;
        let (_input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Esc)]));
        assert!(sent.elapsed() >= timeout, "{:?}", sent.elapsed());
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
        let (input, sender) = live_input_with(Duration::from_secs(30))?;
        sender.send(b"\x1b".to_vec())?;
        thread::sleep(Duration::from_millis(10));
        sender.send(b"[".to_vec())?;
        thread::sleep(Duration::from_millis(10));
        sender.send(b"A".to_vec())?;
        let (mut input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(frame, Some(vec![key(KeyCode::Up)]));
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

    #[test]
    fn closing_the_shared_state_stops_a_watcher_that_is_waiting_for_an_escape() {
        let shared = Arc::new(Shared {
            state: Mutex::new(Held {
                parser: Parser::new(),
                generation: 0,
                closed: false,
            }),
            changed: Condvar::new(),
        });
        let (sender, _receiver) = mpsc::channel();
        let watcher = {
            let shared = Arc::clone(&shared);
            thread::spawn(move || watch_escape(&shared, &sender, &Clock(Duration::from_secs(60))))
        };
        {
            let mut held = lock(&shared);
            let _ = held.parser.push(b"\x1b");
            held.generation += 1;
        }
        shared.changed.notify_all();
        thread::sleep(Duration::from_millis(50));
        assert!(!watcher.is_finished());
        close(&shared);
        let deadline = Instant::now() + LIMIT;
        while !watcher.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(watcher.is_finished());
    }

    #[test]
    fn closing_wakes_a_watcher_that_has_no_escape_to_wait_for() {
        let shared = Arc::new(Shared {
            state: Mutex::new(Held {
                parser: Parser::new(),
                generation: 0,
                closed: false,
            }),
            changed: Condvar::new(),
        });
        let (sender, _receiver) = mpsc::channel();
        let watcher = {
            let shared = Arc::clone(&shared);
            thread::spawn(move || watch_escape(&shared, &sender, &Clock(ESCAPE_TIMEOUT)))
        };
        thread::sleep(Duration::from_millis(50));
        assert!(!watcher.is_finished());
        close(&shared);
        let deadline = Instant::now() + LIMIT;
        while !watcher.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(watcher.is_finished());
    }

    struct Manual {
        fire: Arc<AtomicBool>,
        armed: Mutex<Sender<u64>>,
    }

    impl Timer for Manual {
        fn wait<'a>(
            &self,
            shared: &'a Shared,
            held: MutexGuard<'a, Held>,
            seen: u64,
        ) -> (MutexGuard<'a, Held>, bool) {
            if let Ok(armed) = self.armed.lock() {
                let _ = armed.send(seen);
            }
            let mut held = held;
            while !held.closed && held.generation == seen && !self.fire.load(Ordering::SeqCst) {
                held = shared
                    .changed
                    .wait_timeout(held, Duration::from_millis(5))
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .0;
            }
            let next = held;
            (next, self.fire.swap(false, Ordering::SeqCst))
        }
    }

    struct Controlled {
        shared: Arc<Shared>,
        fire: Arc<AtomicBool>,
        armed: Receiver<u64>,
        events: Receiver<io::Result<Vec<Event>>>,
        watcher: thread::JoinHandle<()>,
    }

    fn controlled() -> Controlled {
        let shared = Arc::new(Shared {
            state: Mutex::new(Held {
                parser: Parser::new(),
                generation: 0,
                closed: false,
            }),
            changed: Condvar::new(),
        });
        let (sender, events) = mpsc::channel();
        let (armed_sender, armed) = mpsc::channel();
        let fire = Arc::new(AtomicBool::new(false));
        let timer = Manual {
            fire: Arc::clone(&fire),
            armed: Mutex::new(armed_sender),
        };
        let watcher = {
            let shared = Arc::clone(&shared);
            thread::spawn(move || watch_escape(&shared, &sender, &timer))
        };
        Controlled {
            shared,
            fire,
            armed,
            events,
            watcher,
        }
    }

    impl Controlled {
        fn push(&self, bytes: &[u8]) {
            let mut held = lock(&self.shared);
            let _ = held.parser.push(bytes);
            held.generation += 1;
            drop(held);
            self.shared.changed.notify_all();
        }

        fn arm(&self) -> Result<u64, String> {
            self.armed
                .recv_timeout(LIMIT)
                .map_err(|error| format!("the watcher did not arm its timer: {error}"))
        }

        fn next_events(&self) -> Result<Vec<Event>, String> {
            self.events
                .recv_timeout(LIMIT)
                .map_err(|error| format!("no events: {error}"))?
                .map_err(|error| error.to_string())
        }

        fn fire(&self) {
            self.fire.store(true, Ordering::SeqCst);
            self.shared.changed.notify_all();
        }

        fn stop(self) -> Result<(), String> {
            close(&self.shared);
            self.watcher
                .join()
                .map_err(|_| "the watcher panicked".to_string())
        }
    }

    #[test]
    fn a_chunk_that_arrives_before_the_timer_fires_restarts_the_wait() -> Result<(), String> {
        let run = controlled();
        run.push(b"\x1b");
        let first = run.arm()?;
        run.push(b"[");
        let second = run.arm()?;
        assert!(second > first);
        assert!(run.events.try_recv().is_err());
        run.push(b"A");
        assert!(run.events.try_recv().is_err());
        run.stop()
    }

    #[test]
    fn the_timer_firing_expires_the_escape_that_was_waited_for() -> Result<(), String> {
        let run = controlled();
        run.push(b"\x1b");
        run.arm()?;
        run.push(b"[");
        run.arm()?;
        run.fire();
        assert_eq!(
            run.next_events()?,
            vec![key(KeyCode::Esc), key(KeyCode::Char('['))]
        );
        run.stop()
    }

    #[test]
    fn a_timer_that_fires_after_new_bytes_arrived_does_not_expire_them() -> Result<(), String> {
        let run = controlled();
        run.push(b"\x1b");
        run.arm()?;
        {
            let mut held = lock(&run.shared);
            let _ = held.parser.push(b"[");
            held.generation += 1;
            run.fire.store(true, Ordering::SeqCst);
        }
        run.shared.changed.notify_all();
        run.arm()?;
        assert!(run.events.try_recv().is_err());
        run.fire();
        assert_eq!(
            run.next_events()?,
            vec![key(KeyCode::Esc), key(KeyCode::Char('['))]
        );
        run.stop()
    }

    #[test]
    fn a_complete_sequence_leaves_the_timer_idle() -> Result<(), String> {
        let run = controlled();
        run.push(b"\x1b[A");
        assert!(run.armed.recv_timeout(Duration::from_millis(100)).is_err());
        run.fire();
        assert!(run.events.recv_timeout(Duration::from_millis(100)).is_err());
        run.stop()
    }

    #[test]
    fn the_reader_restarts_the_timer_for_every_chunk_it_consumes() -> Result<(), String> {
        let (bytes_sender, bytes) = mpsc::channel();
        let (armed_sender, armed) = mpsc::channel();
        let fire = Arc::new(AtomicBool::new(false));
        let timer = Manual {
            fire: Arc::clone(&fire),
            armed: Mutex::new(armed_sender),
        };
        let input = Input::from_reader_timed(ChannelSource { bytes }, Arc::new(timer), None)
            .map_err(|error| error.to_string())?;
        let wait = |armed: &Receiver<u64>| {
            armed
                .recv_timeout(LIMIT)
                .map_err(|error| format!("the timer was not restarted: {error}"))
        };
        bytes_sender
            .send(b"\x1b".to_vec())
            .map_err(|error| error.to_string())?;
        let first = wait(&armed)?;
        bytes_sender
            .send(b"[".to_vec())
            .map_err(|error| error.to_string())?;
        let second = wait(&armed)?;
        assert!(second > first);
        fire.store(true, Ordering::SeqCst);
        let (_input, frame, _) = frame_within(input, LIMIT)?;
        assert_eq!(
            frame,
            Some(vec![key(KeyCode::Esc), key(KeyCode::Char('['))])
        );
        Ok(())
    }

    struct Silent {
        release: Receiver<()>,
    }

    impl Read for Silent {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            let _ = self.release.recv();
            Ok(0)
        }
    }

    #[test]
    fn a_size_change_wakes_a_blocked_reader() -> Result<(), Box<dyn std::error::Error>> {
        let (release, held) = mpsc::channel();
        let size = Arc::new(Mutex::new((80, 24)));
        let reader = Arc::clone(&size);
        let mut input = Input::from_reader_sized(
            Silent { release: held },
            Box::new(move || reader.lock().ok().map(|size| *size)),
            Duration::from_millis(1),
        )?;
        assert_eq!(input.next_frame()?, Some(vec![Event::Resize(80, 24)]));
        *size.lock().map_err(|_| "poisoned")? = (132, 43);
        assert_eq!(input.next_frame()?, Some(vec![Event::Resize(132, 43)]));
        release.send(())?;
        assert_eq!(input.next_frame()?, None);
        Ok(())
    }

    #[test]
    fn ending_input_ends_the_session_even_with_a_size_watcher()
    -> Result<(), Box<dyn std::error::Error>> {
        let (release, held) = mpsc::channel();
        let mut input = Input::from_reader_sized(
            Silent { release: held },
            Box::new(|| None),
            Duration::from_millis(1),
        )?;
        release.send(())?;
        assert_eq!(input.next_frame()?, None);
        Ok(())
    }
}
