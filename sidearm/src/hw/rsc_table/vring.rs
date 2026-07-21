//! virtio ring entry.

/// One vring descriptor; Linux patches `da` (device address) at load time.
#[repr(C, packed)]
pub(super) struct FwRscVring {
    pub(super) da: u32,
    pub(super) align: u32,
    pub(super) num: u32,
    pub(super) notifyid: u32,
    pub(super) reserved: u32,
}
