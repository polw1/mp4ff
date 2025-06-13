use std::str;

/// Supported subtitle track variants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleVariant {
    /// WebVTT in ISOBMFF
    Wvtt,
    /// TTML subtitles (stpp)
    Stpp,
    /// 3GPP timed text (tx3g)
    Tx3g,
}

/// A subtitle track and its extracted samples
/// Single subtitle sample with timing information
pub struct Sample {
    /// Raw bytes of the subtitle sample
    pub bytes: Vec<u8>,
    /// Decode time (start) in track timescale units
    pub start: u64,
    /// Duration in track timescale units
    pub dur: u32,
}

/// Subtitle track consisting of all extracted samples
pub struct Track {
    pub variant: SubtitleVariant,
    /// Timescale from the track `mdhd` box
    pub timescale: u32,
    pub samples: Vec<Sample>,
}

fn read_u32(data: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > data.len() { return None; }
    let v = u32::from_be_bytes([
        data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3],
    ]);
    *pos += 4;
    Some(v)
}

fn read_u64(data: &[u8], pos: &mut usize) -> Option<u64> {
    if *pos + 8 > data.len() { return None; }
    let v = u64::from_be_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
        data[*pos + 4],
        data[*pos + 5],
        data[*pos + 6],
        data[*pos + 7],
    ]);
    *pos += 8;
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

fn parse_mdhd_timescale(mdhd: &[u8]) -> Option<u32> {
    if mdhd.len() < 12 { return None; }
    let mut p = 0usize;
    let ver = mdhd[p];
    p += if ver == 1 { 4 + 8 + 8 } else { 4 + 4 + 4 };
    if p + 4 > mdhd.len() { return None; }
    let ts = u32::from_be_bytes([mdhd[p], mdhd[p+1], mdhd[p+2], mdhd[p+3]]);
    Some(ts)
}

fn parse_stts_entries(stts: &[u8]) -> Option<Vec<(u32, u32)>> {
    if stts.len() < 8 { return None; }
    let mut p = 4; // version+flags
    let entry_count = read_u32(stts, &mut p)? as usize;
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let count = read_u32(stts, &mut p)?;
        let delta = read_u32(stts, &mut p)?;
        entries.push((count, delta));
    }
    Some(entries)
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

pub fn find_tx3g_track(data: &[u8]) -> Result<Track, &'static str> {
    find_track_inner(data, SubtitleVariant::Tx3g).ok_or("no tx3g track")
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
            if handler != b"text" && handler != b"subt" {
                return None;
            }
        }
        SubtitleVariant::Stpp => {
            if handler != b"subt" { return None; }
        }
        SubtitleVariant::Tx3g => {
            if handler != b"sbtl" && handler != b"text" && handler != b"subt" {
                return None;
            }
        }
    }
    let mdhd = find_box(mdia, "mdhd")?;
    let timescale = parse_mdhd_timescale(mdhd)?;

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
        SubtitleVariant::Tx3g => {
            if !stsd.windows(4).any(|w| w == b"tx3g") { return None; }
        }
    }
    let stsz = find_box(stbl, "stsz")?;
    // chunk offsets may use either 32- or 64-bit entries
    let (stco, use_co64) = if let Some(b) = find_box(stbl, "stco") {
        (b, false)
    } else {
        (find_box(stbl, "co64")?, true)
    };
    let stsc = find_box(stbl, "stsc")?;
    let stts = find_box(stbl, "stts")?;

    // Parse stsz table with sample sizes
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

    // Parse chunk offsets (stco/co64)
    let mut p = 4; // version+flags
    let entry_count = read_u32(stco, &mut p)? as usize;
    let mut chunk_offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let off = if use_co64 {
            read_u64(stco, &mut p)?
        } else {
            read_u32(stco, &mut p)? as u64
        };
        chunk_offsets.push(off);
    }

    // Parse stsc entries
    let mut p = 4; // version+flags
    let entry_count = read_u32(stsc, &mut p)? as usize;
    let mut stsc_entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let first_chunk = read_u32(stsc, &mut p)?;
        let samples_per_chunk = read_u32(stsc, &mut p)?;
        let desc_index = read_u32(stsc, &mut p)?;
        stsc_entries.push((first_chunk, samples_per_chunk, desc_index));
    }

    // Parse stts entries for timing
    let stts_entries = parse_stts_entries(stts)?;
    let mut durations = Vec::new();
    for (count, delta) in stts_entries {
        for _ in 0..count { durations.push(delta); }
    }
    if durations.len() != sizes.len() { return None; }

    let (_, mdat_payload_start, mdat_end) = find_box_range(root, "mdat")?;
    let mdat_slice = &root[mdat_payload_start..mdat_end];
    Some(Track{
        variant,
        timescale,
        samples: collect_samples_general(
            mdat_slice,
            mdat_payload_start as u64,
            &chunk_offsets,
            &stsc_entries,
            &sizes,
            &durations,
        ),
    })
}

