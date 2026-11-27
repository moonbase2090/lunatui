// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../../README.md")]
use std::{
    env, io,
    net::Shutdown,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    time::{Duration, Instant},
};

pub struct Reply {
    /// Validated host response bytes.
    pub bytes: Vec<u8>,
    /// Descriptor transferred with the host response, if present.
    pub descriptor: Option<OwnedFd>,
}

struct Operation {
    request: Vec<u8>,
    descriptor: Option<OwnedFd>,
    sent: usize,
    response: Vec<u8>,
    received: usize,
    rights: Vec<OwnedFd>,
    expected_opcode: u16,
    expected_bytes: usize,
    deadline: Instant,
    stale_records: usize,
    stale_bytes: usize,
}

pub struct Control {
    socket: UnixStream,
    operation: Option<Operation>,
    closed: bool,
    legacy: bool,
}

impl Control {
    pub fn exchange(
        &mut self,
        request: Vec<u8>,
        descriptor: Option<OwnedFd>,
        deadline: Instant,
    ) -> io::Result<Reply> {
        self.begin(request, descriptor, Instant::now())?;
        loop {
            let now = Instant::now();
            if now >= deadline {
                self.operation = None;
                self.closed = true;
                let _ = self.socket.shutdown(Shutdown::Both);
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "control exchange deadline",
                ));
            }
            if let Some(reply) = self.step(now)? {
                return Ok(reply);
            }
            let mut descriptor = libc::pollfd {
                fd: self.raw_fd(),
                events: if self.wants_write() {
                    libc::POLLOUT
                } else {
                    libc::POLLIN
                },
                revents: 0,
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            let milliseconds = i32::try_from(remaining.as_millis().max(1).min(i32::MAX as u128))
                .unwrap_or(i32::MAX);
            let result =
                // SAFETY: `descriptor` is one initialized pollfd borrowing our live socket.
                unsafe { libc::poll(&raw mut descriptor, 1, milliseconds) };
            if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                return Err(io::Error::last_os_error());
            }
        }
    }

    pub fn new(socket: UnixStream) -> io::Result<Self> {
        Self::with_dialect(socket, false)
    }

    fn with_dialect(socket: UnixStream, legacy: bool) -> io::Result<Self> {
        verify_peer(&socket)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            operation: None,
            closed: false,
            legacy,
        })
    }

    pub fn raw_fd(&self) -> i32 {
        self.socket.as_raw_fd()
    }
    pub fn wants_write(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|op| op.sent < op.request.len())
    }

    pub fn begin(
        &mut self,
        request: Vec<u8>,
        descriptor: Option<OwnedFd>,
        now: Instant,
    ) -> io::Result<()> {
        if self.closed || self.operation.is_some() {
            return Err(crate::invalid("control unavailable"));
        }
        if request.len() < 32
            || request.len() > 4096
            || crate::u32_at(&request, 0) as usize != request.len()
        {
            return Err(crate::invalid("request size"));
        }
        let opcode = crate::u16_at(&request, 4);
        let (expected_opcode, expected_bytes) = match opcode {
            1 if self.legacy => (2, 48),
            1 => (9, 128),
            3 => (4, 40),
            5 => (6, 32),
            7 if self.legacy => (8, 32),
            7 => (15, 48),
            10 if !self.legacy => (11, 48),
            12 if !self.legacy => (13, 48),
            _ => return Err(crate::invalid("request opcode")),
        };
        if descriptor.is_some() != (opcode == 10) {
            return Err(crate::invalid("request descriptors"));
        }
        self.operation = Some(Operation {
            request,
            descriptor,
            sent: 0,
            response: vec![0; 4],
            received: 0,
            rights: Vec::new(),
            expected_opcode,
            expected_bytes,
            deadline: now + Duration::from_millis(200),
            stale_records: 0,
            stale_bytes: 0,
        });
        Ok(())
    }

    pub fn step(&mut self, now: Instant) -> io::Result<Option<Reply>> {
        let result = self.step_inner(now);
        if result.is_err() {
            self.operation = None;
            self.closed = true;
            let _ = self.socket.shutdown(Shutdown::Both);
        }
        result
    }

    #[expect(
        clippy::too_many_lines,
        reason = "A control step validates reply sequence, opcode, size, and descriptor ownership before returning one record."
    )]
    fn step_inner(&mut self, now: Instant) -> io::Result<Option<Reply>> {
        let op = self
            .operation
            .as_mut()
            .ok_or_else(|| crate::invalid("no operation"))?;
        if now >= op.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "control operation deadline",
            ));
        }
        if op.sent < op.request.len() {
            let n = match send(&self.socket, &op.request[op.sent..], op.descriptor.as_ref()) {
                Ok(n) => n,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "control write"));
            }
            op.sent += n;
            op.descriptor = None;
            if op.sent < op.request.len() {
                return Ok(None);
            }
        }
        for _ in 0..2 {
            let received = match receive(&self.socket, &mut op.response[op.received..]) {
                Ok(received) => received,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            if received.bytes == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "control peer closed",
                ));
            }
            op.received += received.bytes;
            op.rights.extend(received.descriptors);
            if op.rights.len() > 16 {
                return Err(crate::invalid("descriptor bound"));
            }
            if op.received != op.response.len() {
                return Ok(None);
            }
            if op.response.len() == 4 {
                let length = crate::u32_at(&op.response, 0) as usize;
                if !(32..=4096).contains(&length) {
                    return Err(crate::invalid("response size"));
                }
                op.response.resize(length, 0);
                continue;
            }
            let r = &op.response;
            let epoch = crate::u64_at(r, 24);
            let wanted_epoch = crate::u64_at(&op.request, 24);
            if epoch < wanted_epoch && crate::u16_at(&op.request, 4) == 1 {
                op.stale_records += 1;
                op.stale_bytes += r.len();
                if op.stale_records > 4 || op.stale_bytes > 16384 {
                    return Err(crate::invalid("stale response bound"));
                }
                validate_reply_shape(r, &op.rights, true, self.legacy)?;
                op.rights.clear();
                op.response = vec![0; 4];
                op.received = 0;
                return Ok(None);
            }
            let opcode = crate::u16_at(r, 4);
            if epoch != wanted_epoch
                || crate::u64_at(r, 16) != crate::u64_at(&op.request, 16)
                || (crate::u16_at(&op.request, 4) != 1
                    && crate::u64_at(r, 8) != crate::u64_at(&op.request, 8))
            {
                return Err(crate::invalid("response identity"));
            }
            validate_reply_shape(r, &op.rights, false, self.legacy)?;
            if opcode == 14 {
                if crate::u64_at(r, 32) <= wanted_epoch
                    || !(1..=4).contains(&crate::u32_at(r, 40))
                    || crate::u32_at(r, 44) != 0
                {
                    return Err(crate::invalid("session-ended body"));
                }
                let op = self
                    .operation
                    .take()
                    .ok_or_else(|| crate::invalid("control operation disappeared"))?;
                return Ok(Some(Reply {
                    bytes: op.response,
                    descriptor: None,
                }));
            }
            if opcode != op.expected_opcode || r.len() != op.expected_bytes {
                return Err(crate::invalid("unexpected response"));
            }
            let op = self
                .operation
                .take()
                .ok_or_else(|| crate::invalid("control operation disappeared"))?;
            return Ok(Some(Reply {
                bytes: op.response,
                descriptor: op.rights.into_iter().next(),
            }));
        }
        Ok(None)
    }
}

