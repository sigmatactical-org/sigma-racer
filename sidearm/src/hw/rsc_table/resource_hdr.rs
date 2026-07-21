//! Resource-table header (version + entry count).

/// Leading header of the resource table blob.
#[repr(C, packed)]
pub(super) struct ResourceHdr {
    pub(super) ver: u32,
    pub(super) num: u32,
    pub(super) reserved: [u32; 2],
}