fn collect_samples_general(
    mdat: &[u8],
    base_offset: u64,
    chunk_offsets: &[u64],
    stsc_entries: &[(u32, u32, u32)],
    sizes: &[u32],
    durs: &[u32],
) -> Vec<Sample> {
    let mut samples = Vec::new();
    let mut sample_index = 0usize;
    let mut decode_time = 0u64;
    for (i, &(first_chunk, samples_per_chunk, _)) in stsc_entries.iter().enumerate() {
        let next_first_chunk = stsc_entries
            .get(i + 1)
            .map(|e| e.0)
            .unwrap_or(chunk_offsets.len() as u32 + 1);
        for chunk in first_chunk..next_first_chunk {
            let chunk_offset = chunk_offsets[(chunk - 1) as usize];
            let mut offset_in_chunk = 0u64;
            for _ in 0..samples_per_chunk {
                if sample_index >= sizes.len() { break; }
                let size = sizes[sample_index] as usize;
                let absolute = chunk_offset + offset_in_chunk;
                if absolute >= base_offset {
                    let start = (absolute - base_offset) as usize;
                    let end = start + size;
                    if end <= mdat.len() {
                        samples.push(Sample {
                            bytes: mdat[start..end].to_vec(),
                            start: decode_time,
                            dur: durs[sample_index],
                        });
                    }
                }
                offset_in_chunk += size as u64;
                decode_time += durs[sample_index] as u64;
                sample_index += 1;
            }
        }
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

pub fn print_tx3g_sample(sample: &[u8]) {
    if sample.len() < 2 {
        println!("  [binary {} bytes]", sample.len());
        return;
    }
    let len = u16::from_be_bytes([sample[0], sample[1]]) as usize;
    let end = 2 + len.min(sample.len() - 2);
    let text = &sample[2..end];
    if let Ok(s) = std::str::from_utf8(text) {
        println!("  {}", s);
    } else {
        println!("  [binary {} bytes]", sample.len());
    }
}

fn extract_wvtt_text(sample: &[u8]) -> Option<String> {
    let mut pos = 0usize;
    while pos + 8 <= sample.len() {
        let start = pos;
        if let Some((name, size)) = parse_box_header(sample, &mut pos) {
            if size as usize > sample.len() - start { break; }
            let payload = &sample[pos..start + size as usize];
            if name == "payl" {
                if let Ok(text) = std::str::from_utf8(payload) {
                    return Some(text.to_string());
                }
            }
            pos = start + size as usize;
        } else { break; }
    }
    None
}

fn extract_stpp_text(sample: &[u8]) -> Option<String> {
    std::str::from_utf8(sample).ok().map(|s| s.to_string())
}

fn extract_tx3g_text(sample: &[u8]) -> Option<String> {
    if sample.len() < 2 { return None; }
    let len = u16::from_be_bytes([sample[0], sample[1]]) as usize;
    let end = 2 + len.min(sample.len() - 2);
    std::str::from_utf8(&sample[2..end]).ok().map(|s| s.to_string())
}

/// Decode subtitle sample text depending on variant
pub fn extract_text(variant: SubtitleVariant, sample: &[u8]) -> Option<String> {
    match variant {
        SubtitleVariant::Wvtt => extract_wvtt_text(sample),
        SubtitleVariant::Stpp => extract_stpp_text(sample),
        SubtitleVariant::Tx3g => extract_tx3g_text(sample),
    }
}
