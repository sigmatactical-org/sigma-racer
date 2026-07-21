//! Parse `candump -L` text logs into timed frames for replay.

/// One CAN data frame with a timestamp relative to the first frame in the log.
#[derive(Debug, Clone, PartialEq)]
pub struct CandumpFrame {
    /// Seconds since the first frame in the log.
    pub at: f64,
    pub id: u32,
    pub data: Vec<u8>,
}

/// Parse `candump -L` lines: `(1730000000.050000) can1 0A0#B004525C00000000`.
/// Timestamps are rebased so the first frame is at t=0. Malformed lines are skipped.
pub fn parse(text: &str) -> Vec<CandumpFrame> {
    let mut base: Option<f64> = None;
    let mut frames = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(ts_tok), Some(_iface), Some(frame_tok)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };

        let Some(ts) = ts_tok
            .trim_start_matches('(')
            .trim_end_matches(')')
            .parse::<f64>()
            .ok()
        else {
            continue;
        };
        let Some((id_str, data_str)) = frame_tok.split_once('#') else {
            continue;
        };
        let Ok(id) = u32::from_str_radix(id_str, 16) else {
            continue;
        };
        let Some(data) = parse_hex(data_str) else {
            continue;
        };

        let base = *base.get_or_insert(ts);
        frames.push(CandumpFrame {
            at: (ts - base).max(0.0),
            id,
            data,
        });
    }

    frames
}

/// Decode an even-length hex string into at most 8 CAN payload bytes.
fn parse_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || s.len() > 16 {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let raw = s.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        let hi = (raw[i] as char).to_digit(16)?;
        let lo = (raw[i + 1] as char).to_digit(16)?;
        bytes.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rebases_candump() {
        let log = "\
(1730000000.500000) can1 0A0#B004525C00000000
(1730000000.550000) can1 120#4101107480000000
# comment
garbage line
(1730000001.000000) can1 200#7C1E950000000000
";
        let frames = parse(log);
        assert_eq!(frames.len(), 3);
        assert!((frames[0].at - 0.0).abs() < 1e-9);
        assert!((frames[1].at - 0.05).abs() < 1e-6);
        assert!((frames[2].at - 0.5).abs() < 1e-6);
        assert_eq!(frames[0].id, 0x0A0);
        assert_eq!(frames[0].data.len(), 8);
        assert_eq!(frames[0].data[0], 0xB0);
    }
}
