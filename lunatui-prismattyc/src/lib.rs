// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod frame;
mod native;
mod native_frame;
mod sink;

pub use frame::{FrameDiff, PresentStats};
pub use native::{
    DISCOVERY_BUDGET, DiscoveryDecoder, DiscoveryResult, MAX_APC_BODY, MAX_CAPABILITY_VERSION,
    NATIVE_ALPHA_FEATURE, NATIVE_GRID_FEATURE, NativeDiscovery, NativePolicy,
    ansi_fallback_repaint_prefix, discover_from_input, discovery_query,
};
pub use native_frame::{
    ACTIVE_FLAG_ALPHA, ATTRIBUTE_RECORD_BYTES, AttributeMode, EncodeError, FRAME_HEADER_BYTES,
    FrameEncoder, MAX_FRAME_AXIS, MAX_FRAME_CELLS, RING_ABI_MINOR_ALPHA, SLOT_BYTES,
};
pub use sink::{HostRenderer, PrismattycBackend};
