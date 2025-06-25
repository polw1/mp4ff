use std::io::Cursor;

use crate::bits::reader::BitReader;

use super::NaluType;

/// Extended Sample Aspect Ratio code for VUI.
const EXTENDED_SAR: u32 = 255;

/// Scaling list with either 4x4 or 8x8 entries.
pub type ScalingList = Vec<i32>;

/// Parameters for Hypothetical Reference Decoder as parsed from VUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HrdParameters {
    pub cpb_count_minus1: u32,
    pub bit_rate_scale: u32,
    pub cpb_size_scale: u32,
    pub cpb_entries: Vec<CpbEntry>,
    pub initial_cpb_removal_delay_length_minus1: u32,
    pub cpb_removal_delay_length_minus1: u32,
    pub dpb_output_delay_length_minus1: u32,
    pub time_offset_length: u32,
}

/// One entry inside [`HrdParameters`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpbEntry {
    pub bit_rate_value_minus1: u32,
    pub cpb_size_value_minus1: u32,
    pub cbr_flag: bool,
}

/// VUI parameters defined in the AVC specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VuiParameters {
    pub sample_aspect_ratio_width: u32,
    pub sample_aspect_ratio_height: u32,
    pub overscan_info_present_flag: bool,
    pub overscan_appropriate_flag: bool,
    pub video_signal_type_present_flag: bool,
    pub video_format: u32,
    pub video_full_range_flag: bool,
    pub colour_description_flag: bool,
    pub colour_primaries: u32,
    pub transfer_characteristics: u32,
    pub matrix_coefficients: u32,
    pub chroma_loc_info_present_flag: bool,
    pub chroma_sample_loc_type_top_field: u32,
    pub chroma_sample_loc_type_bottom_field: u32,
    pub timing_info_present_flag: bool,
    pub num_units_in_tick: u32,
    pub time_scale: u32,
    pub fixed_frame_rate_flag: bool,
    pub nal_hrd_parameters_present_flag: bool,
    pub nal_hrd_parameters: Option<HrdParameters>,
    pub vcl_hrd_parameters_present_flag: bool,
    pub vcl_hrd_parameters: Option<HrdParameters>,
    pub low_delay_hrd_flag: bool,
    pub pic_struct_present_flag: bool,
    pub bitstream_restriction_flag: bool,
    pub motion_vectors_over_pic_boundaries_flag: bool,
    pub max_bytes_per_pic_denom: u32,
    pub max_bits_per_mb_denom: u32,
    pub log2_max_mv_length_horizontal: u32,
    pub log2_max_mv_length_vertical: u32,
    pub max_num_reorder_frames: u32,
    pub max_dec_frame_buffering: u32,
}

impl Default for VuiParameters {
    fn default() -> Self {
        Self {
            sample_aspect_ratio_width: 0,
            sample_aspect_ratio_height: 0,
            overscan_info_present_flag: false,
            overscan_appropriate_flag: false,
            video_signal_type_present_flag: false,
            video_format: 0,
            video_full_range_flag: false,
            colour_description_flag: false,
            colour_primaries: 0,
            transfer_characteristics: 0,
            matrix_coefficients: 0,
            chroma_loc_info_present_flag: false,
            chroma_sample_loc_type_top_field: 0,
            chroma_sample_loc_type_bottom_field: 0,
            timing_info_present_flag: false,
            num_units_in_tick: 0,
            time_scale: 0,
            fixed_frame_rate_flag: false,
            nal_hrd_parameters_present_flag: false,
            nal_hrd_parameters: None,
            vcl_hrd_parameters_present_flag: false,
            vcl_hrd_parameters: None,
            low_delay_hrd_flag: false,
            pic_struct_present_flag: false,
            bitstream_restriction_flag: false,
            motion_vectors_over_pic_boundaries_flag: false,
            max_bytes_per_pic_denom: 0,
            max_bits_per_mb_denom: 0,
            log2_max_mv_length_horizontal: 0,
            log2_max_mv_length_vertical: 0,
            max_num_reorder_frames: 0,
            max_dec_frame_buffering: 0,
        }
    }
}

