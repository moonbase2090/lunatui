// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

#[cfg(unix)]
mod client;
mod frame;
mod native;
mod native_frame;
#[cfg(unix)]
pub mod native_image;
mod sink;

#[cfg(unix)]
pub(crate) use client::{backing, control, images, ring};

#[cfg(unix)]
pub(crate) fn invalid(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

#[cfg(unix)]
pub(crate) fn u16_at(bytes: &[u8], at: usize) -> u16 {
    let mut value = [0; 2];
    value.copy_from_slice(&bytes[at..at + 2]);
    u16::from_le_bytes(value)
}
#[cfg(unix)]
pub(crate) fn u32_at(bytes: &[u8], at: usize) -> u32 {
    let mut value = [0; 4];
    value.copy_from_slice(&bytes[at..at + 4]);
    u32::from_le_bytes(value)
}
#[cfg(unix)]
pub(crate) fn u64_at(bytes: &[u8], at: usize) -> u64 {
    let mut value = [0; 8];
    value.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(value)
}

pub use frame::{FrameDiff, PresentStats};
pub use native::{
    DISCOVERY_BUDGET, DiscoveryDecoder, DiscoveryResult, MAX_APC_BODY, MAX_CAPABILITY_VERSION,
    NATIVE_ALPHA_FEATURE, NATIVE_GRID_FEATURE, NATIVE_IMAGES_FEATURE, NativeDiscovery,
    NativePolicy, ansi_fallback_repaint_prefix, discover_from_input, discovery_query,
};
pub use native_frame::{
    ACTIVE_FLAG_ALPHA, ATTRIBUTE_RECORD_BYTES, AttributeMode, EncodeError, FRAME_HEADER_BYTES,
    FrameEncoder, MAX_FRAME_AXIS, MAX_FRAME_CELLS, RING_ABI_MINOR_ALPHA, SLOT_BYTES,
};
pub use sink::{HostRenderer, PrismattycBackend};
