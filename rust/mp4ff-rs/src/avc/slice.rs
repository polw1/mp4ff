use super::{NaluType, Sps, Pps};
use crate::bits::reader::BitReader;
use std::collections::HashMap;
use std::io::{self, Cursor, Read};

/// Reader for EBSP bitstreams dropping start code emulation prevention bytes.
#[derive(Debug)]
struct EbspReader<R: Read> {
    rd: R,
    err: Option<io::Error>,
    n: u32,
    value: u64,
    pos: i64,
    zero_count: u8,
}

impl<R: Read> EbspReader<R> {
    fn new(rd: R) -> Self { Self { rd, err: None, n: 0, value: 0, pos: -1, zero_count: 0 } }
    fn acc_error(&self) -> Option<&io::Error> { self.err.as_ref() }
    fn nr_bytes_read(&self) -> i64 { self.pos + 1 }

    fn read(&mut self, n: u32) -> u32 {
        if self.err.is_some() { return 0; }
        while self.n < n {
            let mut buf = [0u8; 1];
            if let Err(e) = self.rd.read_exact(&mut buf) {
                self.err = Some(e); return 0;
            }
            let mut b = buf[0];
            self.pos += 1;
            if self.zero_count == 2 && b == 0x03 {
                if let Err(e) = self.rd.read_exact(&mut buf) {
                    self.err = Some(e); return 0;
                }
                b = buf[0];
                self.pos += 1;
                self.zero_count = 0;
            }
            if b == 0 { self.zero_count += 1; } else { self.zero_count = 0; }
            self.value = (self.value << 8) | b as u64;
            self.n += 8;
        }
        let v = (self.value >> (self.n - n)) as u32;
        self.n -= n;
        self.value &= (1u64 << self.n) - 1;
        v
    }

    fn read_flag(&mut self) -> bool { self.read(1) == 1 }
    fn read_signed(&mut self, n: u32) -> i32 {
        let v = self.read(n);
        if n == 0 { return 0; }
        let first = v >> (n - 1);
        if first == 1 { (v as i32) | (!0 << n) } else { v as i32 }
    }
}

/// Slice types as defined in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceType {
    P = 0,
    B = 1,
    I = 2,
    SP = 3,
    SI = 4,
}

impl std::fmt::Display for SliceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SliceType::P => "P",
            SliceType::B => "B",
            SliceType::I => "I",
            SliceType::SP => "SP",
            SliceType::SI => "SI",
        };
        f.write_str(s)
    }
}

/// Parsed AVC slice header with a limited set of fields.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SliceHeader {
    pub slice_type: u32,
    pub first_mb_in_slice: u32,
    pub pic_param_id: u32,
    pub seq_param_id: u32,
    pub color_plane_id: u32,
    pub frame_num: u32,
    pub idr_pic_id: u32,
    pub pic_order_cnt_lsb: u32,
    pub delta_pic_order_cnt_bottom: i32,
    pub delta_pic_order_cnt: [i32; 2],
    pub redundant_pic_cnt: u32,
    pub num_ref_idx_l0_active_minus1: u32,
    pub num_ref_idx_l1_active_minus1: u32,
    pub modification_of_pic_nums_idc: u32,
    pub abs_diff_pic_num_minus1: u32,
    pub long_term_pic_num: u32,
    pub abs_diff_view_idx_minus1: u32,
    pub luma_log2_weight_denom: u32,
    pub chroma_log2_weight_denom: u32,
    pub difference_of_pic_nums_minus1: u32,
    pub long_term_fram_idx: u32,
    pub max_long_term_frame_idx_plus1: u32,
    pub cabac_init_idc: u32,
    pub slice_qp_delta: i32,
    pub slice_qs_delta: i32,
    pub disable_deblocking_filter_idc: u32,
    pub slice_alpha_c0_offset_div2: i32,
    pub slice_beta_offset_div2: i32,
    pub slice_group_change_cycle: u32,
    pub size: u32,
    pub field_pic_flag: bool,
    pub bottom_field_flag: bool,
    pub direct_spatial_mv_pred_flag: bool,
    pub num_ref_idx_active_override_flag: bool,
    pub ref_pic_list_modification_l0_flag: bool,
    pub ref_pic_list_modification_l1_flag: bool,
    pub no_output_of_prior_pics_flag: bool,
    pub long_term_reference_flag: bool,
    pub sp_for_switch_flag: bool,
    pub adaptive_ref_pic_marking_mode_flag: bool,
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
            let _first_mb_in_slice = read_ue_br(&mut r);
            let mut st = read_ue_br(&mut r);
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