/// AVC Sequence Parameter Set as parsed from a NAL unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sps {
    pub profile: u8,
    pub profile_compatibility: u8,
    pub level: u8,
    pub parameter_set_id: u32,
    pub chroma_format_idc: u32,
    pub separate_colour_plane_flag: bool,
    pub bit_depth_luma_minus8: u32,
    pub bit_depth_chroma_minus8: u32,
    pub qpprime_y_zero_transform_bypass_flag: bool,
    pub seq_scaling_matrix_present_flag: bool,
    pub seq_scaling_lists: Vec<Option<ScalingList>>,
    pub log2_max_frame_num_minus4: u32,
    pub pic_order_cnt_type: u32,
    pub log2_max_pic_order_cnt_lsb_minus4: u32,
    pub delta_pic_order_always_zero_flag: bool,
    pub offset_for_non_ref_pic: u32,
    pub offset_for_top_to_bottom_field: u32,
    pub ref_frames_in_pic_order_cnt_cycle: Vec<u32>,
    pub num_ref_frames: u32,
    pub gaps_in_frame_num_value_allowed_flag: bool,
    pub frame_mbs_only_flag: bool,
    pub mb_adaptive_frame_field_flag: bool,
    pub direct_8x8_inference_flag: bool,
    pub frame_cropping_flag: bool,
    pub frame_crop_left_offset: u32,
    pub frame_crop_right_offset: u32,
    pub frame_crop_top_offset: u32,
    pub frame_crop_bottom_offset: u32,
    pub width: u32,
    pub height: u32,
    pub nr_bytes_before_vui: i32,
    pub nr_bytes_read: i32,
    pub vui: Option<VuiParameters>,
}

impl Sps {
    /// Return the four constraint bits from `profile_compatibility`.
    pub fn constraint_flags(&self) -> u8 {
        self.profile_compatibility >> 4
    }

    /// Chroma array type as defined in the specification.
    pub fn chroma_array_type(&self) -> u32 {
        if !self.separate_colour_plane_flag {
            self.chroma_format_idc
        } else {
            0
        }
    }

    /// True if Cpb and Dpb delay values are present in the Picture Timing SEI.
    pub fn cpb_dpb_delays_present(&self) -> bool {
        if let Some(vui) = &self.vui {
            vui.nal_hrd_parameters_present_flag || vui.vcl_hrd_parameters_present_flag
        } else {
            false
        }
    }

    /// True if the pic_struct field is present in the Picture Timing SEI.
    pub fn pic_struct_present(&self) -> bool {
        self.vui.as_ref().map_or(false, |v| v.pic_struct_present_flag)
    }
}

/// Parse a SPS NAL unit including its header.
/// `parse_vui_beyond_aspect_ratio` controls whether only the aspect ratio part
/// of the VUI should be parsed (useful for codec string extraction).
pub fn parse_sps_nalu(nalu: &[u8]) -> Option<Sps> {
    parse_sps_nalu_with_vui(nalu, true)
}

