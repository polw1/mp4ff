use crate::bits::reader::BitReader;
use std::io::Cursor;

use super::NaluType;

/// Scaling list with either 4x4 or 8x8 entries.
pub type ScalingList = Vec<i32>;

/// Parsed information from a PPS NAL unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pps {
    pub pic_parameter_set_id: u32,
    pub seq_parameter_set_id: u32,
    pub entropy_coding_mode_flag: bool,
    pub bottom_field_pic_order_in_frame_present_flag: bool,
    pub num_slice_groups_minus1: u32,
    pub slice_group_map_type: u32,
    pub run_length_minus1: Vec<u32>,
    pub top_left: Vec<u32>,
    pub bottom_right: Vec<u32>,
    pub slice_group_change_direction_flag: bool,
    pub slice_group_change_rate_minus1: u32,
    pub pic_size_in_map_units_minus1: u32,
    pub slice_group_id: Vec<u32>,
    pub num_ref_idx_i0_default_active_minus1: u32,
    pub num_ref_idx_i1_default_active_minus1: u32,
    pub weighted_pred_flag: bool,
    pub weighted_bipred_idc: u32,
    pub pic_init_qp_minus26: i32,
    pub pic_init_qs_minus26: i32,
    pub chroma_qp_index_offset: i32,
    pub deblocking_filter_control_present_flag: bool,
    pub constrained_intra_pred_flag: bool,
    pub redundant_pic_cnt_present_flag: bool,
    pub transform8x8_mode_flag: bool,
    pub pic_scaling_matrix_present_flag: bool,
    pub pic_scaling_lists: Vec<Option<ScalingList>>,
    pub second_chroma_qp_index_offset: i32,
}

/// Parse a PPS NAL unit including the NAL header.
pub fn parse_pps_nalu(nalu: &[u8]) -> Option<Pps> {
    if nalu.is_empty() || NaluType::from_header_byte(nalu[0]) != NaluType::PPS {
        return None;
    }

    let bytes = remove_emulation_prevention_bytes(&nalu[1..]);
    let total_bits = bytes.len() * 8;
    let mut r = BitReader::new(Cursor::new(bytes));

    let pic_parameter_set_id = read_ue(&mut r);
    let seq_parameter_set_id = read_ue(&mut r);
    let entropy_coding_mode_flag = r.read_flag();
    let bottom_field_pic_order_in_frame_present_flag = r.read_flag();
    let num_slice_groups_minus1 = read_ue(&mut r);

    let mut slice_group_map_type = 0u32;
    let mut run_length_minus1 = Vec::new();
    let mut top_left = Vec::new();
    let mut bottom_right = Vec::new();
    let mut slice_group_change_direction_flag = false;
    let mut slice_group_change_rate_minus1 = 0u32;
    let mut pic_size_in_map_units_minus1 = 0u32;
    let mut slice_group_id = Vec::new();

    if num_slice_groups_minus1 > 0 {
        slice_group_map_type = read_ue(&mut r);
        match slice_group_map_type {
            0 => {
                for _ in 0..=num_slice_groups_minus1 {
                    run_length_minus1.push(read_ue(&mut r));
                }
            }
            2 => {
                for _ in 0..=num_slice_groups_minus1 {
                    top_left.push(read_ue(&mut r));
                    bottom_right.push(read_ue(&mut r));
                }
            }
            3 | 4 | 5 => {
                slice_group_change_direction_flag = r.read_flag();
                slice_group_change_rate_minus1 = read_ue(&mut r);
            }
            6 => {
                let nr_bits = ceil_log2(num_slice_groups_minus1 + 1);
                for _ in 0..=num_slice_groups_minus1 {
                    slice_group_id.push(r.read(nr_bits));
                }
            }
            _ => {}
        }
    }

    let num_ref_idx_i0_default_active_minus1 = read_ue(&mut r);
    let num_ref_idx_i1_default_active_minus1 = read_ue(&mut r);
    let weighted_pred_flag = r.read_flag();
    let weighted_bipred_idc = r.read(2);
    let pic_init_qp_minus26 = read_se(&mut r);
    let pic_init_qs_minus26 = read_se(&mut r);
    let chroma_qp_index_offset = read_se(&mut r);
    let deblocking_filter_control_present_flag = r.read_flag();
    let constrained_intra_pred_flag = r.read_flag();
    let redundant_pic_cnt_present_flag = r.read_flag();

    let mut transform8x8_mode_flag = false;
    let mut pic_scaling_matrix_present_flag = false;
    let mut pic_scaling_lists = Vec::new();
    let mut second_chroma_qp_index_offset = 0;

    if (r.nr_bits_read() as usize) < total_bits {
        transform8x8_mode_flag = r.read_flag();
        if (r.nr_bits_read() as usize) < total_bits {
            pic_scaling_matrix_present_flag = r.read_flag();
            if pic_scaling_matrix_present_flag {
                let mut nr_scaling_lists = 6;
                if transform8x8_mode_flag {
                    nr_scaling_lists += 2; // assume chroma_format_idc != 3
                }
                for i in 0..nr_scaling_lists {
                    let present = r.read_flag();
                    if present {
                        let size = if i < 6 { 16 } else { 64 };
                        pic_scaling_lists.push(Some(read_scaling_list(&mut r, size)));
                    } else {
                        pic_scaling_lists.push(None);
                    }
                }
            }
            if (r.nr_bits_read() as usize) < total_bits {
                second_chroma_qp_index_offset = read_se(&mut r);
            }
        }
    }

    Some(Pps {
        pic_parameter_set_id,
        seq_parameter_set_id,
        entropy_coding_mode_flag,
        bottom_field_pic_order_in_frame_present_flag,
        num_slice_groups_minus1,
        slice_group_map_type,
        run_length_minus1,
        top_left,
        bottom_right,
        slice_group_change_direction_flag,
        slice_group_change_rate_minus1,
        pic_size_in_map_units_minus1,
        slice_group_id,
        num_ref_idx_i0_default_active_minus1,
        num_ref_idx_i1_default_active_minus1,
        weighted_pred_flag,
        weighted_bipred_idc,
        pic_init_qp_minus26,
        pic_init_qs_minus26,
        chroma_qp_index_offset,
        deblocking_filter_control_present_flag,
        constrained_intra_pred_flag,
        redundant_pic_cnt_present_flag,
        transform8x8_mode_flag,
        pic_scaling_matrix_present_flag,
        pic_scaling_lists,
        second_chroma_qp_index_offset,
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

fn read_se<R: std::io::Read>(r: &mut BitReader<R>) -> i32 {
    let ue = read_ue(r) as i32;
    if ue % 2 == 1 { (ue + 1) / 2 } else { -(ue / 2) }
}

fn read_scaling_list<R: std::io::Read>(r: &mut BitReader<R>, size: usize) -> ScalingList {
    let mut list = Vec::with_capacity(size);
    let mut last_scale = 8i32;
    let mut next_scale = 8i32;
    for _ in 0..size {
        if next_scale != 0 {
            let delta = read_se(r);
            next_scale = (last_scale + delta + 256) % 256;
        }
        if next_scale == 0 { list.push(last_scale); } else { list.push(next_scale); }
        last_scale = *list.last().unwrap();
    }
    list
}

fn ceil_log2(mut v: u32) -> u32 {
    if v <= 1 { return 0; }
    v -= 1;
    32 - v.leading_zeros()
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
