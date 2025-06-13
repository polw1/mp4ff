use std::str;

/// Supported subtitle track variants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleVariant {
    /// WebVTT in ISOBMFF
    Wvtt,
    /// TTML subtitles (stpp)
    Stpp,
}

/// A subtitle track and its extracted samples
pub struct Track {
    pub variant: SubtitleVariant,
    pub samples: Vec<Vec<u8>>,
}

fn read_u32(data: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > data.len() { return None; }
    let v = u32::from_be_bytes([data[*pos], data[*pos+1], data[*pos+2], data[*pos+3]]);
    *pos += 4;
    Some(v)
}

fn parse_box_header(data: &[u8], pos: &mut usize) -> Option<(String, u64)> {
    if *pos + 8 > data.len() { return None; }
    let size = read_u32(data, pos)? as u64;
    let name = &data[*pos..*pos+4];
    *pos += 4;
    let mut real_size = size;
    if size == 1 {
        if *pos + 8 > data.len() { return None; }
        real_size = u64::from_be_bytes([
            data[*pos], data[*pos+1], data[*pos+2], data[*pos+3],
            data[*pos+4], data[*pos+5], data[*pos+6], data[*pos+7],
        ]);
        *pos += 8;
    }
    Some((str::from_utf8(name).ok()?.to_string(), real_size))
}

fn find_box<'a>(data: &'a [u8], name: &str) -> Option<&'a [u8]> {
    let (_, start, end) = find_box_range(data, name)?;
    Some(&data[start..end])
}

fn find_box_range<'a>(data: &'a [u8], name: &str) -> Option<(usize, usize, usize)> {
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let start = pos;
        let (box_name, size) = parse_box_header(data, &mut pos)?;
        if size as usize > data.len() - start { return None; }
        let payload_start = pos;
        let payload_end = start + size as usize;
        if box_name == name { return Some((start, payload_start, payload_end)); }
        pos = payload_end;
    }
    None
}

pub fn find_wvtt_track(data: &[u8]) -> Result<Track, &'static str> {
    find_track_inner(data, SubtitleVariant::Wvtt).ok_or("no wvtt track")
}

pub fn find_stpp_track(data: &[u8]) -> Result<Track, &'static str> {
    find_track_inner(data, SubtitleVariant::Stpp).ok_or("no stpp track")
}

fn find_track_inner(data: &[u8], variant: SubtitleVariant) -> Option<Track> {
    let moov = find_box(data, "moov")?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos)?;
        if size as usize > moov.len() - start { return None; }
        let payload = &moov[pos .. start + size as usize];
        if name == "trak" {
            if let Some(track) = parse_trak(data, payload, variant) { return Some(track); }
        }
        pos = start + size as usize;
    }
    None
}

fn parse_trak(root: &[u8], data: &[u8], variant: SubtitleVariant) -> Option<Track> {
    let mdia = find_box(data, "mdia")?;
    let hdlr = find_box(mdia, "hdlr")?;
    if hdlr.len() < 16 { return None; }
    let handler = &hdlr[8..12];
    match variant {
        SubtitleVariant::Wvtt => {
            if handler != b"text" { return None; }
        }
        SubtitleVariant::Stpp => {
            if handler != b"subt" { return None; }
        }
    }
    let minf = find_box(mdia, "minf")?;
    let stbl = find_box(minf, "stbl")?;
    let stsd = find_box(stbl, "stsd")?;
    match variant {
        SubtitleVariant::Wvtt => {
            if !stsd.windows(4).any(|w| w == b"wvtt") { return None; }
        }
        SubtitleVariant::Stpp => {
            if !stsd.windows(4).any(|w| w == b"stpp") { return None; }
        }
    }
    let stsz = find_box(stbl, "stsz")?;
    let stco = find_box(stbl, "stco")?;
    let stsc = find_box(stbl, "stsc")?;

    // Simple parsing with assumption 1 sample per chunk and single stsc entry
    let mut p = 4; // skip version+flags
    let sample_uniform = read_u32(stsz, &mut p)?;
    let sample_count = read_u32(stsz, &mut p)? as usize;
    let mut sizes = Vec::with_capacity(sample_count);
    if sample_uniform == 0 {
        for _ in 0..sample_count {
            sizes.push(read_u32(stsz, &mut p)?);
        }
    } else {
        for _ in 0..sample_count { sizes.push(sample_uniform); }
    }

    let mut p = 4; // stco version+flags
    let entry_count = read_u32(stco, &mut p)? as usize;
    let mut offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count { offsets.push(read_u32(stco, &mut p)? as u64); }

    let mut p = 4; // stsc version+flags
    let entries = read_u32(stsc, &mut p)? as usize;
    if entries != 1 { return None; }
    let first_chunk = read_u32(stsc, &mut p)?;
    let samples_per_chunk = read_u32(stsc, &mut p)?;
    if first_chunk != 1 || samples_per_chunk != 1 { return None; }
    let _desc = read_u32(stsc, &mut p)?;

    if offsets.len() != sizes.len() { return None; }

    let (_, mdat_payload_start, mdat_end) = find_box_range(root, "mdat")?;
    let mdat_slice = &root[mdat_payload_start..mdat_end];
    Some(Track{
        variant,
        samples: collect_samples(mdat_slice, mdat_payload_start as u64, &offsets, &sizes),
    })
}

fn collect_samples(mdat: &[u8], base_offset: u64, offsets: &[u64], sizes: &[u32]) -> Vec<Vec<u8>> {
    let mut samples = Vec::new();
    for (&off, &size) in offsets.iter().zip(sizes.iter()) {
        if off < base_offset { continue; }
        let start = (off - base_offset) as usize;
        let end = start + size as usize;
        if end <= mdat.len() { samples.push(mdat[start..end].to_vec()); }
    }
    samples
}

pub fn print_wvtt_sample(sample: &[u8]) {
    let mut pos = 0usize;
    while pos + 8 <= sample.len() {
        let start = pos;
        if let Some((name, size)) = parse_box_header(sample, &mut pos) {
            if size as usize > sample.len() - start { break; }
            let payload = &sample[pos..start + size as usize];
            if name == "payl" {
                if let Ok(text) = std::str::from_utf8(payload) {
                    println!("  cue: {}", text);
                }
            }
            pos = start + size as usize;
        } else { break; }
    }
}

pub fn print_stpp_sample(sample: &[u8]) {
    if let Ok(text) = std::str::from_utf8(sample) {
        println!("  {}", text);
    } else {
        println!("  [binary {} bytes]", sample.len());
    }
}
