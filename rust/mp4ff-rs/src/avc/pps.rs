use crate::bits::reader::BitReader;
use std::io::Cursor;

use super::NaluType;

/// Minimal representation of a PPS with only the referenced SPS id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pps {
    pub pic_parameter_set_id: u32,
    pub seq_parameter_set_id: u32,
}

/// Parse a PPS NAL unit (including NAL header). Only the first two fields are
/// extracted for now.
pub fn parse_pps_nalu(nalu: &[u8]) -> Option<Pps> {
    if nalu.is_empty() || NaluType::from_header_byte(nalu[0]) != NaluType::PPS {
        return None;
    }
    let mut r = BitReader::new(Cursor::new(&nalu[1..]));
    let pps_id = read_ue(&mut r);
    let sps_id = read_ue(&mut r);
    Some(Pps {
        pic_parameter_set_id: pps_id,
        seq_parameter_set_id: sps_id,
    })
}

fn read_ue<R: std::io::Read>(r: &mut BitReader<R>) -> u32 {
    let mut leading = 0u32;
    while r.read(1) == 0 {
        if r.acc_error().is_some() {
            return 0;
        }
        leading += 1;
    }
    let prefix = (1u32 << leading) - 1;
    let suffix = if leading > 0 { r.read(leading) } else { 0 };
    prefix + suffix
}
