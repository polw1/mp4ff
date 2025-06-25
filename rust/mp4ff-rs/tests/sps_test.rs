use mp4ff::avc::{parse_sps_nalu_with_vui, codec_string, Sps, VuiParameters, HrdParameters, CpbEntry};

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

const SPS1: &str = "67640020accac05005bb0169e0000003002000000c9c4c000432380008647c12401cb1c31380";
const SPS2: &str = "6764000dacd941419f9e10000003001000000303c0f1429960";
const SPS3: &str = "27640020ac2ec05005bb011000000300100000078e840016e300005b8d8bdef83b438627";

#[test]
fn test_sps_parser1() {
    let data = decode_hex(SPS1);
    let mut got = parse_sps_nalu_with_vui(&data, true).expect("sps");
    got.nr_bytes_before_vui = 0;
    got.nr_bytes_read = 0;
    let want = Sps {
        profile: 100,
        profile_compatibility: 0,
        level: 32,
        parameter_set_id: 0,
        chroma_format_idc: 1,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        seq_scaling_lists: Vec::new(),
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 4,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        ref_frames_in_pic_order_cnt_cycle: Vec::new(),
        num_ref_frames: 2,
        gaps_in_frame_num_value_allowed_flag: false,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        width: 1280,
        height: 720,
        nr_bytes_before_vui: 0,
        nr_bytes_read: 0,
        vui: Some(VuiParameters {
            sample_aspect_ratio_width: 1,
            sample_aspect_ratio_height: 1,
            overscan_info_present_flag: false,
            overscan_appropriate_flag: false,
            video_signal_type_present_flag: true,
            video_format: 5,
            video_full_range_flag: false,
            colour_description_flag: false,
            colour_primaries: 0,
            transfer_characteristics: 0,
            matrix_coefficients: 0,
            chroma_loc_info_present_flag: true,
            chroma_sample_loc_type_top_field: 0,
            chroma_sample_loc_type_bottom_field: 0,
            timing_info_present_flag: true,
            num_units_in_tick: 1,
            time_scale: 100,
            fixed_frame_rate_flag: true,
            nal_hrd_parameters_present_flag: true,
            nal_hrd_parameters: Some(HrdParameters {
                cpb_count_minus1: 0,
                bit_rate_scale: 1,
                cpb_size_scale: 3,
                cpb_entries: vec![CpbEntry { bit_rate_value_minus1: 34374, cpb_size_value_minus1: 34374, cbr_flag: true }],
                initial_cpb_removal_delay_length_minus1: 16,
                cpb_removal_delay_length_minus1: 9,
                dpb_output_delay_length_minus1: 4,
                time_offset_length: 0,
            }),
            vcl_hrd_parameters_present_flag: false,
            vcl_hrd_parameters: None,
            low_delay_hrd_flag: false,
            pic_struct_present_flag: true,
            bitstream_restriction_flag: true,
            motion_vectors_over_pic_boundaries_flag: true,
            max_bytes_per_pic_denom: 4,
            max_bits_per_mb_denom: 0,
            log2_max_mv_length_horizontal: 13,
            log2_max_mv_length_vertical: 11,
            max_num_reorder_frames: 1,
            max_dec_frame_buffering: 2,
        }),
    };
    assert_eq!(got, want);
}

#[test]
fn test_sps_parser2() {
    let data = decode_hex(SPS2);
    let mut got = parse_sps_nalu_with_vui(&data, true).expect("sps");
    got.nr_bytes_before_vui = 0;
    got.nr_bytes_read = 0;
    let want = Sps {
        profile: 100,
        profile_compatibility: 0,
        level: 13,
        parameter_set_id: 0,
        chroma_format_idc: 1,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        seq_scaling_lists: Vec::new(),
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 2,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        ref_frames_in_pic_order_cnt_cycle: Vec::new(),
        num_ref_frames: 4,
        gaps_in_frame_num_value_allowed_flag: false,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: true,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 6,
        width: 320,
        height: 180,
        nr_bytes_before_vui: 0,
        nr_bytes_read: 0,
        vui: Some(VuiParameters {
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
            timing_info_present_flag: true,
            num_units_in_tick: 1,
            time_scale: 60,
            fixed_frame_rate_flag: false,
            nal_hrd_parameters_present_flag: false,
            nal_hrd_parameters: None,
            vcl_hrd_parameters_present_flag: false,
            vcl_hrd_parameters: None,
            low_delay_hrd_flag: false,
            pic_struct_present_flag: false,
            bitstream_restriction_flag: true,
            motion_vectors_over_pic_boundaries_flag: true,
            max_bytes_per_pic_denom: 0,
            max_bits_per_mb_denom: 0,
            log2_max_mv_length_horizontal: 9,
            log2_max_mv_length_vertical: 9,
            max_num_reorder_frames: 2,
            max_dec_frame_buffering: 4,
        }),
    };
    assert_eq!(got, want);
}

#[test]
fn test_sps_parser3() {
    let data = decode_hex(SPS3);
    let mut got = parse_sps_nalu_with_vui(&data, true).expect("sps");
    got.nr_bytes_before_vui = 0;
    got.nr_bytes_read = 0;
    let want = Sps {
        profile: 100,
        profile_compatibility: 0,
        level: 32,
        parameter_set_id: 0,
        chroma_format_idc: 1,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        seq_scaling_lists: Vec::new(),
        log2_max_frame_num_minus4: 4,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        ref_frames_in_pic_order_cnt_cycle: Vec::new(),
        num_ref_frames: 2,
        gaps_in_frame_num_value_allowed_flag: false,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        width: 1280,
        height: 720,
        nr_bytes_before_vui: 0,
        nr_bytes_read: 0,
        vui: Some(VuiParameters {
            sample_aspect_ratio_width: 1,
            sample_aspect_ratio_height: 1,
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
            timing_info_present_flag: true,
            num_units_in_tick: 1,
            time_scale: 120,
            fixed_frame_rate_flag: true,
            nal_hrd_parameters_present_flag: true,
            nal_hrd_parameters: Some(HrdParameters {
                cpb_count_minus1: 0,
                bit_rate_scale: 4,
                cpb_size_scale: 2,
                cpb_entries: vec![CpbEntry { bit_rate_value_minus1: 5858, cpb_size_value_minus1: 187499, cbr_flag: false }],
                initial_cpb_removal_delay_length_minus1: 23,
                cpb_removal_delay_length_minus1: 23,
                dpb_output_delay_length_minus1: 23,
                time_offset_length: 24,
            }),
            vcl_hrd_parameters_present_flag: false,
            vcl_hrd_parameters: None,
            low_delay_hrd_flag: false,
            pic_struct_present_flag: true,
            bitstream_restriction_flag: true,
            motion_vectors_over_pic_boundaries_flag: true,
            max_bytes_per_pic_denom: 2,
            max_bits_per_mb_denom: 1,
            log2_max_mv_length_horizontal: 13,
            log2_max_mv_length_vertical: 11,
            max_num_reorder_frames: 1,
            max_dec_frame_buffering: 2,
        }),
    };
    assert_eq!(got, want);
}

#[test]
fn test_codec_string() {
    let data = decode_hex(SPS1);
    let sps = parse_sps_nalu_with_vui(&data, true).unwrap();
    let codec = codec_string("avc3", &sps);
    assert_eq!(codec, "avc3.640020");
}
