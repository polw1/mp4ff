use super::NaluType;

/// Convert a bytestream with Annex B start codes to a sample using 4-byte lengths.
/// The conversion is performed in a new buffer which is returned.
pub fn convert_bytestream_to_nalu_sample(stream: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(stream.len());
    let mut pos = 0usize;
    while pos + 3 <= stream.len() {
        if pos + 3 < stream.len() && &stream[pos..pos+3] == [0,0,1] {
            pos += 3;
        } else if pos + 4 < stream.len() && &stream[pos..pos+4] == [0,0,0,1] {
            pos += 4;
        } else {
            pos += 1;
            continue;
        }
        let start = pos;
        while pos + 3 <= stream.len() && &stream[pos..pos+3] != [0,0,1] && (pos + 4 > stream.len() || &stream[pos..pos+4] != [0,0,0,1]) {
            pos += 1;
        }
        let end = pos;
        let len = (end - start) as u32;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&stream[start..end]);
    }
    out
}

/// Replace 4-byte lengths in a sample with start codes (Annex B).
pub fn convert_sample_to_bytestream(sample: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(sample.len() + sample.len() / 16);
    let mut pos = 0usize;
    while pos + 4 <= sample.len() {
        let len = u32::from_be_bytes([sample[pos], sample[pos+1], sample[pos+2], sample[pos+3]]) as usize;
        pos += 4;
        if pos + len > sample.len() { break; }
        out.extend_from_slice(&[0,0,0,1]);
        out.extend_from_slice(&sample[pos..pos+len]);
        pos += len;
    }
    out
}

/// Extract the first video NAL unit from a bytestream.
pub fn get_first_video_nalu_from_bytestream(stream: &[u8]) -> Option<&[u8]> {
    let mut pos = 0usize;
    while pos + 3 <= stream.len() {
        let start_code_len = if pos + 4 < stream.len() && &stream[pos..pos+4] == [0,0,0,1] {
            4
        } else if &stream[pos..pos+3] == [0,0,1] {
            3
        } else {
            pos += 1;
            continue;
        };
        pos += start_code_len;
        let start = pos;
        while pos + 3 <= stream.len() && &stream[pos..pos+3] != [0,0,1] && (pos + 4 > stream.len() || &stream[pos..pos+4] != [0,0,0,1]) {
            pos += 1;
        }
        let end = pos;
        let nalu = &stream[start..end];
        if !nalu.is_empty() && NaluType::from_header_byte(nalu[0]).is_video() {
            return Some(nalu);
        }
    }
    None
}