pub fn parse_sps_nalu_with_vui(nalu: &[u8], parse_vui_beyond_aspect_ratio: bool) -> Option<Sps> {
    if nalu.is_empty() {
        return None;
    }
    if NaluType::from_header_byte(nalu[0]) != NaluType::SPS {
        return None;
    }
    let bytes = remove_emulation_prevention_bytes(&nalu[1..]);
    let mut r = BitReader::new(Cursor::new(bytes));

    let profile = r.read(8) as u8;
    let profile_compatibility = r.read(8) as u8;
    let level = r.read(8) as u8;
    let parameter_set_id = read_ue(&mut r);

    let mut chroma_format_idc = 1u32;
    if profile == 138 {
        chroma_format_idc = 0;
    }
    let mut separate_colour_plane_flag = false;
    let mut bit_depth_luma_minus8 = 0u32;
    let mut bit_depth_chroma_minus8 = 0u32;
    let mut qpprime_y_zero_transform_bypass_flag = false;
    let mut seq_scaling_matrix_present_flag = false;
    let mut seq_scaling_lists = Vec::new();

    match profile {
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135 => {
            chroma_format_idc = read_ue(&mut r);
            if chroma_format_idc == 3 {
                separate_colour_plane_flag = r.read_flag();
            }
            bit_depth_luma_minus8 = read_ue(&mut r);
            bit_depth_chroma_minus8 = read_ue(&mut r);
            qpprime_y_zero_transform_bypass_flag = r.read_flag();
            seq_scaling_matrix_present_flag = r.read_flag();
            if seq_scaling_matrix_present_flag {
                let mut nr = 12;
                if chroma_format_idc != 3 { nr = 8; }
                for i in 0..nr {
                    if r.read_flag() {
                        let size = if i < 6 { 16 } else { 64 };
                        seq_scaling_lists.push(Some(read_scaling_list(&mut r, size)));
                    } else {
                        seq_scaling_lists.push(None);
                    }
                }
            }
        }
        _ => {}
    }

    let log2_max_frame_num_minus4 = read_ue(&mut r);
    let pic_order_cnt_type = read_ue(&mut r);
    let mut log2_max_pic_order_cnt_lsb_minus4 = 0u32;
    let mut delta_pic_order_always_zero_flag = false;
    let mut offset_for_non_ref_pic = 0u32;
    let mut offset_for_top_to_bottom_field = 0u32;
    let mut ref_frames_in_pic_order_cnt_cycle = Vec::new();

    if pic_order_cnt_type == 0 {
        log2_max_pic_order_cnt_lsb_minus4 = read_ue(&mut r);
    } else if pic_order_cnt_type == 1 {
        delta_pic_order_always_zero_flag = r.read_flag();
        offset_for_non_ref_pic = read_ue(&mut r);
        offset_for_top_to_bottom_field = read_ue(&mut r);
        let num = read_ue(&mut r);
        for _ in 0..num {
            ref_frames_in_pic_order_cnt_cycle.push(read_ue(&mut r));
        }
    }

    let num_ref_frames = read_ue(&mut r);
    let gaps_in_frame_num_value_allowed_flag = r.read_flag();

    let pic_width_in_mbs_minus1 = read_ue(&mut r);
    let pic_height_in_map_units_minus1 = read_ue(&mut r);

    let mut width = (pic_width_in_mbs_minus1 + 1) * 16;
    let mut height = (pic_height_in_map_units_minus1 + 1) * 16;

    let frame_mbs_only_flag = r.read_flag();
    let mut mb_adaptive_frame_field_flag = false;
    if !frame_mbs_only_flag {
        mb_adaptive_frame_field_flag = r.read_flag();
    } else {
        // nothing
    }
    let direct_8x8_inference_flag = r.read_flag();
    let frame_cropping_flag = r.read_flag();

    let mut frame_crop_left_offset = 0u32;
    let mut frame_crop_right_offset = 0u32;
    let mut frame_crop_top_offset = 0u32;
    let mut frame_crop_bottom_offset = 0u32;

    let mut crop_unit_x = 0u32;
    let mut crop_unit_y = 0u32;
    let mut frame_mbs_only = 0u32;
    if frame_mbs_only_flag { frame_mbs_only = 1; } else { height *= 2; }

    if frame_cropping_flag {
        match chroma_format_idc {
            0 => { crop_unit_x = 1; crop_unit_y = 2 - frame_mbs_only; }
            1 => { crop_unit_x = 2; crop_unit_y = 2 * (2 - frame_mbs_only); }
            2 => { crop_unit_x = 2; crop_unit_y = 1 * (2 - frame_mbs_only); }
            3 => { crop_unit_x = 1; crop_unit_y = 1 * (2 - frame_mbs_only); }
            _ => return None,
        }
        frame_crop_left_offset = read_ue(&mut r);
        frame_crop_right_offset = read_ue(&mut r);
        frame_crop_top_offset = read_ue(&mut r);
        frame_crop_bottom_offset = read_ue(&mut r);
        let frame_crop_width = frame_crop_left_offset + frame_crop_right_offset;
        let frame_crop_height = frame_crop_top_offset + frame_crop_bottom_offset;
        width -= frame_crop_width * crop_unit_x;
        height -= frame_crop_height * crop_unit_y;
    }

    let vui_parameters_present_flag = r.read_flag();
    let nr_bytes_before_vui = r.nr_bytes_read() as i32;
    let mut vui = None;
    if vui_parameters_present_flag {
        vui = Some(parse_vui(&mut r, parse_vui_beyond_aspect_ratio));
    }
    let nr_bytes_read = r.nr_bytes_read() as i32;

    if r.acc_error().is_some() {
        return None;
    }

    Some(Sps {
        profile,
        profile_compatibility,
        level,
        parameter_set_id,
        chroma_format_idc,
        separate_colour_plane_flag,
        bit_depth_luma_minus8,
        bit_depth_chroma_minus8,
        qpprime_y_zero_transform_bypass_flag,
        seq_scaling_matrix_present_flag,
        seq_scaling_lists,
        log2_max_frame_num_minus4,
        pic_order_cnt_type,
        log2_max_pic_order_cnt_lsb_minus4,
        delta_pic_order_always_zero_flag,
        offset_for_non_ref_pic,
        offset_for_top_to_bottom_field,
        ref_frames_in_pic_order_cnt_cycle,
        num_ref_frames,
        gaps_in_frame_num_value_allowed_flag,
        frame_mbs_only_flag,
        mb_adaptive_frame_field_flag,
        direct_8x8_inference_flag,
        frame_cropping_flag,
        frame_crop_left_offset,
        frame_crop_right_offset,
        frame_crop_top_offset,
        frame_crop_bottom_offset,
        width,
        height,
        nr_bytes_before_vui,
        nr_bytes_read,
        vui,
    })
}

