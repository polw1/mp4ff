use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::mp4::r#box::{find_box, parse_box_header};

/// Basic video track information
#[derive(Debug, PartialEq, Eq)]
pub struct VideoInfo {
    pub width: u16,
    pub height: u16,
    pub codec: String,
}

/// Read the first video track in an MP4 file and return [`VideoInfo`].
pub fn read_mp4_video_info<P: AsRef<Path>>(path: P) -> io::Result<Option<VideoInfo>> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok(find_video_track(&data))
}

fn find_video_track(data: &[u8]) -> Option<VideoInfo> {
    let moov = find_box(data, "moov")?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos)?;
        if size as usize > moov.len() - start { return None; }
        let payload = &moov[pos..start + size as usize];
        if name == "trak" {
            if let Some(info) = parse_trak(payload) { return Some(info); }
        }
        pos = start + size as usize;
    }
    None
}

fn parse_trak(data: &[u8]) -> Option<VideoInfo> {
    let mdia = find_box(data, "mdia")?;
    let hdlr = find_box(mdia, "hdlr")?;
    if hdlr.len() < 12 { return None; }
    if &hdlr[8..12] != b"vide" { return None; }
    let tkhd = find_box(data, "tkhd")?;
    let (width, height) = parse_tkhd_size(tkhd)?;
    let minf = find_box(mdia, "minf")?;
    let stbl = find_box(minf, "stbl")?;
    let stsd = find_box(stbl, "stsd")?;
    let codec = parse_stsd_codec(stsd)?;
    Some(VideoInfo { width, height, codec })
}

fn parse_tkhd_size(tkhd: &[u8]) -> Option<(u16, u16)> {
    if tkhd.len() < 84 { return None; }
    let version = tkhd[0];
    let pos = if version == 1 { 88 } else { 76 };
    if tkhd.len() < pos + 8 { return None; }
    let w = u32::from_be_bytes([tkhd[pos], tkhd[pos+1], tkhd[pos+2], tkhd[pos+3]]) >> 16;
    let h = u32::from_be_bytes([tkhd[pos+4], tkhd[pos+5], tkhd[pos+6], tkhd[pos+7]]) >> 16;
    Some((w as u16, h as u16))
}

fn parse_stsd_codec(stsd: &[u8]) -> Option<String> {
    if stsd.len() < 16 { return None; }
    let mut p = 0usize;
    let _ = parse_box_header(stsd, &mut p)?; // stsd header
    if p + 8 > stsd.len() { return None; }
    let _entry_size = u32::from_be_bytes([stsd[p], stsd[p+1], stsd[p+2], stsd[p+3]]) as usize;
    let typ = &stsd[p+4..p+8];
    std::str::from_utf8(typ).ok().map(|s| s.to_string())
}
