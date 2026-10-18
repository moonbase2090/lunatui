// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

mod ascii_width;
mod detect;
mod first_mismatch;
mod row_diff;

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::{AtomicU8, Ordering};

pub use ascii_width::ascii_display_width_scalar;
pub use detect::{DispatchLevel, active_dispatch_level, differential_levels};
pub use row_diff::{collect_runs_in_row_scalar, collect_runs_in_row_simd};

/// Finds the first index where `a` and `b` differ, including length mismatch past the shared prefix.
pub type FirstMismatchFn = fn(&[u128], &[u128]) -> Option<usize>;

/// Counts printable ASCII terminal columns (`0x20`..=`0x7e`); `text` must be ASCII.
pub type AsciiDisplayWidthFn = fn(&str) -> usize;

struct KernelTable {
    level: DispatchLevel,
    first_mismatch: FirstMismatchFn,
    ascii_display_width: AsciiDisplayWidthFn,
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
    ascii_display_width: ascii_display_width_scalar,
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
        ascii_display_width: ascii_display_width_for_level(level),
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

/// Dispatched printable-ASCII width; `text` must be ASCII.
#[must_use]
pub fn ascii_display_width(text: &str) -> usize {
    debug_assert!(text.is_ascii());
    let table = kernel_table();
    (table.ascii_display_width)(text)
}

/// Selects the `first_mismatch` implementation for a specific level (differential harness).
#[must_use]
pub fn first_mismatch_for_level(level: DispatchLevel) -> FirstMismatchFn {
    match level {
        DispatchLevel::Scalar => first_mismatch_scalar,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Sse2 => first_mismatch_sse2,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Avx2 if detect::x86_avx2_available() => first_mismatch_avx2,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Avx2 => first_mismatch_sse2,
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

/// Selects the printable-ASCII width kernel for a specific level (differential harness).
#[must_use]
pub fn ascii_display_width_for_level(level: DispatchLevel) -> AsciiDisplayWidthFn {
    match level {
        DispatchLevel::Scalar => ascii_display_width_scalar,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Sse2 => ascii_display_width_sse2,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Avx2 if detect::x86_avx2_available() => ascii_display_width_avx2,
        #[cfg(target_arch = "x86_64")]
        DispatchLevel::Avx2 => ascii_display_width_sse2,
        #[cfg(target_arch = "aarch64")]
        DispatchLevel::Neon => ascii_display_width_neon,
    }
}

/// Returns `true` when every compiled dispatch level agrees with [`ascii_display_width_scalar`].
#[must_use]
pub fn ascii_display_width_levels_agree(text: &str) -> bool {
    debug_assert!(text.is_ascii());
    let reference = ascii_display_width_scalar(text);
    differential_levels()
        .iter()
        .all(|&level| ascii_display_width_for_level(level)(text) == reference)
}

#[cfg(target_arch = "x86_64")]
fn ascii_display_width_sse2(text: &str) -> usize {
    ascii_width::ascii_display_width_sse2(text)
}

#[cfg(target_arch = "x86_64")]
fn ascii_display_width_avx2(text: &str) -> usize {
    ascii_width::ascii_display_width_avx2(text)
}

#[cfg(target_arch = "aarch64")]
fn ascii_display_width_neon(text: &str) -> usize {
    ascii_width::ascii_display_width_neon(text)
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
