//! virtio device entry.

use core::cell::UnsafeCell;

/// virtio RPMsg device descriptor.
#[repr(C, packed)]
pub(super) struct FwRscVdev {
    pub(super) typ: u32,
    pub(super) id: u32,
    pub(super) notifyid: u32,
    pub(super) dfeatures: u32,
    pub(super) gfeatures: u32,
    pub(super) config_len: u32,
    // Interior-mutable: the firmware writes DRIVER_OK here and Linux DMA-reads
    // it, so this byte must not be treated as an immutable `static` (writing
    // through a `&static` cast to `*mut` is UB). `UnsafeCell<u8>` is layout-
    // identical to `u8`, so the on-wire resource-table layout is unchanged.
    pub(super) status: UnsafeCell<u8>,
    pub(super) num_of_vrings: u8,
    pub(super) reserved: [u8; 2],
}
