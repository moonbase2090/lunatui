// SPDX-License-Identifier: MIT
use sha2::{Digest, Sha256};
use std::{
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    sync::atomic::{AtomicBool, Ordering},
};

pub const KIND: u16 = if cfg!(target_os = "macos") { 2 } else { 1 };

pub fn validate(
    descriptor: BorrowedFd<'_>,
    logical: u64,
    physical: u64,
    kind: u16,
    image: bool,
) -> io::Result<()> {
    if logical == 0 || kind != KIND {
        return Err(crate::invalid("backing kind"));
    }
    let raw = descriptor.as_raw_fd();
    // SAFETY: `raw` is borrowed from the live `BorrowedFd` above.
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if flags & libc::FD_CLOEXEC == 0 {
        return Err(crate::invalid("backing close-on-exec"));
    }
    // SAFETY: `stat` is a C output struct that fstat fully initializes.
    let mut status: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: `raw` is live and `status` is a valid writable output pointer.
    if unsafe { libc::fstat(raw, &raw mut status) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: geteuid has no pointer arguments or preconditions.
    if status.st_uid != unsafe { libc::geteuid() }
        || u64::try_from(status.st_size).ok() != Some(physical)
    {
        return Err(crate::invalid("backing owner or extent"));
    }
    #[cfg(target_os = "linux")]
    {
        let required =
            libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | if image { libc::F_SEAL_WRITE } else { 0 };
        // SAFETY: `raw` remains borrowed from the live descriptor.
        let seals = unsafe { libc::fcntl(raw, libc::F_GET_SEALS) };
        if status.st_mode & libc::S_IFMT != libc::S_IFREG
            || physical != logical
            || seals < 0
            || seals & required != required
        {
            return Err(crate::invalid("sealed backing required"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = image;
        if !cfg!(target_arch = "aarch64") {
            return Err(crate::invalid("unproven Darwin execution mode"));
        }
        // SAFETY: this C-layout output struct is fully initialized by proc_pidfdinfo.
        let mut typed: PshmFdInfo = unsafe { std::mem::zeroed() };
        // SAFETY: `raw` is live and `typed` is a valid writable buffer of its declared size.
        let n = unsafe {
            libc::proc_pidfdinfo(
                libc::getpid(),
                raw,
                5,
                (&mut typed as *mut PshmFdInfo).cast(),
                std::mem::size_of_val(&typed) as _,
            )
        };
        if n != std::mem::size_of_val(&typed) as i32
            || typed.pshm.stat.vst_uid != status.st_uid
            || typed.pshm.stat.vst_size != status.st_size
            || physical < logical
            || logical
                .checked_add(65536)
                .is_none_or(|limit| physical >= limit)
        {
            return Err(crate::invalid("typed fixed PSHM required"));
        }
    }
    Ok(())
}

/// A validated, immutable image backing ready to upload to a native host.
pub struct PreparedImage {
    /// Read-only descriptor supplied to the host during image admission.
    pub descriptor: OwnedFd,
    /// Allocated backing extent in bytes.
    pub physical: u64,
    /// SHA-256 of the exact RGBA payload.
    pub digest: [u8; 32],
}

impl PreparedImage {
    /// Run on the one preparation worker, never in the heartbeat/event loop.
    /// Copies RGBA pixels into a validated platform backing object.
    pub fn prepare(pixels: &[u8]) -> io::Result<Self> {
        Self::prepare_cancellable(pixels, &AtomicBool::new(false))
    }

    /// Copies pixels while allowing the caller to cancel an in-progress preparation.
    pub fn prepare_cancellable(pixels: &[u8], cancelled: &AtomicBool) -> io::Result<Self> {
        if pixels.is_empty() || pixels.len() > 64 * 1024 * 1024 {
            return Err(crate::invalid("image byte limit"));
        }
        #[cfg(target_os = "linux")]
        // SAFETY: the name is a static NUL-terminated C string and flags are valid.
        let raw = unsafe {
            libc::memfd_create(
                c"native-client-image".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
            )
        };
        #[cfg(target_os = "macos")]
        let raw = {
            let mut random = [0; 12];
            getrandom::fill(&mut random).map_err(|error| io::Error::other(error.to_string()))?;
            let suffix: String = random.iter().map(|value| format!("{value:02x}")).collect();
            let name = std::ffi::CString::new(format!("/ncp{suffix}"))
                .map_err(|error| io::Error::other(error.to_string()))?;
            // SAFETY: `name` is NUL terminated and shm_open copies it during this call.
            let raw = unsafe {
                libc::shm_open(
                    name.as_ptr(),
                    libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                    0o600,
                )
            };
            if raw < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `name` remains a live NUL-terminated string.
            if unsafe { libc::shm_unlink(name.as_ptr()) } != 0 {
                let error = io::Error::last_os_error();
                // SAFETY: `raw` is the newly created descriptor and is not yet owned.
                unsafe { libc::close(raw) };
                return Err(error);
            }
            raw
        };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful memfd_create/shm_open returned a fresh owned descriptor.
        let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
        // SAFETY: `raw` is owned above and the bounded pixel extent fits off_t on supported targets.
        if unsafe { libc::ftruncate(raw, pixels.len() as libc::off_t) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the descriptor has the exact size set above and the mapping length is nonzero.
        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                pixels.len(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                raw,
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let mut digest = Sha256::new();
        let mut aborted = false;
        for (index, chunk) in pixels.chunks(65536).enumerate() {
            if cancelled.load(Ordering::Acquire) {
                aborted = true;
                break;
            }
            // SAFETY: each chunk and its destination are within the exact-length mapping.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    chunk.as_ptr(),
                    address.cast::<u8>().add(index * 65536),
                    chunk.len(),
                )
            };
            digest.update(chunk);
        }
        // SAFETY: `address` is the successful mapping returned above.
        unsafe { libc::munmap(address, pixels.len()) };
        if aborted {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "image preparation cancelled",
            ));
        }
        #[cfg(target_os = "linux")]
        // SAFETY: `raw` is a live memfd created with MFD_ALLOW_SEALING.
        if unsafe {
            libc::fcntl(
                raw,
                libc::F_ADD_SEALS,
                libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE | libc::F_SEAL_SEAL,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fstat fully initializes this C output struct.
        let mut status: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: `raw` is live and `status` is a valid writable output pointer.
        if unsafe { libc::fstat(raw, &raw mut status) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let physical =
            u64::try_from(status.st_size).map_err(|_| crate::invalid("negative source extent"))?;
        validate(
            descriptor.as_fd(),
            pixels.len() as u64,
            physical,
            KIND,
            true,
        )?;
        Ok(Self {
            descriptor,
            physical,
            digest: digest.finalize().into(),
        })
    }
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct ProcFileInfo {
    flags: u32,
    status: u32,
    offset: i64,
    kind: i32,
    guards: u32,
}
#[cfg(target_os = "macos")]
#[repr(C)]
struct PshmInfo {
    stat: libc::vinfo_stat,
    address: u64,
    name: [libc::c_char; 1024],
}
#[cfg(target_os = "macos")]
#[repr(C)]
struct PshmFdInfo {
    file: ProcFileInfo,
    pshm: PshmInfo,
}
#[cfg(target_os = "macos")]
const _: () = {
    assert!(std::mem::size_of::<PshmFdInfo>() == 1192);
    assert!(std::mem::offset_of!(PshmFdInfo, pshm) == 24);
};
