// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

//! SIMD dispatch framework: scalar reference kernels and runtime-selected implementations.
//!
//! `unsafe` stays in this module (PRD §10). Production behavior is unchanged in roadmap PR 47;
//! later PRs swap in vectorized bodies behind the same function pointers.

mod detect;

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

pub use detect::{DispatchLevel, active_dispatch_level, differential_levels};

/// Finds the first index where `a` and `b` differ; `None` when lengths match and all cells are equal.
pub type FirstMismatchFn = fn(&[u128], &[u128]) -> Option<usize>;

struct KernelTable {
    level: DispatchLevel,
    first_mismatch: FirstMismatchFn,
}

struct SyncKernelTable(UnsafeCell<KernelTable>);

impl SyncKernelTable {
    const fn new(table: KernelTable) -> Self {
        Self(UnsafeCell::new(table))
    }
}

// SAFETY: `KERNELS` is written once before `INIT` is set, then read-only.
unsafe impl Sync for SyncKernelTable {}

static INIT: AtomicBool = AtomicBool::new(false);
static KERNELS: SyncKernelTable = SyncKernelTable::new(KernelTable {
    level: DispatchLevel::Scalar,
    first_mismatch: first_mismatch_scalar,
});

fn kernel_table() -> &'static KernelTable {
    if !INIT.load(Ordering::Acquire) {
        let level = active_dispatch_level();
        let table = KernelTable {
            level,
            first_mismatch: first_mismatch_for_level(level),
        };
        // SAFETY: `INIT` is false, so no other thread has published `KERNELS` yet.
        unsafe {
            *KERNELS.0.get() = table;
        }
        INIT.store(true, Ordering::Release);
    }
    // SAFETY: `INIT` is true, so `KERNELS` was written exactly once and is only read afterward.
    unsafe { &*KERNELS.0.get() }
}

/// Active runtime dispatch level (initialized once on first kernel use).
#[must_use]
pub fn runtime_dispatch_level() -> DispatchLevel {
    kernel_table().level
}

/// Scalar reference: first unequal `u128` cell index, or `None` when equal.
#[must_use]
pub fn first_mismatch_scalar(a: &[u128], b: &[u128]) -> Option<usize> {
    if a.len() != b.len() {
        return None;
    }
    for (index, (left, right)) in a.iter().zip(b.iter()).enumerate() {
        if left != right {
            return Some(index);
        }
    }
    None
}

/// Dispatched kernel using the cached runtime level.
#[must_use]
pub fn first_mismatch(a: &[u128], b: &[u128]) -> Option<usize> {
    let table = kernel_table();
    (table.first_mismatch)(a, b)
}

/// Selects the `first_mismatch` implementation for a specific level (differential harness).
#[must_use]
pub fn first_mismatch_for_level(level: DispatchLevel) -> FirstMismatchFn {
    match level {
        DispatchLevel::Scalar => first_mismatch_scalar,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Sse2 => first_mismatch_sse2,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Avx2 => first_mismatch_avx2,
        #[cfg(target_arch = "aarch64")]
        DispatchLevel::Neon => first_mismatch_neon,
    }
}

/// Returns `true` when every compiled dispatch level agrees with `first_mismatch_scalar`.
#[must_use]
pub fn first_mismatch_levels_agree(a: &[u128], b: &[u128]) -> bool {
    let reference = first_mismatch_scalar(a, b);
    differential_levels()
        .iter()
        .all(|&level| first_mismatch_for_level(level)(a, b) == reference)
}

#[cfg(target_arch = "x86_64")]
fn first_mismatch_sse2(a: &[u128], b: &[u128]) -> Option<usize> {
    // PR 47: vector bodies land in PR 48; SSE2 path is wired for dispatch testing.
    first_mismatch_scalar(a, b)
}

#[cfg(target_arch = "x86_64")]
fn first_mismatch_avx2(a: &[u128], b: &[u128]) -> Option<usize> {
    first_mismatch_scalar(a, b)
}

#[cfg(target_arch = "aarch64")]
fn first_mismatch_neon(a: &[u128], b: &[u128]) -> Option<usize> {
    first_mismatch_scalar(a, b)
}
