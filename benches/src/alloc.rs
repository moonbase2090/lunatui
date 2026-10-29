// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Allocation counts, either running totals or the difference between two
/// snapshots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocStats {
    /// Calls to `alloc` and `alloc_zeroed`.
    pub allocations: usize,
    /// Calls to `realloc`.
    pub reallocations: usize,
    /// Calls to `dealloc`.
    pub deallocations: usize,
    /// Bytes requested by allocations and reallocations (the new size for a
    /// reallocation).
    pub bytes_allocated: usize,
}

impl AllocStats {
    /// Returns the counts accumulated between `earlier` and `self`.
    #[must_use]
    pub fn since(self, earlier: Self) -> Self {
        Self {
            allocations: self.allocations.saturating_sub(earlier.allocations),
            reallocations: self.reallocations.saturating_sub(earlier.reallocations),
            deallocations: self.deallocations.saturating_sub(earlier.deallocations),
            bytes_allocated: self.bytes_allocated.saturating_sub(earlier.bytes_allocated),
        }
    }
}

/// Wraps an allocator and counts every allocation, reallocation and
/// deallocation made through it.
///
/// [`snapshot`](Self::snapshot) returns process-wide totals.
/// [`measure`](Self::measure) counts only the calling thread, so allocations
/// made by other threads while the closure runs, such as the test harness's
/// own bookkeeping, are not included.
///
/// ```
/// use lunatui_bench::CountingAllocator;
///
/// #[global_allocator]
/// static ALLOC: CountingAllocator = CountingAllocator::system();
///
/// let (buffer, stats) = ALLOC.measure(|| Vec::<u8>::with_capacity(64));
/// assert_eq!(stats.allocations, 1);
/// assert_eq!(stats.bytes_allocated, 64);
/// # drop(buffer);
/// ```
pub struct CountingAllocator<A = System> {
    inner: A,
    allocations: AtomicUsize,
    reallocations: AtomicUsize,
    deallocations: AtomicUsize,
    bytes_allocated: AtomicUsize,
}

impl CountingAllocator<System> {
    /// Counts allocations made through the system allocator.
    #[must_use]
    pub const fn system() -> Self {
        Self::new(System)
    }
}

impl<A> CountingAllocator<A> {
    /// Counts allocations made through `inner`.
    #[must_use]
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            allocations: AtomicUsize::new(0),
            reallocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
            bytes_allocated: AtomicUsize::new(0),
        }
    }

    /// Returns the running totals since the process started.
    #[must_use]
    pub fn snapshot(&self) -> AllocStats {
        AllocStats {
            allocations: self.allocations.load(Ordering::Relaxed),
            reallocations: self.reallocations.load(Ordering::Relaxed),
            deallocations: self.deallocations.load(Ordering::Relaxed),
            bytes_allocated: self.bytes_allocated.load(Ordering::Relaxed),
        }
    }

    /// Runs `work` and returns its result with the allocations it made on the
    /// calling thread.
    pub fn measure<R>(&self, work: impl FnOnce() -> R) -> (R, AllocStats) {
        let before = THREAD_TOTALS.with(Cell::get);
        let result = work();
        (result, THREAD_TOTALS.with(Cell::get).since(before))
    }

    fn record_allocation(&self, bytes: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bytes_allocated.fetch_add(bytes, Ordering::Relaxed);
        record_on_thread(|totals| {
            totals.allocations = totals.allocations.wrapping_add(1);
            totals.bytes_allocated = totals.bytes_allocated.wrapping_add(bytes);
        });
    }

    fn record_reallocation(&self, bytes: usize) {
        self.reallocations.fetch_add(1, Ordering::Relaxed);
        self.bytes_allocated.fetch_add(bytes, Ordering::Relaxed);
        record_on_thread(|totals| {
            totals.reallocations = totals.reallocations.wrapping_add(1);
            totals.bytes_allocated = totals.bytes_allocated.wrapping_add(bytes);
        });
    }

    fn record_deallocation(&self) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        record_on_thread(|totals| {
            totals.deallocations = totals.deallocations.wrapping_add(1);
        });
    }
}

thread_local! {
    static THREAD_TOTALS: Cell<AllocStats> = const {
        Cell::new(AllocStats {
            allocations: 0,
            reallocations: 0,
            deallocations: 0,
            bytes_allocated: 0,
        })
    };
}

fn record_on_thread(update: impl FnOnce(&mut AllocStats)) {
    let _ = THREAD_TOTALS.try_with(|cell| {
        let mut totals = cell.get();
        update(&mut totals);
        cell.set(totals);
    });
}

// SAFETY: every method forwards its arguments unchanged to `inner`, which
// upholds the `GlobalAlloc` contract, and returns its result unchanged. The
// counter updates neither allocate nor touch the memory being managed: the
// per-thread totals are a const-initialized `Cell` with no destructor, so
// reading or writing them never allocates.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CountingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.record_allocation(layout.size());
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract for `layout`.
        unsafe { self.inner.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.record_allocation(layout.size());
        // SAFETY: the caller upholds `GlobalAlloc::alloc_zeroed`'s contract for `layout`.
        unsafe { self.inner.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.record_reallocation(new_size);
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract for
        // `ptr`, `layout` and `new_size`.
        unsafe { self.inner.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.record_deallocation();
        // SAFETY: the caller upholds `GlobalAlloc::dealloc`'s contract for `ptr` and `layout`.
        unsafe { self.inner.dealloc(ptr, layout) }
    }
}

#[cfg(test)]
mod tests {
    use super::AllocStats;

    #[test]
    fn since_subtracts_each_counter() {
        let earlier = AllocStats {
            allocations: 1,
            reallocations: 2,
            deallocations: 3,
            bytes_allocated: 40,
        };
        let later = AllocStats {
            allocations: 4,
            reallocations: 2,
            deallocations: 5,
            bytes_allocated: 100,
        };
        let expected = AllocStats {
            allocations: 3,
            reallocations: 0,
            deallocations: 2,
            bytes_allocated: 60,
        };
        assert_eq!(later.since(earlier), expected);
    }
}
