//! Complete NXP resource-table layout.

use super::imx_vendor::FwRscImxVendor;
use super::resource_hdr::ResourceHdr;
use super::vdev::FwRscVdev;
use super::vring::FwRscVring;

/// The full table as Linux `remoteproc` parses it (header, offsets, entries).
#[repr(C, packed)]
pub(super) struct NxpResourceTable {
    pub(super) hdr: ResourceHdr,
    pub(super) offset: [u32; 2],
    pub(super) imx_vs: FwRscImxVendor,
    pub(super) vdev: FwRscVdev,
    pub(super) vring0: FwRscVring,
    pub(super) vring1: FwRscVring,
}