/// Parse an AVC slice header with limited validation.
pub fn parse_slice_header(
    nalu: &[u8],
    sps_map: &HashMap<u32, Sps>,
    pps_map: &HashMap<u32, Pps>,
) -> Option<SliceHeader> {
    if nalu.is_empty() { return None; }
    let ntype = NaluType::from_header_byte(nalu[0]);
    if ntype != NaluType::NonIDR && ntype != NaluType::IDR { return None; }

    let mut r = EbspReader::new(Cursor::new(nalu));
    let nal_hdr = r.read(8);
    let _nal_ref_idc = (nal_hdr >> 5) & 0x3;

    let mut sh = SliceHeader::default();
    sh.first_mb_in_slice = read_ue(&mut r);
    sh.slice_type = read_ue(&mut r);
    sh.pic_param_id = read_ue(&mut r);
    let pps = pps_map.get(&sh.pic_param_id)?;
    sh.seq_param_id = pps.seq_parameter_set_id;
    let sps = sps_map.get(&sh.seq_param_id)?;

    if sps.separate_colour_plane_flag { sh.color_plane_id = r.read(2); }
    sh.frame_num = r.read((sps.log2_max_frame_num_minus4 + 4) as u32);
    if !sps.frame_mbs_only_flag {
        sh.field_pic_flag = r.read_flag();
        if sh.field_pic_flag { sh.bottom_field_flag = r.read_flag(); }
    }
    if ntype == NaluType::IDR { sh.idr_pic_id = read_ue(&mut r); }

    if sps.pic_order_cnt_type == 0 {
        sh.pic_order_cnt_lsb = r.read((sps.log2_max_pic_order_cnt_lsb_minus4 + 4) as u32);
        if pps.bottom_field_pic_order_in_frame_present_flag && !sh.field_pic_flag {
            sh.delta_pic_order_cnt_bottom = read_se(&mut r);
        }
    } else if sps.pic_order_cnt_type == 1 && !sps.delta_pic_order_always_zero_flag {
        sh.delta_pic_order_cnt[0] = read_se(&mut r);
        if pps.bottom_field_pic_order_in_frame_present_flag && !sh.field_pic_flag {
            sh.delta_pic_order_cnt[1] = read_se(&mut r);
        }
    }

    if pps.redundant_pic_cnt_present_flag {
        sh.redundant_pic_cnt = read_ue(&mut r);
    }

    let slice_mod = sh.slice_type % 5;
    if slice_mod == SliceType::B as u32 { sh.direct_spatial_mv_pred_flag = r.read_flag(); }

    match slice_mod {
        x if x == SliceType::P as u32 || x == SliceType::SP as u32 || x == SliceType::B as u32 => {
            sh.num_ref_idx_active_override_flag = r.read_flag();
            if sh.num_ref_idx_active_override_flag {
                sh.num_ref_idx_l0_active_minus1 = read_ue(&mut r);
                if slice_mod == SliceType::B as u32 {
                    sh.num_ref_idx_l1_active_minus1 = read_ue(&mut r);
                }
            } else {
                sh.num_ref_idx_l0_active_minus1 = pps.num_ref_idx_i0_default_active_minus1;
                sh.num_ref_idx_l1_active_minus1 = pps.num_ref_idx_i1_default_active_minus1;
            }
        }
        _ => {}
    }

    if slice_mod != SliceType::I as u32 && slice_mod != SliceType::SI as u32 {
        sh.ref_pic_list_modification_l0_flag = r.read_flag();
        if sh.ref_pic_list_modification_l0_flag {
            loop {
                sh.modification_of_pic_nums_idc = read_ue(&mut r);
                match sh.modification_of_pic_nums_idc {
                    0 | 1 => sh.abs_diff_pic_num_minus1 = read_ue(&mut r),
                    2 => sh.long_term_pic_num = read_ue(&mut r),
                    4 | 5 => sh.abs_diff_view_idx_minus1 = read_ue(&mut r),
                    3 => break,
                    _ => {}
                }
                if r.acc_error().is_some() { break; }
            }
        }
    }

    if slice_mod == SliceType::B as u32 {
        sh.ref_pic_list_modification_l1_flag = r.read_flag();
        if sh.ref_pic_list_modification_l1_flag {
            loop {
                sh.modification_of_pic_nums_idc = read_ue(&mut r);
                match sh.modification_of_pic_nums_idc {
                    0 | 1 => sh.abs_diff_pic_num_minus1 = read_ue(&mut r),
                    2 => sh.long_term_pic_num = read_ue(&mut r),
                    4 | 5 => sh.abs_diff_view_idx_minus1 = read_ue(&mut r),
                    3 => break,
                    _ => {}
                }
                if r.acc_error().is_some() { break; }
            }
        }
    }

    if pps.weighted_pred_flag && (slice_mod == SliceType::P as u32 || slice_mod == SliceType::SP as u32) ||
       (pps.weighted_bipred_idc == 1 && slice_mod == SliceType::B as u32) {
        sh.luma_log2_weight_denom = read_ue(&mut r);
        if sps.chroma_array_type() != 0 {
            sh.chroma_log2_weight_denom = read_ue(&mut r);
        }
        for _ in 0..=sh.num_ref_idx_l0_active_minus1 {
            let luma_weight_l0_flag = r.read_flag();
            if luma_weight_l0_flag {
                let _ = read_se(&mut r); // luma_weight_l0
                let _ = read_se(&mut r); // luma_offset_l0
            }
            if sps.chroma_array_type() != 0 {
                let chroma_weight_l0_flag = r.read_flag();
                if chroma_weight_l0_flag {
                    for _ in 0..2 { let _ = read_se(&mut r); let _ = read_se(&mut r); }
                }
            }
        }
        if slice_mod == SliceType::B as u32 {
            for _ in 0..=sh.num_ref_idx_l1_active_minus1 {
                let luma_weight_l1_flag = r.read_flag();
                if luma_weight_l1_flag {
                    let _ = read_se(&mut r); let _ = read_se(&mut r);
                }
                if sps.chroma_array_type() != 0 {
                    let chroma_weight_l1_flag = r.read_flag();
                    if chroma_weight_l1_flag {
                        for _ in 0..2 { let _ = read_se(&mut r); let _ = read_se(&mut r); }
                    }
                }
            }
        }
    }

    if _nal_ref_idc != 0 {
        if ntype == NaluType::IDR {
            sh.no_output_of_prior_pics_flag = r.read_flag();
            sh.long_term_reference_flag = r.read_flag();
        } else {
            sh.adaptive_ref_pic_marking_mode_flag = r.read_flag();
            if sh.adaptive_ref_pic_marking_mode_flag {
                loop {
                    let mmco = read_ue(&mut r);
                    match mmco {
                        1 | 3 => sh.difference_of_pic_nums_minus1 = read_ue(&mut r),
                        2 => sh.long_term_pic_num = read_ue(&mut r),
                        _ => {}
                    }
                    match mmco {
                        3 | 6 => sh.long_term_fram_idx = read_ue(&mut r),
                        4 => sh.max_long_term_frame_idx_plus1 = read_ue(&mut r),
                        0 => break,
                        _ => {}
                    }
                    if r.acc_error().is_some() { break; }
                }
            }
        }
    }

    if pps.entropy_coding_mode_flag && slice_mod != SliceType::I as u32 && slice_mod != SliceType::SI as u32 {
        sh.cabac_init_idc = read_ue(&mut r);
    }
    sh.slice_qp_delta = read_se(&mut r);
    if slice_mod == SliceType::SP as u32 || slice_mod == SliceType::SI as u32 {
        if slice_mod == SliceType::SP as u32 { sh.sp_for_switch_flag = r.read_flag(); }
        sh.slice_qs_delta = read_se(&mut r);
    }
    if pps.deblocking_filter_control_present_flag {
        sh.disable_deblocking_filter_idc = read_ue(&mut r);
        if sh.disable_deblocking_filter_idc != 1 {
            sh.slice_alpha_c0_offset_div2 = read_se(&mut r);
            sh.slice_beta_offset_div2 = read_se(&mut r);
        }
    }
    if pps.num_slice_groups_minus1 > 0 && pps.slice_group_map_type >= 3 && pps.slice_group_map_type <= 5 {
        let pic_size_in_map_units = pps.pic_size_in_map_units_minus1 + 1;
        let slice_group_change_rate = pps.slice_group_change_rate_minus1 + 1;
        let v = pic_size_in_map_units / slice_group_change_rate + 1;
        let nr_bits = ceil_log2(v);
        sh.slice_group_change_cycle = r.read(nr_bits);
    }

    sh.size = r.nr_bytes_read() as u32;
    if r.acc_error().is_some() { None } else { Some(sh) }
}