/// Duplicates the inherited host endpoint so the caller owns an independently closeable stream.
pub fn inherited_stream() -> io::Result<Option<UnixStream>> {
    let Some(value) = env::var_os("PRISMATTYC_NATIVE_FD") else {
        return Ok(None);
    };
    let descriptor = value
        .to_str()
        .and_then(|text| text.parse::<i32>().ok())
        .filter(|fd| *fd >= 0)
        .ok_or_else(|| crate::invalid("invalid native descriptor environment"))?;
    let duplicate =
        // SAFETY: the descriptor number was parsed from the inherited process environment.
        unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, 3) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    let owned =
        // SAFETY: successful F_DUPFD_CLOEXEC returned a new owned descriptor.
        unsafe { OwnedFd::from_raw_fd(duplicate) };
    let stream = UnixStream::from(owned);
    verify_peer(&stream)?;
    let flags =
        // SAFETY: peer credentials above validated this inherited Unix socket descriptor.
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if flags & libc::FD_CLOEXEC == 0
        &&
            // SAFETY: the inherited descriptor is still open and peer-validated.
            unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(Some(stream))
}

struct Received {
    bytes: usize,
    descriptors: Vec<OwnedFd>,
}

#[expect(
    clippy::too_many_lines,
    reason = "Ancillary descriptor validation must happen in one ordered pass before any received descriptor is returned."
)]
fn receive(socket: &UnixStream, output: &mut [u8]) -> io::Result<Received> {
    let mut ancillary = [0usize; 512];
    let mut vector = libc::iovec {
        iov_base: output.as_mut_ptr().cast(),
        iov_len: output.len(),
    };
    let mut message: libc::msghdr =
        // SAFETY: all-zero is a valid initial state for this C ABI structure.
        unsafe { std::mem::zeroed() };
    message.msg_iov = &raw mut vector;
    message.msg_iovlen = 1;
    message.msg_control = ancillary.as_mut_ptr().cast();
    #[cfg(target_os = "linux")]
    {
        message.msg_controllen = std::mem::size_of_val(&ancillary);
    }
    #[cfg(target_os = "macos")]
    {
        message.msg_controllen = std::mem::size_of_val(&ancillary)
            .try_into()
            .map_err(|_| crate::invalid("ancillary buffer size"))?;
    }
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC;
    #[cfg(target_os = "macos")]
    let flags = libc::MSG_DONTWAIT;
    let count =
        // SAFETY: the iovec and ancillary arrays remain live and writable for this call.
        unsafe { libc::recvmsg(socket.as_raw_fd(), &raw mut message, flags) };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut descriptors = Vec::new();
    let mut invalid = message.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0;
    let control_len = message.msg_controllen as usize;
    let capacity = std::mem::size_of_val(&ancillary);
    invalid |= control_len > capacity;
    #[cfg(target_os = "linux")]
    {
        message.msg_controllen = control_len.min(capacity);
    }
    #[cfg(target_os = "macos")]
    {
        message.msg_controllen = control_len
            .min(capacity)
            .try_into()
            .map_err(|_| crate::invalid("ancillary length"))?;
    }
    let control_start = message.msg_control as usize;
    let control_end = control_start.saturating_add(control_len.min(capacity));
    let mut header =
        // SAFETY: msg_control points to our initialized ancillary array with bounded length.
        unsafe { libc::CMSG_FIRSTHDR(&raw const message) };
    while !header.is_null() {
        let position = header as usize;
        if position < control_start
            || position
                .checked_add(std::mem::size_of::<libc::cmsghdr>())
                .is_none_or(|end| end > control_end)
        {
            invalid = true;
            break;
        }
        let length =
            // SAFETY: the pointer range check above confirms this header fits in received data.
            unsafe { (*header).cmsg_len as usize };
        let prefix =
            // SAFETY: CMSG_LEN is a pure layout calculation for a zero-length payload.
            unsafe { libc::CMSG_LEN(0) as usize };
        let available = control_end - position;
        let cut = length > available;
        invalid |= cut;
        let copied = length.min(available);
        if copied < prefix
            ||
                // SAFETY: the validated header pointer refers to an initialized cmsghdr.
                unsafe {
                (*header).cmsg_level != libc::SOL_SOCKET || (*header).cmsg_type != libc::SCM_RIGHTS
            }
        {
            invalid = true;
            break;
        }
        let payload = copied - prefix;
        if payload % std::mem::size_of::<i32>() != 0 {
            invalid = true;
        }
        let data =
            // SAFETY: the validated header points to a complete SCM_RIGHTS record.
            unsafe { libc::CMSG_DATA(header) };
        for offset in
            (0..payload / std::mem::size_of::<i32>()).map(|n| n * std::mem::size_of::<i32>())
        {
            let raw =
                // SAFETY: payload bounds and descriptor-sized element offsets were checked above.
                unsafe { data.add(offset).cast::<i32>().read_unaligned() };
            let owned =
                // SAFETY: SCM_RIGHTS delivered each raw descriptor as newly owned by this process.
                unsafe { OwnedFd::from_raw_fd(raw) };
            #[cfg(target_os = "macos")]
            {
                let fd_flags =
                    // SAFETY: the received descriptor is owned and remains open.
                    unsafe { libc::fcntl(owned.as_raw_fd(), libc::F_GETFD) };
                if fd_flags < 0
                    ||
                        // SAFETY: the received descriptor is owned and remains open.
                        unsafe {
                        libc::fcntl(
                            owned.as_raw_fd(),
                            libc::F_SETFD,
                            fd_flags | libc::FD_CLOEXEC,
                        )
                    } < 0
                {
                    invalid = true;
                }
            }
            descriptors.push(owned);
        }
        if cut {
            break;
        }
        header =
            // SAFETY: libc computes the next header within the validated control length.
            unsafe { libc::CMSG_NXTHDR(&raw const message, header) };
    }
    if invalid || descriptors.len() > 16 {
        return Err(crate::invalid("invalid ancillary data"));
    }
    Ok(Received {
        bytes: usize::try_from(count).map_err(|_| crate::invalid("negative message length"))?,
        descriptors,
    })
}

