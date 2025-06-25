use mp4ff::avc::{parse_pps_nalu, Pps};

#[test]
fn test_pps_parser() {
    let data = [0x68u8, 0xe8, 0x43, 0x32, 0xc8, 0xb0];
    let got = parse_pps_nalu(&data).expect("pps");
    let wanted = Pps {
        pic_parameter_set_id: 0,
        seq_parameter_set_id: 0,
        entropy_coding_mode_flag: true,
        bottom_field_pic_order_in_frame_present_flag: false,
        num_slice_groups_minus1: 0,
        slice_group_map_type: 0,
        run_length_minus1: Vec::new(),
        top_left: Vec::new(),
        bottom_right: Vec::new(),
        slice_group_change_direction_flag: false,
        slice_group_change_rate_minus1: 0,
        pic_size_in_map_units_minus1: 0,
        slice_group_id: Vec::new(),
        num_ref_idx_i0_default_active_minus1: 15,
        num_ref_idx_i1_default_active_minus1: 0,
        weighted_pred_flag: true,
        weighted_bipred_idc: 0,
        pic_init_qp_minus26: 0,
        pic_init_qs_minus26: 0,
        chroma_qp_index_offset: -2,
        deblocking_filter_control_present_flag: true,
        constrained_intra_pred_flag: false,
        redundant_pic_cnt_present_flag: false,
        transform8x8_mode_flag: true,
        pic_scaling_matrix_present_flag: false,
        pic_scaling_lists: Vec::new(),
        second_chroma_qp_index_offset: -2,
    };
    assert_eq!(got, wanted);
}