fn parse_vui<R: std::io::Read>(r: &mut BitReader<R>, beyond_aspect_ratio: bool) -> VuiParameters {
    let mut vui = VuiParameters::default();
    let aspect_ratio_info_present_flag = r.read_flag();
    if aspect_ratio_info_present_flag {
        let aspect_ratio_idc = r.read(8);
        if aspect_ratio_idc == EXTENDED_SAR {
            vui.sample_aspect_ratio_width = r.read(16);
            vui.sample_aspect_ratio_height = r.read(16);
        } else if let Some((w, h)) = get_sar_from_idc(aspect_ratio_idc) {
            vui.sample_aspect_ratio_width = w;
            vui.sample_aspect_ratio_height = h;
        }
    }
    if !beyond_aspect_ratio {
        return vui;
    }
    vui.overscan_info_present_flag = r.read_flag();
    if vui.overscan_info_present_flag {
        vui.overscan_appropriate_flag = r.read_flag();
    }
    vui.video_signal_type_present_flag = r.read_flag();
    if vui.video_signal_type_present_flag {
        vui.video_format = r.read(3);
        vui.video_full_range_flag = r.read_flag();
        vui.colour_description_flag = r.read_flag();
        if vui.colour_description_flag {
            vui.colour_primaries = r.read(8);
            vui.transfer_characteristics = r.read(8);
            vui.matrix_coefficients = r.read(8);
        }
    }
    vui.chroma_loc_info_present_flag = r.read_flag();
    if vui.chroma_loc_info_present_flag {
        vui.chroma_sample_loc_type_top_field = read_ue(r);
        vui.chroma_sample_loc_type_bottom_field = read_ue(r);
    }
    vui.timing_info_present_flag = r.read_flag();
    if vui.timing_info_present_flag {
        vui.num_units_in_tick = r.read(32);
        vui.time_scale = r.read(32);
        vui.fixed_frame_rate_flag = r.read_flag();
    }
    vui.nal_hrd_parameters_present_flag = r.read_flag();
    if vui.nal_hrd_parameters_present_flag {
        vui.nal_hrd_parameters = Some(parse_hrd_parameters(r));
    }
    vui.vcl_hrd_parameters_present_flag = r.read_flag();
    if vui.vcl_hrd_parameters_present_flag {
        vui.vcl_hrd_parameters = Some(parse_hrd_parameters(r));
    }
    if vui.nal_hrd_parameters_present_flag || vui.vcl_hrd_parameters_present_flag {
        vui.low_delay_hrd_flag = r.read_flag();
    }
    vui.pic_struct_present_flag = r.read_flag();
    vui.bitstream_restriction_flag = r.read_flag();
    if vui.bitstream_restriction_flag {
        vui.motion_vectors_over_pic_boundaries_flag = r.read_flag();
        vui.max_bytes_per_pic_denom = read_ue(r);
        vui.max_bits_per_mb_denom = read_ue(r);
        vui.log2_max_mv_length_horizontal = read_ue(r);
        vui.log2_max_mv_length_vertical = read_ue(r);
        vui.max_num_reorder_frames = read_ue(r);
        vui.max_dec_frame_buffering = read_ue(r);
    }
    vui
}

fn parse_hrd_parameters<R: std::io::Read>(r: &mut BitReader<R>) -> HrdParameters {
    let cpb_count_minus1 = read_ue(r);
    let bit_rate_scale = r.read(4);
    let cpb_size_scale = r.read(4);
    let mut cpb_entries = Vec::new();
    for _ in 0..=cpb_count_minus1 {
        let bit_rate_value_minus1 = read_ue(r);
        let cpb_size_value_minus1 = read_ue(r);
        let cbr_flag = r.read_flag();
        cpb_entries.push(CpbEntry { bit_rate_value_minus1, cpb_size_value_minus1, cbr_flag });
    }
    let initial_cpb_removal_delay_length_minus1 = r.read(5);
    let cpb_removal_delay_length_minus1 = r.read(5);
    let dpb_output_delay_length_minus1 = r.read(5);
    let time_offset_length = r.read(5);
    HrdParameters {
        cpb_count_minus1,
        bit_rate_scale,
        cpb_size_scale,
        cpb_entries,
        initial_cpb_removal_delay_length_minus1,
        cpb_removal_delay_length_minus1,
        dpb_output_delay_length_minus1,
        time_offset_length,
    }
}

fn get_sar_from_idc(index: u32) -> Option<(u32, u32)> {
    if index == 0 || index > 16 { return None; }
    let table = [
        (1,1), (12,11), (10,11), (16,11),
        (40,33), (24,11), (20,11), (32,11),
        (80,33), (18,11), (15,11), (64,33),
        (160,99), (4,3), (3,2), (2,1)
    ];
    Some(table[(index - 1) as usize])
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