fn validate_reply_shape(
    bytes: &[u8],
    rights: &[OwnedFd],
    stale: bool,
    legacy: bool,
) -> io::Result<()> {
    let opcode = crate::u16_at(bytes, 4);
    let expected = match opcode {
        2 if stale || legacy => 48,
        8 if stale || legacy => 32,
        9 => 128,
        4 => 40,
        6 => 32,
        11 | 13 | 14 | 15 => 48,
        _ => return Err(crate::invalid("reply opcode")),
    };
    if bytes.len() != expected
        || rights.len() != usize::from(opcode == 9 || opcode == 2)
        || crate::u64_at(bytes, 8) == 0
    {
        return Err(crate::invalid("reply layout or descriptors"));
    }
    let flags = crate::u16_at(bytes, 6);
    let active_flags = if legacy { 0 } else { 2 };
    if (opcode == 4
        && if stale {
            flags > 3
        } else {
            flags != active_flags
        })
        || (opcode != 4 && flags != 0)
    {
        return Err(crate::invalid("reply flags"));
    }
    Ok(())
}

fn send(socket: &UnixStream, bytes: &[u8], descriptor: Option<&OwnedFd>) -> io::Result<usize> {
    let mut vector = libc::iovec {
        iov_base: bytes.as_ptr().cast_mut().cast(),
        iov_len: bytes.len(),
    };
    let mut message: libc::msghdr =
        // SAFETY: sendmsg reads fields initialized below from this C structure.
        unsafe { std::mem::zeroed() };
    let mut ancillary = [0usize; 4];
    message.msg_iov = &raw mut vector;
    message.msg_iovlen = 1;
    if let Some(descriptor) = descriptor {
        message.msg_control = ancillary.as_mut_ptr().cast();
        message.msg_controllen =
            // SAFETY: CMSG_SPACE is a pure layout calculation for one descriptor.
            unsafe { libc::CMSG_SPACE(4) } as _;
        // SAFETY: ancillary storage has CMSG_SPACE capacity and the header is initialized below.
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&raw const message);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = libc::CMSG_LEN(4) as _;
            libc::CMSG_DATA(header)
                .cast::<i32>()
                .write_unaligned(descriptor.as_raw_fd());
        }
    }
    let n =
        // SAFETY: the socket and iovec are live; optional ancillary data fits its buffer.
        unsafe {
        libc::sendmsg(
            socket.as_raw_fd(),
            &raw const message,
            libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
        )
    };
    if n < 0 {
        Err(io::Error::last_os_error())
    } else {
        usize::try_from(n).map_err(|_| crate::invalid("negative send length"))
    }
}

fn verify_peer(socket: &UnixStream) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let uid = {
        let (mut uid, mut gid) = (0, 0);
        if
        // SAFETY: getpeereid writes both valid credential output pointers.
        unsafe { libc::getpeereid(socket.as_raw_fd(), &raw mut uid, &raw mut gid) } != 0 {
            return Err(io::Error::last_os_error());
        }
        uid
    };
    #[cfg(target_os = "linux")]
    let uid = {
        let mut credentials: libc::ucred =
            // SAFETY: getsockopt fully initializes this C output struct.
            unsafe { std::mem::zeroed() };
        let mut length = libc::socklen_t::try_from(std::mem::size_of_val(&credentials))
            .map_err(|_| crate::invalid("credential size"))?;
        if
        // SAFETY: socket is a live UnixStream and output pointers reference valid storage.
        unsafe {
            libc::getsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&raw mut credentials).cast(),
                &raw mut length,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if length as usize != std::mem::size_of_val(&credentials) || credentials.pid <= 0 {
            return Err(crate::invalid("peer credentials"));
        }
        credentials.uid
    };
    if uid !=
        // SAFETY: geteuid has no pointer arguments or preconditions.
        unsafe { libc::geteuid() }
    {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "peer UID"));
    }
    Ok(())
}
