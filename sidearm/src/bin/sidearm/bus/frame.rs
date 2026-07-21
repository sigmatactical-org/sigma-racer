//! A received safety-bus CAN frame.

/// A received classic/FD CAN frame (up to 8 bytes of the payload used here).
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub id: u32,
    data: [u8; 8],
    len: usize,
}

impl Frame {
    /// Frame from raw id + payload (truncated to 8 bytes).
    pub fn new(id: u32, data: &[u8]) -> Self {
        let mut buf = [0u8; 8];
        let len = data.len().min(8);
        buf[..len].copy_from_slice(&data[..len]);
        Self { id, data: buf, len }
    }

    /// The valid payload bytes for this frame.
    pub fn payload(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
