// SPDX-License-Identifier: MIT OR Apache-2.0
//! A counting global allocator for zero-allocation tests.
//!
//! This is the only `unsafe` code in the repository. It is test-only (this crate is never
//! published and no library crate depends on it) and forwards every call unchanged to the
//! system allocator.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// Counts allocations made on the current thread, then defers to [`System`].
///
/// Install with `#[global_allocator] static A: CountingAlloc = CountingAlloc;` in a test
/// binary and use [`measure`] around the code under test.
#[derive(Debug)]
pub struct CountingAlloc;

// SAFETY: every method forwards its arguments unchanged to `System`, which upholds the
// `GlobalAlloc` contract; the thread-local counters are `Cell`s with const initialisers
// (no allocation, no destructor), so counting cannot recurse into the allocator.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get().wrapping_add(1)));
        let _ = BYTES.try_with(|b| b.set(b.get().wrapping_add(layout.size())));
        // SAFETY: same layout, same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get().wrapping_add(1)));
        let _ = BYTES.try_with(|b| b.set(b.get().wrapping_add(layout.size())));
        // SAFETY: same layout, same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` come from a previous call on this allocator, which
        // forwarded to `System`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = COUNT.try_with(|c| c.set(c.get().wrapping_add(1)));
        let _ = BYTES.try_with(|b| b.set(b.get().wrapping_add(new_size)));
        // SAFETY: arguments forwarded unchanged; contract is the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Allocation count and bytes requested while a closure ran on the current thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    /// Number of `alloc`, `alloc_zeroed` and `realloc` calls.
    pub allocations: usize,
    /// Total bytes requested.
    pub bytes: usize,
}

/// Runs `f` and reports the allocations it performed on this thread.
pub fn measure<R>(f: impl FnOnce() -> R) -> (R, Usage) {
    let c0 = COUNT.with(Cell::get);
    let b0 = BYTES.with(Cell::get);
    let r = f();
    let c1 = COUNT.with(Cell::get);
    let b1 = BYTES.with(Cell::get);
    (
        r,
        Usage {
            allocations: c1.wrapping_sub(c0),
            bytes: b1.wrapping_sub(b0),
        },
    )
}
