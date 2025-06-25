use crate::avc::get_nalus_from_sample;
use crate::bits::reader::{read_u32, read_u64};
use crate::mp4::r#box::{find_box, find_box_range, parse_box_header};
use crate::mp4::moov::{parse_mdhd_timescale, parse_stts_entries};

pub struct Sample {
    /// Raw bytes of the sample
    pub bytes: Vec<u8>,
    /// Decode time (start) in track timescale units
    pub start: u64,
    /// Duration in track timescale units
    pub dur: u32,
    /// Parsed NAL units extracted from `bytes`
    pub nalus: Vec<Vec<u8>>, 
}

/// Error type returned when extraction fails
#[derive(Debug)]
pub enum Error {
    InvalidData(&'static str),
}

/// Extract the first AVC video track from an MP4 file
pub fn extract_avc_track(data: &[u8]) -> Result<Vec<Sample>, Error> {
    let moov = find_box(data, "moov").ok_or(Error::InvalidData("no moov"))?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos).ok_or(Error::InvalidData("invalid box"))?;
        if size as usize > moov.len() - start { return Err(Error::InvalidData("size")); }
        let payload = &moov[pos..start + size as usize];
        if name == "trak" {
            if let Some(samples) = parse_trak(data, payload) { return Ok(samples); }
        }
        pos = start + size as usize;
    }
    Err(Error::InvalidData("no video trak"))
}

fn parse_trak(root: &[u8], data: &[u8]) -> Option<Vec<Sample>> {
    let mdia = find_box(data, "mdia")?;
    let hdlr = find_box(mdia, "hdlr")?;
    if hdlr.len() < 12 { return None; }
    if &hdlr[8..12] != b"vide" { return None; }
    let mdhd = find_box(mdia, "mdhd")?;
    let _timescale = parse_mdhd_timescale(mdhd)?;

    let minf = find_box(mdia, "minf")?;
    let stbl = find_box(minf, "stbl")?;
    let stsd = find_box(stbl, "stsd")?;
    if !stsd.windows(4).any(|w| w == b"avc1" || w == b"avc3") {
        return None;
    }
    let stsz = find_box(stbl, "stsz")?;
    let (stco, use_co64) = if let Some(b) = find_box(stbl, "stco") {
        (b, false)
    } else {
        (find_box(stbl, "co64")?, true)
    };
    let stsc = find_box(stbl, "stsc")?;
    let stts = find_box(stbl, "stts")?;

    // stsz
    let mut p = 4; // version+flags
    let sample_uniform = read_u32(stsz, &mut p)?;
    let sample_count = read_u32(stsz, &mut p)? as usize;
    let mut sizes = Vec::with_capacity(sample_count);
    if sample_uniform == 0 {
        for _ in 0..sample_count { sizes.push(read_u32(stsz, &mut p)?); }
    } else {
        for _ in 0..sample_count { sizes.push(sample_uniform); }
    }

    // stco/co64
    let mut p = 4;
    let entry_count = read_u32(stco, &mut p)? as usize;
    let mut chunk_offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let off = if use_co64 { read_u64(stco, &mut p)? } else { read_u32(stco, &mut p)? as u64 };
        chunk_offsets.push(off);
    }

    // stsc
    let mut p = 4;
    let entry_count = read_u32(stsc, &mut p)? as usize;
    let mut stsc_entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let first_chunk = read_u32(stsc, &mut p)?;
        let samples_per_chunk = read_u32(stsc, &mut p)?;
        let desc_index = read_u32(stsc, &mut p)?;
        stsc_entries.push((first_chunk, samples_per_chunk, desc_index));
    }

    // stts
    let stts_entries = parse_stts_entries(stts)?;
    let mut durations = Vec::new();
    for (count, delta) in stts_entries { for _ in 0..count { durations.push(delta); } }
    if durations.len() != sizes.len() { return None; }

    let (_, mdat_payload_start, mdat_end) = find_box_range(root, "mdat")?;
    let mdat_slice = &root[mdat_payload_start..mdat_end];
    Some(collect_samples(mdat_slice, mdat_payload_start as u64, &chunk_offsets, &stsc_entries, &sizes, &durations))
}

fn collect_samples(
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
        let next_first_chunk = stsc_entries.get(i + 1).map(|e| e.0).unwrap_or(chunk_offsets.len() as u32 + 1);
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
                        let slice = &mdat[start..end];
                        let nalus = get_nalus_from_sample(slice).unwrap_or_default().into_iter().map(|n| n.to_vec()).collect();
                        samples.push(Sample { bytes: slice.to_vec(), start: decode_time, dur: durs[sample_index], nalus });
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
