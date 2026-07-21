//! Boot-time heap shared by all firmware binaries.
//!
//! Exists for the `alloc`-mode CAN dictionary (`dbc-rs`): the DBC parse
//! allocates once at startup; encode/decode are allocation-free. Nothing in
//! the control path allocates.

/// Global allocator for `no_std` firmware builds.
#[global_allocator]
static HEAP: embedded_alloc::LlffHeap = embedded_alloc::LlffHeap::empty();

/// Initialize the heap. Call exactly once, before any allocation.
#[allow(unsafe_code)]
pub fn init() {
    use core::mem::MaybeUninit;
    const HEAP_BYTES: usize = 32 * 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_BYTES] = [MaybeUninit::uninit(); HEAP_BYTES];
    // SAFETY: single call site per binary, before the executor starts.
    unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_BYTES) }
}
