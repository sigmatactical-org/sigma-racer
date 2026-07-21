//! Frame type delivered by the FLEXCAN1 RX mailbox.

/// Classic CAN frame delivered from the RX mailbox.
#[derive(Clone, Copy, Debug)]
pub struct RxFrame {
    /// Standard 11-bit identifier.
    pub id: u32,
    /// Payload bytes (only the first `len` are valid).
    pub data: [u8; 8],
    /// Payload length in bytes (0–8).
    pub len: usize,
}
