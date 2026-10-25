// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod frame;
mod native;
mod sink;

pub use frame::{FrameDiff, PresentStats};
pub use native::{
    DISCOVERY_BUDGET, DiscoveryDecoder, DiscoveryResult, MAX_APC_BODY, MAX_CAPABILITY_VERSION,
    NATIVE_GRID_FEATURE, NativeDiscovery, NativePolicy, ansi_fallback_repaint_prefix,
    discover_from_input, discovery_query,
};
pub use sink::{HostRenderer, PrismattycBackend};
