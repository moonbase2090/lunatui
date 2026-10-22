// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

#![doc = include_str!("../README.md")]

mod frame;
mod sink;

pub use frame::{FrameDiff, PresentStats};
pub use sink::{HostRenderer, PrismattycBackend};
