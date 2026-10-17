// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod detect;
mod first_mismatch;

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::{AtomicU8, Ordering};

pub use detect::{DispatchLevel, active_dispatch_level, differential_levels};

/// Finds the first index where `a` and `b` differ, including length mismatch past the shared prefix.
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

// SAFETY: `KERNELS` is published only after `STATE` reaches `STATE_READY`.
unsafe impl Sync for SyncKernelTable {}

const STATE_UNINIT: u8 = 0;
const STATE_INITING: u8 = 1;
const STATE_READY: u8 = 2;

static STATE: AtomicU8 = AtomicU8::new(STATE_UNINIT);
static KERNELS: SyncKernelTable = SyncKernelTable::new(KernelTable {
    level: DispatchLevel::Scalar,
    first_mismatch: first_mismatch_scalar,
});

fn kernel_table() -> &'static KernelTable {
    loop {
        if STATE.load(Ordering::Acquire) == STATE_READY {
            break;
        }
        try_publish_kernel_table();
        spin_loop();
    }
    // SAFETY: the loop exits only after `STATE_READY` publishes `KERNELS`.
    unsafe { &*KERNELS.0.get() }
}

fn try_publish_kernel_table() {
    if STATE
        .compare_exchange_weak(
            STATE_UNINIT,
            STATE_INITING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return;
    }
    let level = active_dispatch_level();
    let table = KernelTable {
        level,
        first_mismatch: first_mismatch_for_level(level),
    };
    // SAFETY: exclusive initializer; readers wait until `STATE_READY`.
    unsafe {
        *KERNELS.0.get() = table;
    }
    STATE.store(STATE_READY, Ordering::Release);
}

/// Active runtime dispatch level (initialized once on first kernel use).
#[must_use]
pub fn runtime_dispatch_level() -> DispatchLevel {
    kernel_table().level
}

/// Scalar reference: first unequal `u128` cell index, or `None` when the slices are equal.
#[must_use]
pub fn first_mismatch_scalar(a: &[u128], b: &[u128]) -> Option<usize> {
    let shared = a.len().min(b.len());
    for index in 0..shared {
        if a[index] != b[index] {
            return Some(index);
        }
    }
    if a.len() == b.len() {
        None
    } else {
        Some(shared)
    }
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
    first_mismatch::first_mismatch_sse2(a, b)
}

#[cfg(target_arch = "x86_64")]
fn first_mismatch_avx2(a: &[u128], b: &[u128]) -> Option<usize> {
    first_mismatch::first_mismatch_avx2(a, b)
}

#[cfg(target_arch = "aarch64")]
fn first_mismatch_neon(a: &[u128], b: &[u128]) -> Option<usize> {
    first_mismatch::first_mismatch_neon(a, b)
}
