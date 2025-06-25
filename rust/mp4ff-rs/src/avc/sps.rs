use std::io::Cursor;

use crate::bits::reader::BitReader;

use super::NaluType;

/// Parsed information from an SPS NAL unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sps {
    pub profile: u8,
    pub profile_compatibility: u8,
    pub level: u8,
    pub width: u16,
    pub height: u16,
}

/// Parse an SPS NAL unit (including the NAL header) and return width and height.
/// The parsing is intentionally limited and ignores most fields.
pub fn parse_sps_nalu(nalu: &[u8]) -> Option<Sps> {
    if nalu.is_empty() { return None; }
    if NaluType::from_header_byte(nalu[0]) != NaluType::SPS { return None; }
    let bytes = remove_emulation_prevention_bytes(&nalu[1..]);
    let mut r = BitReader::new(Cursor::new(bytes));
    let profile_idc = r.read(8) as u8;
    let compat = r.read(8) as u8;
    let level_idc = r.read(8) as u8;
    let _id = read_ue(&mut r);
    let mut chroma_format_idc = 1u32;
    match profile_idc {
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135 => {
            chroma_format_idc = read_ue(&mut r);
            if chroma_format_idc == 3 { r.read(1); }
            let _ = read_ue(&mut r); // bit_depth_luma_minus8
            let _ = read_ue(&mut r); // bit_depth_chroma_minus8
            r.read(1); // qpprime_y_zero_transform_bypass_flag
            if r.read(1) == 1 { // seq_scaling_matrix_present_flag
                let lists = if chroma_format_idc != 3 { 8 } else { 12 };
                for i in 0..lists { if r.read(1) == 1 { read_scaling_list(&mut r, if i < 6 { 16 } else { 64 }); } }
            }
        }
        _ => {}
    }
    let _ = read_ue(&mut r); // log2_max_frame_num_minus4
    let pic_order_cnt_type = read_ue(&mut r);
    if pic_order_cnt_type == 0 {
        let _ = read_ue(&mut r); // log2_max_pic_order_cnt_lsb_minus4
    } else if pic_order_cnt_type == 1 {
        r.read(1); // delta_pic_order_always_zero_flag
        let _ = read_se(&mut r); // offset_for_non_ref_pic
        let _ = read_se(&mut r); // offset_for_top_to_bottom_field
        let num = read_ue(&mut r);
        for _ in 0..num { let _ = read_se(&mut r); }
    }
    let _ = read_ue(&mut r); // num_ref_frames
    r.read(1); // gaps_in_frame_num_value_allowed_flag
    let width_in_mbs = read_ue(&mut r) + 1;
    let height_in_map = read_ue(&mut r) + 1;
    let frame_mbs_only_flag = r.read(1);
    let mut width = width_in_mbs * 16;
    let mut height = height_in_map * 16;
    if frame_mbs_only_flag == 0 { r.read(1); height *= 2; }
    r.read(1); // direct_8x8_inference_flag
    let cropping_flag = r.read(1);
    let mut crop_left = 0u32;
    let mut crop_right = 0u32;
    let mut crop_top = 0u32;
    let mut crop_bottom = 0u32;
    if cropping_flag == 1 {
        crop_left = read_ue(&mut r);
        crop_right = read_ue(&mut r);
        crop_top = read_ue(&mut r);
        crop_bottom = read_ue(&mut r);
    }
    let (crop_unit_x, crop_unit_y) = match chroma_format_idc {
        0 => (1, 2 - frame_mbs_only_flag as u32),
        1 => (2, 2 * (2 - frame_mbs_only_flag as u32)),
        2 => (2, 1 * (2 - frame_mbs_only_flag as u32)),
        _ => (1, 1 * (2 - frame_mbs_only_flag as u32)),
    };
    width -= (crop_left + crop_right) * crop_unit_x;
    height -= (crop_top + crop_bottom) * crop_unit_y;
    Some(Sps {
        profile: profile_idc,
        profile_compatibility: compat,
        level: level_idc,
        width: width as u16,
        height: height as u16,
    })
}

fn read_ue<R: std::io::Read>(r: &mut BitReader<R>) -> u32 {
    let mut leading = 0u32;
    while r.read(1) == 0 { if r.acc_error().is_some() { return 0; } leading += 1; }
    let prefix = (1u32 << leading) - 1;
    let suffix = if leading > 0 { r.read(leading) } else { 0 };
    prefix + suffix
}

fn read_se<R: std::io::Read>(r: &mut BitReader<R>) -> i32 {
    let ue = read_ue(r) as i32;
    if ue % 2 == 1 { (ue + 1) / 2 } else { -(ue / 2) }
}

fn read_scaling_list<R: std::io::Read>(r: &mut BitReader<R>, size: usize) {
    let mut last_scale = 8i32;
    let mut next_scale = 8i32;
    for _ in 0..size {
        if next_scale != 0 {
            let delta = read_se(r);
            next_scale = (last_scale + delta + 256) % 256;
        }
        if next_scale == 0 { last_scale = last_scale; } else { last_scale = next_scale; }
    }
}

fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut zero_count = 0u8;
    for &b in data {
        if zero_count == 2 && b == 0x03 {
            zero_count = 0;
            continue;
        }
        out.push(b);
        if b == 0 { zero_count += 1; } else { zero_count = 0; }
    }
    out
}
