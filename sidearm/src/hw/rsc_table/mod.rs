//! OpenAMP resource table for Linux `remoteproc` + virtio RPMsg.
//!
//! Layout follows Zephyr `nxp_resource_table.c` for imx8mp DDR firmware.

mod imx_vendor;
mod resource_hdr;
mod sync_table;
mod table;
mod vdev;
mod vring;

use core::cell::UnsafeCell;
use core::mem::size_of;
use core::ptr::addr_of;
use imx_vendor::FwRscImxVendor;
use resource_hdr::ResourceHdr;
use sync_table::SyncTable;
use table::NxpResourceTable;
use vdev::FwRscVdev;
use vring::FwRscVring;

const RSC_VENDOR_START: u32 = 128;
const RSC_VDEV: u32 = 3;
const VIRTIO_ID_RPMSG: u32 = 7;
const ADDR_ANY: u32 = 0xFFFF_FFFF;

const OFF_IMX: u32 = (size_of::<ResourceHdr>() + size_of::<[u32; 2]>()) as u32;
const OFF_VDEV: u32 = OFF_IMX + size_of::<FwRscImxVendor>() as u32;

/// Linux parses this blob from the `.resource_table` ELF section.
#[unsafe(link_section = ".resource_table")]
#[used]
static RESOURCE_TABLE: SyncTable = SyncTable(NxpResourceTable {
    hdr: ResourceHdr {
        ver: 1,
        num: 2,
        reserved: [0, 0],
    },
    offset: [OFF_IMX, OFF_VDEV],
    imx_vs: FwRscImxVendor {
        typ: RSC_VENDOR_START,
        len: size_of::<FwRscImxVendor>() as u32,
        magic_num: 0x6E78_7073, // 'nxps'
        version: 0,
        features: 0x1,
    },
    vdev: FwRscVdev {
        typ: RSC_VDEV,
        id: VIRTIO_ID_RPMSG,
        notifyid: 0,
        dfeatures: 1,
        gfeatures: 0,
        config_len: 0,
        status: UnsafeCell::new(0),
        num_of_vrings: 2,
        reserved: [0, 0],
    },
    vring0: FwRscVring {
        da: ADDR_ANY,
        align: 16,
        num: 8,
        notifyid: 0,
        reserved: 0,
    },
    vring1: FwRscVring {
        da: ADDR_ANY,
        align: 16,
        num: 8,
        notifyid: 1,
        reserved: 0,
    },
});

/// VirtIO config status: driver OK.
pub const VIRTIO_CONFIG_STATUS_DRIVER_OK: u8 = 0x04;

/// Mutable pointer to the virtio status byte inside the resource table.
///
/// Sound because `status` is an `UnsafeCell`, so writing through this pointer is
/// permitted; the byte's alignment is 1, so no misaligned access.
pub fn vdev_status_ptr() -> *mut u8 {
    RESOURCE_TABLE.0.vdev.status.get()
}

/// Host-patched TX vring device address (Linux fills `ADDR_ANY` at load).
pub fn vring0_da() -> u32 {
    // `addr_of!` avoids forming a reference to the packed (possibly-misaligned)
    // field; `read_unaligned` copes with the alignment. The `UnsafeCell` in the
    // table keeps the compiler from assuming the host-patched value is constant.
    unsafe { addr_of!(RESOURCE_TABLE.0.vring0.da).read_unaligned() }
}

/// Host-patched RX vring device address.
pub fn vring1_da() -> u32 {
    unsafe { addr_of!(RESOURCE_TABLE.0.vring1.da).read_unaligned() }
}
