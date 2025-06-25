use std::env;
use std::fs;
use std::path::PathBuf;

use image::RgbImage;
use mp4ff::avc::{self, decode_avc_decoder_config, get_parameter_sets, NaluType, parse_sps_nalu};
use mp4ff::{extract_avc_track};
use mp4ff::mp4::r#box::{find_box, parse_box_header};
use mp4ff::mp4::moov::parse_mdhd_timescale;

fn find_video_timescale(data: &[u8]) -> Option<u32> {
    let moov = find_box(data, "moov")?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos)?;
        if size as usize > moov.len() - start { return None; }
        let payload = &moov[pos..start + size as usize];
        if name == "trak" {
            let mdia = find_box(payload, "mdia")?;
            let hdlr = find_box(mdia, "hdlr")?;
            if hdlr.len() < 12 { return None; }
            if &hdlr[8..12] != b"vide" { pos = start + size as usize; continue; }
            let mdhd = find_box(mdia, "mdhd")?;
            return parse_mdhd_timescale(mdhd);
        }
        pos = start + size as usize;
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file>", args[0]);
        std::process::exit(1);
    }
    let path = PathBuf::from(&args[1]);
    let data = fs::read(&path)?;

    let samples = extract_avc_track(&data).map_err(|_| "no avc track")?;
    let timescale = find_video_timescale(&data).ok_or("no timescale")?;

    // gather SPS/PPS
    let mut sps_list = Vec::new();
    let mut pps_list = Vec::new();
    if let Some(first) = samples.get(0) {
        let (s, p) = get_parameter_sets(&first.bytes);
        sps_list = s;
        pps_list = p;
    }
    if sps_list.is_empty() || pps_list.is_empty() {
        if let Some(conf) = decode_avc_decoder_config(&data) {
            if sps_list.is_empty() { sps_list = conf.sps; }
            if pps_list.is_empty() { pps_list = conf.pps; }
        }
    }
    let sps = sps_list.get(0).ok_or("no sps")?;
    let sps_parsed = parse_sps_nalu(sps).ok_or("bad sps")?;

    let target = (timescale as u64) * 5;
    let mut chosen = &samples[0];
    for s in &samples {
        if s.start >= target { chosen = s; break; }
    }

    // Ensure the chosen sample has an IDR NALU
    if !chosen.nalus.iter().any(|n| NaluType::from_header_byte(n[0]) == NaluType::IDR) {
        eprintln!("No IDR at target position, using first sample");
    }

    let img: RgbImage = avc::decode_idr_to_rgb(&chosen.nalus, &sps_parsed);
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let out_path = path.with_file_name(format!("thumbnail_{}.png", file_stem));
    img.save(&out_path)?;
    println!("Thumbnail saved to {}", out_path.display());
    Ok(())
}
