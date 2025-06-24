use super::NaluType;

/// Parse an SEI NAL unit and return the raw payloads.
/// This is a very small subset of the functionality in the Go version.
pub fn parse_sei_nalu(nalu: &[u8]) -> Option<Vec<Vec<u8>>> {
    if nalu.is_empty() || NaluType::from_header_byte(nalu[0]) != NaluType::SEI {
        return None;
    }
    let mut pos = 1usize; // after header
    let mut payloads = Vec::new();
    while pos < nalu.len() {
        let mut typ = 0u32;
        while pos < nalu.len() {
            let b = nalu[pos];
            pos += 1;
            typ += b as u32;
            if b != 0xff { break; }
        }
        let mut len = 0u32;
        while pos < nalu.len() {
            let b = nalu[pos];
            pos += 1;
            len += b as u32;
            if b != 0xff { break; }
        }
        if pos + len as usize > nalu.len() { break; }
        payloads.push(nalu[pos..pos+len as usize].to_vec());
        pos += len as usize;
        if typ == 0 && len == 0 { break; }
    }
    Some(payloads)
}