fn read_ue<R: Read>(r: &mut EbspReader<R>) -> u32 {
    let mut leading = 0u32;
    while r.read(1) == 0 {
        if r.acc_error().is_some() { return 0; }
        leading += 1;
    }
    let prefix = (1u32 << leading) - 1;
    let suffix = if leading > 0 { r.read(leading) } else { 0 };
    prefix + suffix
}

fn read_ue_br<R: std::io::Read>(r: &mut BitReader<R>) -> u32 {
    let mut leading = 0u32;
    while r.read(1) == 0 {
        if r.acc_error().is_some() { return 0; }
        leading += 1;
    }
    let prefix = (1u32 << leading) - 1;
    let suffix = if leading > 0 { r.read(leading) } else { 0 };
    prefix + suffix
}

fn read_se<R: Read>(r: &mut EbspReader<R>) -> i32 {
    let ue = read_ue(r) as i32;
    if ue % 2 == 1 { (ue + 1) / 2 } else { -(ue / 2) }
}

fn ceil_log2(mut v: u32) -> u32 {
    if v <= 1 { return 0; }
    v -= 1;
    32 - v.leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn decode_hex(s: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            let hi = (bytes[i] as char).to_digit(16).unwrap();
            let lo = (bytes[i + 1] as char).to_digit(16).unwrap();
            out.push(((hi << 4) | lo) as u8);
            i += 2;
        }
        out
    }

    #[test]
    fn test_slice_type_parser() {
        let data = decode_hex("25888040ffde08e47a7bff05ab");
        let st = get_slice_type_from_nalu(&data).unwrap();
        assert_eq!(st, SliceType::I);
    }

    #[test]
    fn test_slice_type_strings() {
        assert_eq!(SliceType::P.to_string(), "P");
        assert_eq!(SliceType::B.to_string(), "B");
        assert_eq!(SliceType::I.to_string(), "I");
        assert_eq!(SliceType::SP.to_string(), "SP");
        assert_eq!(SliceType::SI.to_string(), "SI");
    }

    fn load_nalus(data: &[u8]) -> (HashMap<u32, Sps>, HashMap<u32, Pps>, Vec<Vec<u8>>) {
        let nalus = crate::avc::annexb::extract_nalus_from_bytestream(data);
        let mut sps_map = HashMap::new();
        let mut pps_map = HashMap::new();
        let mut video = Vec::new();
        for nalu in &nalus {
            match NaluType::from_header_byte(nalu[0]) {
                NaluType::SPS => {
                    if let Some(s) = crate::avc::sps::parse_sps_nalu(nalu) {
                        sps_map.insert(s.parameter_set_id, s);
                    }
                }
                NaluType::PPS => {
                    if let Some(p) = crate::avc::pps::parse_pps_nalu(nalu) {
                        pps_map.insert(p.pic_parameter_set_id, p);
                    }
                }
                t if t.is_video() => video.push(nalu.clone()),
                _ => {}
            }
        }
        (sps_map, pps_map, video)
    }

    #[test]
    fn test_parse_slice_header_blackframe() {
        const DATA: &[u8] = include_bytes!("../../../../avc/testdata/blackframe.264");
        let (sps_map, pps_map, nalus) = load_nalus(DATA);
        for nalu in nalus {
            if NaluType::from_header_byte(nalu[0]) == NaluType::IDR {
                let sh = parse_slice_header(&nalu, &sps_map, &pps_map).unwrap();
                assert_eq!(sh.slice_type, 7);
                assert_eq!(sh.slice_qp_delta, 6);
                assert_eq!(sh.slice_alpha_c0_offset_div2, -3);
                assert_eq!(sh.slice_beta_offset_div2, -3);
                assert_eq!(sh.size, 7);
            }
        }
    }

    #[test]
    fn test_parse_slice_header_two_frames() {
        const DATA: &[u8] = include_bytes!("../../../../avc/testdata/two-frames.264");
        let (sps_map, pps_map, nalus) = load_nalus(DATA);
        let mut idr_done = false;
        for nalu in nalus {
            match NaluType::from_header_byte(nalu[0]) {
                NaluType::IDR => {
                    let sh = parse_slice_header(&nalu, &sps_map, &pps_map).unwrap();
                    assert_eq!(sh.slice_type, SliceType::I as u32);
                    assert_eq!(sh.idr_pic_id, 1);
                    assert_eq!(sh.slice_qp_delta, 8);
                    assert_eq!(sh.size, 5);
                    idr_done = true;
                }
                NaluType::NonIDR => {
                    if idr_done {
                        let sh = parse_slice_header(&nalu, &sps_map, &pps_map).unwrap();
                        assert_eq!(sh.slice_type, SliceType::P as u32);
                        assert_eq!(sh.frame_num, 1);
                        assert_eq!(sh.modification_of_pic_nums_idc, 3);
                        assert_eq!(sh.slice_qp_delta, 13);
                        assert!(sh.num_ref_idx_active_override_flag);
                        assert!(sh.ref_pic_list_modification_l0_flag);
                        assert_eq!(sh.size, 5);
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_slice_header_length() {
        let sps_hex = "6764001eacd940a02ff9610000030001000003003c8f162d96";
        let pps_hex = "68ebecb22c";
        let nalu_hex = "419a6649e10f2653022fff8700000302c8a32d32";
        let sps = crate::avc::sps::parse_sps_nalu(&decode_hex(sps_hex)).unwrap();
        let mut sps_map = HashMap::new();
        sps_map.insert(sps.parameter_set_id, sps);
        let pps = crate::avc::pps::parse_pps_nalu(&decode_hex(pps_hex)).unwrap();
        let mut pps_map = HashMap::new();
        pps_map.insert(pps.pic_parameter_set_id, pps);
        let nalu = decode_hex(nalu_hex);
        let sh = parse_slice_header(&nalu, &sps_map, &pps_map).unwrap();
        assert_eq!(sh.size, 11);
    }
}
