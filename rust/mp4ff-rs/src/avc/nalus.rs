
use super::NaluType;

/// Extract NAL units from a sample with 4-byte lengths.
/// Returns a vector of slices into the original sample.
/// If the sample is malformed, `None` is returned.
pub fn get_nalus_from_sample(sample: &[u8]) -> Option<Vec<&[u8]>> {
    if sample.len() < 4 {
        return None;
    }
    let mut pos = 0usize;
    let mut nalus = Vec::new();
    while pos + 4 <= sample.len() {
        let len = u32::from_be_bytes([sample[pos], sample[pos+1], sample[pos+2], sample[pos+3]]) as usize;
        pos += 4;
        if pos + len > sample.len() {
            return None;
        }
        nalus.push(&sample[pos..pos+len]);
        pos += len;
    }
    Some(nalus)
}

/// Display helper for NAL unit lists used in tests and debugging.
pub fn dump_nalu_types(sample: &[u8]) -> String {
    match get_nalus_from_sample(sample) {
        Some(list) => list
            .iter()
            .map(|n| format!("{:?}", NaluType::from_header_byte(n[0])))
            .collect::<Vec<_>>()
            .join(","),
        None => "<invalid>".to_string(),
    }
}
