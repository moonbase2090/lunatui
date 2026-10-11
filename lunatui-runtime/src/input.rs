// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use lunatui_term::{Event, Parser};

/// Reads terminal input on a worker thread and groups available events into frames.
pub struct Input {
    receiver: Receiver<io::Result<Vec<Event>>>,
    pending_error: Option<io::Error>,
    finished: bool,
}

impl Input {
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
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use lunatui_term::{Event, KeyCode, KeyEvent};

    use crate::Input;

    struct FakeSource {
        bytes: Vec<u8>,
        delivered: bool,
    }

    impl FakeSource {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                bytes,
                delivered: false,
            }
        }
    }

    impl Read for FakeSource {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.delivered {
                return Ok(0);
            }
            self.delivered = true;
            let length = self.bytes.len().min(buffer.len());
            buffer[..length].copy_from_slice(&self.bytes[..length]);
            Ok(length)
        }
    }

    #[test]
    fn bursts_coalesce_to_one_frame() -> Result<(), Box<dyn std::error::Error>> {
        let source = FakeSource::new(b"abc".to_vec());
        let mut input = Input::from_reader(source)?;

        let frame = input.next_frame()?.ok_or("source ended before its burst")?;

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
