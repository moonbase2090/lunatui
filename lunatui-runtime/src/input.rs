// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use lunatui_term::{Event, Parser};

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
        thread::Builder::new()
            .name(String::from("lunatui-input"))
            .spawn(move || {
                let mut parser = Parser::new();
                let mut buffer = [0; 4096];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            let events = parser.flush();
                            if !events.is_empty() {
                                let _ = sender.send(Ok(events));
                            }
                            return;
                        }
                        Ok(length) => {
                            let events = parser.push(&buffer[..length]);
                            if !events.is_empty() && sender.send(Ok(events)).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = sender.send(Err(error));
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

    use lunatui_term::{Event, KeyCode, KeyEvent};

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
}
