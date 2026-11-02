// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

use super::PtySize;

const MAX_INPUT: usize = 65_536;

/// One Unix child, its controlling PTY and a bounded, nonblocking input queue.
///
/// The app owns polling. No reader thread or outer-terminal session is created.
/// Dropping a running process kills its process group and reaps the direct child.
/// After `try_wait` has reaped a child, cleanup never signals its former PID.
#[derive(Debug)]
pub struct PtyProcess {
    master: File,
    child: Child,
    size: PtySize,
    input: VecDeque<u8>,
    status: Option<ExitStatus>,
    eof: bool,
}

impl PtyProcess {
    /// Starts a command with stdin, stdout and stderr attached to a new controlling PTY.
    ///
    /// Arguments and working directory remain those of the command. The helper advertises
    /// xterm-256color with truecolor and removes outer graphics/native terminal identifiers.
    /// It replaces any configured standard streams.
    ///
    /// # Errors
    ///
    /// Returns allocation, descriptor, session or executable-start errors.
    pub fn spawn(mut command: Command, size: PtySize) -> io::Result<Self> {
        let (master, slave) = open(size)?;
        command
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave))
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor");
        for name in [
            "PRISMATTYC_NATIVE_FD",
            "KITTY_WINDOW_ID",
            "WEZTERM_PANE",
            "TERM_PROGRAM",
            "TERM_PROGRAM_VERSION",
        ] {
            command.env_remove(name);
        }
        // SAFETY: after fork this closure only calls async-signal-safe OS functions.
        // The standard streams are already the live slave descriptors prepared above.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1
                    || libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY.into(), 0) == -1
                {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn()?;
        Ok(Self {
            master,
            child,
            size,
            input: VecDeque::new(),
            status: None,
            eof: false,
        })
    }

    /// The direct child's PID, for diagnostics.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// The dimensions last accepted by the OS.
    #[must_use]
    pub const fn size(&self) -> PtySize {
        self.size
    }

    /// Changes the kernel PTY size. The kernel notifies its foreground process group.
    ///
    /// # Errors
    ///
    /// Returns the OS resize error without changing the stored dimensions.
    pub fn resize(&mut self, size: PtySize) -> io::Result<()> {
        if size == self.size {
            return Ok(());
        }
        let winsize = window_size(size);
        // SAFETY: master is an owned PTY descriptor and winsize is a valid input structure.
        if unsafe {
            libc::ioctl(
                self.master.as_raw_fd(),
                libc::TIOCSWINSZ,
                &raw const winsize,
            )
        } == -1
        {
            return Err(io::Error::last_os_error());
        }
        self.size = size;
        Ok(())
    }

    /// Queues an entire input chunk without accepting a partial chunk.
    ///
    /// # Errors
    ///
    /// Returns `WouldBlock` if the 64 KiB queue would overflow, or `BrokenPipe`
    /// if the direct child has already been reaped.
    pub fn queue_input(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.status.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "child has exited",
            ));
        }
        if self.input.len().saturating_add(bytes.len()) > MAX_INPUT {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "child input queue is full",
            ));
        }
        self.input.extend(bytes);
        Ok(())
    }

    /// Bytes still waiting for the child to read.
    #[must_use]
    pub fn pending_input(&self) -> usize {
        self.input.len()
    }

    /// Writes queued input until the PTY would block. Unwritten bytes stay queued.
    ///
    /// # Errors
    ///
    /// Returns permanent write errors. Poll again on a later app frame after success.
    pub fn flush_input(&mut self) -> io::Result<()> {
        while !self.input.is_empty() {
            let (first, _) = self.input.as_slices();
            match self.master.write(first) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "PTY accepted no input",
                    ));
                }
                Ok(written) => drop(self.input.drain(..written)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Polls and caches the direct child's exit status without blocking.
    ///
    /// # Errors
    ///
    /// Returns the OS wait error. Continue draining output after a child exits.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.status.is_none() {
            self.status = self.child.try_wait()?;
        }
        Ok(self.status)
    }

    /// Kills the running child's process group and reaps the direct child.
    ///
    /// A previously reaped child returns its cached status without sending a signal.
    /// Detached descendants are outside this process group's ownership.
    ///
    /// # Errors
    ///
    /// Returns an OS signal or wait error.
    pub fn terminate(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.status {
            return Ok(status);
        }
        let pid = i32::try_from(self.child.id()).map_err(io::Error::other)?;
        // SAFETY: setsid made this unreaped child its own process-group leader.
        // Its PID cannot be reused before we reap it; no signal is sent after reaping.
        if unsafe { libc::kill(-pid, libc::SIGKILL) } == -1 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
        let status = self.child.wait()?;
        self.status = Some(status);
        Ok(status)
    }
}

impl Read for PtyProcess {
    /// Reads without blocking. `WouldBlock` means no child output is available yet.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.eof {
            return Ok(0);
        }
        match self.master.read(buffer) {
            Ok(0) => {
                self.eof = true;
                Ok(0)
            }
            // Linux reports EIO after all slave descriptors close.
            Err(error) if error.raw_os_error() == Some(libc::EIO) => {
                self.eof = true;
                Ok(0)
            }
            result => result,
        }
    }
}

impl Drop for PtyProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

fn window_size(size: PtySize) -> libc::winsize {
    libc::winsize {
        ws_col: size.cols(),
        ws_row: size.rows(),
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}

fn open(size: PtySize) -> io::Result<(File, File)> {
    let mut master = -1;
    let mut slave = -1;
    let mut winsize = window_size(size);
    // SAFETY: output pointers and the initialized size are valid. No termios is supplied.
    if unsafe {
        libc::openpty(
            &raw mut master,
            &raw mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &raw mut winsize,
        )
    } == -1
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openpty returned two distinct owned descriptors.
    let master = unsafe { File::from_raw_fd(master) };
    // SAFETY: ownership of the second descriptor has not been transferred before this point.
    let slave = unsafe { File::from_raw_fd(slave) };
    for file in [&master, &slave] {
        // SAFETY: file owns this live descriptor; fcntl does not retain a pointer.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: master is an owned descriptor and these commands take integer flags only.
    let flags = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the owned descriptor receives its existing flags plus nonblocking mode.
    let nonblocking =
        unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if nonblocking == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok((master, slave))
}
