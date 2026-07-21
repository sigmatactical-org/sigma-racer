//! NXP vendor resource entry.

/// i.MX vendor-specific entry (`'nxps'` magic) expected by the NXP remoteproc
/// driver ahead of the vdev entry.
#[repr(C, packed)]
pub(super) struct FwRscImxVendor {
    pub(super) typ: u32,
    pub(super) len: u32,
    pub(super) magic_num: u32,
    pub(super) version: u32,
    pub(super) features: u32,
}
