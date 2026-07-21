//! `Sync` wrapper allowing the table to live in a `static`.

use super::table::NxpResourceTable;

/// Newtype so the table (holding an `UnsafeCell`) can live in a `static`.
/// The memory is shared with Linux and accessed only through volatile/unaligned
/// raw ops, so asserting `Sync` is sound.
pub(super) struct SyncTable(pub(super) NxpResourceTable);

// SAFETY: access is via raw volatile/unaligned pointers; no `&mut` aliasing.
unsafe impl Sync for SyncTable {}
