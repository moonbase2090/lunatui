// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::{
    io,
    os::fd::{AsFd, AsRawFd, OwnedFd},
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};

const RING_BYTES: usize = 4_198_400;
const SLOT_BYTES: usize = 1_048_576;
const SLOT_BYTES_U32: u32 = 1_048_576;

pub struct Producer {
    address: NonNull<u8>,
    _descriptor: OwnedFd,
    published: u64,
}

impl Producer {
    pub fn from_setup(descriptor: OwnedFd, setup: &[u8]) -> io::Result<Self> {
        if setup.len() != 128
            || crate::u16_at(setup, 4) != 9
            || crate::u16_at(setup, 6) != 0
            || crate::u64_at(setup, 32) != RING_BYTES as u64
            || crate::u32_at(setup, 48) != 4
            || crate::u32_at(setup, 52) != SLOT_BYTES_U32
            || crate::u16_at(setup, 58) != 1
            || crate::u32_at(setup, 60) != 0
            || setup[112..].iter().any(|&byte| byte != 0)
        {
            return Err(crate::invalid("setup ring layout"));
        }
        crate::backing::validate(
            descriptor.as_fd(),
            RING_BYTES as u64,
            crate::u64_at(setup, 40),
            crate::u16_at(setup, 56),
            false,
        )?;
        // SAFETY: validate proved the platform backing cannot shrink below the mapped extent.
        unsafe { Self::from_validated(descriptor, crate::u64_at(setup, 8)) }
    }

    /// The descriptor must already have passed platform backing validation.
    ///
    /// # Safety
    /// The supplied object must remain at least `RING_BYTES` long for this mapping's lifetime.
    pub unsafe fn from_validated(descriptor: OwnedFd, session: u64) -> io::Result<Self> {
        // SAFETY: the method's caller guarantees this object remains at least RING_BYTES long.
        unsafe { Self::from_validated_minor(descriptor, session) }
    }

    unsafe fn from_validated_minor(descriptor: OwnedFd, session: u64) -> io::Result<Self> {
        let address =
            // SAFETY: the caller validated the descriptor extent and this maps exactly RING_BYTES.
            unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                RING_BYTES,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                descriptor.as_raw_fd(),
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let Some(address) = NonNull::new(address.cast::<u8>()) else {
            // SAFETY: mmap succeeded; this releases the complete mapping at its returned address.
            unsafe { libc::munmap(address, RING_BYTES) };
            return Err(crate::invalid("null ring mapping"));
        };
        let producer = Self {
            address,
            _descriptor: descriptor,
            published: 0,
        };
        let mut header = [0; 4096];
        // SAFETY: the validated mapping covers at least one full 4096-byte ring header.
        unsafe {
            std::ptr::copy_nonoverlapping(address.as_ptr(), header.as_mut_ptr(), header.len());
        };
        let mut expected = [0; 4096];
        expected[..8].copy_from_slice(b"LUNARING");
        expected[8..10].copy_from_slice(&1u16.to_le_bytes());
        expected[10..12].copy_from_slice(&2u16.to_le_bytes());
        expected[12..16].copy_from_slice(&4096u32.to_le_bytes());
        expected[16..24].copy_from_slice(&(RING_BYTES as u64).to_le_bytes());
        expected[24..28].copy_from_slice(&4u32.to_le_bytes());
        expected[28..32].copy_from_slice(&SLOT_BYTES_U32.to_le_bytes());
        expected[32..40].copy_from_slice(&session.to_le_bytes());
        if session == 0 || header != expected {
            return Err(crate::invalid("ring header"));
        }
        Ok(producer)
    }

    fn counter(&self, at: usize) -> &AtomicU64 {
        // SAFETY: callers use fixed 8-byte-aligned header offsets in the validated mapping.
        unsafe { &*self.address.cast::<AtomicU64>().as_ptr().add(at / 8) }
    }

    pub fn publish(&mut self, frame: &[u8]) -> io::Result<u64> {
        if frame.len() < 96
            || frame.len() > SLOT_BYTES
            || usize::try_from(crate::u32_at(frame, 0)).ok() != Some(frame.len())
        {
            return Err(crate::invalid("frame size"));
        }
        let next = self
            .published
            .checked_add(1)
            .ok_or_else(|| crate::invalid("sequence exhausted"))?;
        if crate::u64_at(frame, 8) != next {
            return Err(crate::invalid("frame sequence"));
        }
        let consumed = self.counter(128).load(Ordering::Acquire);
        if consumed > self.published {
            return Err(crate::invalid("consumer counter"));
        }
        if self.published - consumed >= 4 {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "ring full"));
        }
        let slot =
            usize::try_from((next - 1) % 4).map_err(|_| crate::invalid("ring slot index"))?;
        let at = 4096 + slot * SLOT_BYTES;
        // SAFETY: slot bounds follow the modulo-4 index and SLOT_BYTES-sized ring layout.
        unsafe {
            let destination = self.address.as_ptr().add(at);
            std::ptr::write_bytes(destination, 0, SLOT_BYTES);
            std::ptr::copy_nonoverlapping(frame.as_ptr(), destination, frame.len());
        }
        self.counter(64).store(next, Ordering::Release);
        self.published = next;
        Ok(next)
    }
}

impl Drop for Producer {
    fn drop(&mut self) {
        // SAFETY: address and length are exactly the successful mapping stored by Producer.
        unsafe { libc::munmap(self.address.as_ptr().cast(), RING_BYTES) };
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::err_expect,
        clippy::expect_used,
        reason = "test fixtures use explicit panic messages for setup and rejection assertions"
    )]

    use super::{Producer, RING_BYTES, SLOT_BYTES_U32};
    use std::{fs::File, os::fd::OwnedFd};

    fn descriptor() -> OwnedFd {
        File::open("/dev/null")
            .expect("open test descriptor")
            .into()
    }

    fn valid_setup() -> [u8; 128] {
        let mut setup = [0; 128];
        setup[4..6].copy_from_slice(&9u16.to_le_bytes());
        setup[8..16].copy_from_slice(&1u64.to_le_bytes());
        setup[32..40].copy_from_slice(&(RING_BYTES as u64).to_le_bytes());
        setup[40..48].copy_from_slice(&(RING_BYTES as u64).to_le_bytes());
        setup[48..52].copy_from_slice(&4u32.to_le_bytes());
        setup[52..56].copy_from_slice(&SLOT_BYTES_U32.to_le_bytes());
        setup[56..58].copy_from_slice(&super::super::backing::KIND.to_le_bytes());
        setup[58..60].copy_from_slice(&1u16.to_le_bytes());
        setup
    }

    #[test]
    fn from_setup_rejects_wrong_record_length() {
        let mut setup = valid_setup().to_vec();
        setup.pop();
        let error = Producer::from_setup(descriptor(), &setup)
            .err()
            .expect("short setup rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn from_setup_rejects_wrong_ring_extent() {
        let mut setup = valid_setup();
        setup[32..40].copy_from_slice(&1u64.to_le_bytes());
        let error = Producer::from_setup(descriptor(), &setup)
            .err()
            .expect("wrong ring extent rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn from_setup_rejects_wrong_slot_count() {
        let mut setup = valid_setup();
        setup[48..52].copy_from_slice(&3u32.to_le_bytes());
        let error = Producer::from_setup(descriptor(), &setup)
            .err()
            .expect("wrong slot count rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn from_setup_rejects_nonzero_reserved_tail() {
        let mut setup = valid_setup();
        setup[127] = 1;
        let error = Producer::from_setup(descriptor(), &setup)
            .err()
            .expect("nonzero reserved tail rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
