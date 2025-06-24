use super::{NaluType};
use crate::bits::reader::BitReader;
use std::io::Cursor;

/// Slice types as defined in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceType {
    P = 0,
    B = 1,
    I = 2,
    SP = 3,
    SI = 4,
}

/// Return the slice type (0-4) from a NAL unit containing a slice header.
/// Only limited validation is performed.
pub fn get_slice_type_from_nalu(nalu: &[u8]) -> Option<SliceType> {
    if nalu.len() <= 1 {
        return None;
    }
    let ntype = NaluType::from_header_byte(nalu[0]);
    match ntype {
        NaluType::NonIDR | NaluType::IDR => {
            let mut r = BitReader::new(Cursor::new(&nalu[1..]));
            let _first_mb_in_slice = read_ue(&mut r);
            let mut st = read_ue(&mut r);
            if st >= 5 { st -= 5; }
            match st {
                0 => Some(SliceType::P),
                1 => Some(SliceType::B),
                2 => Some(SliceType::I),
                3 => Some(SliceType::SP),
                4 => Some(SliceType::SI),
                _ => None,
            }
        }
        _ => None,
    }
}

fn read_ue<R: std::io::Read>(r: &mut BitReader<R>) -> u32 {
    let mut leading = 0u32;
    while r.read(1) == 0 {
        if r.acc_error().is_some() { return 0; }
        leading += 1;
    }
    let prefix = (1u32 << leading) - 1;
    let suffix = if leading > 0 { r.read(leading) } else { 0 };
    prefix + suffix
}
