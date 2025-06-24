use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use mp4ff::read_mp4_video_info;
use mp4ff::bits::reader::{read_u32, read_u64};
use mp4ff::mp4::r#box::{find_box, find_box_range, parse_box_header};
use mp4ff::mp4::moov::{parse_mdhd_timescale, parse_stts_entries};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file> [output.png]", args[0]);
        return;
    }
    let path = PathBuf::from(&args[1]);
    let out_path = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        path.with_extension("png")
    };

    let info = match read_mp4_video_info(&path) {
        Ok(Some(info)) => info,
        Ok(None) => {
            eprintln!("no video track found");
            return;
        }
        Err(e) => {
            eprintln!("Failed to read file: {e}");
            return;
        }
    };

    if info.codec != "avc1" && info.codec != "avc3" {
        eprintln!("unsupported codec: {}", info.codec);
        return;
    }

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open file: {e}");
            return;
        }
    };
    let mut data = Vec::new();
    if let Err(e) = file.read_to_end(&mut data) {
        eprintln!("Failed to read file: {e}");
        return;
    }

    match extract_frame_as_png(&data, 5.0, &out_path, info.width, info.height) {
        Ok(()) => println!("Saved thumbnail to {}", out_path.display()),
        Err(e) => eprintln!("Failed to extract thumbnail: {e}"),
    }
}

fn extract_frame_as_png(data: &[u8], seconds: f64, out: &Path, width: u16, height: u16) -> io::Result<()> {
    let sample = match find_video_sample(data, seconds) {
        Some(s) => s,
        None => return Err(io::Error::new(io::ErrorKind::Other, "sample not found")),
    };
    // TODO: Decode H264 sample to raw pixels using AVC parser
    // Placeholder: create a black PNG with the correct dimensions
    write_black_png(out, width, height)?;
    // Avoid unused variable warning
    let _ = sample;
    Ok(())
}

fn find_video_sample(data: &[u8], seconds: f64) -> Option<Vec<u8>> {
    let moov = find_box(data, "moov")?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos)?;
        if size as usize > moov.len() - start { return None; }
        let payload = &moov[pos..start + size as usize];
        if name == "trak" {
            if let Some(sample) = parse_video_trak(data, payload, seconds) { return Some(sample); }
        }
        pos = start + size as usize;
    }
    None
}

fn parse_video_trak(root: &[u8], data: &[u8], seconds: f64) -> Option<Vec<u8>> {
    let mdia = find_box(data, "mdia")?;
    let hdlr = find_box(mdia, "hdlr")?;
    if hdlr.len() < 12 || &hdlr[8..12] != b"vide" { return None; }
    let mdhd = find_box(mdia, "mdhd")?;
    let timescale = parse_mdhd_timescale(mdhd)?;

    let minf = find_box(mdia, "minf")?;
    let stbl = find_box(minf, "stbl")?;
    let stsz = find_box(stbl, "stsz")?;
    let (stco, use_co64) = if let Some(b) = find_box(stbl, "stco") {
        (b, false)
    } else {
        (find_box(stbl, "co64")?, true)
    };
    let stsc = find_box(stbl, "stsc")?;
    let stts = find_box(stbl, "stts")?;

    let mut p = 4;
    let sample_uniform = read_u32(stsz, &mut p)?;
    let sample_count = read_u32(stsz, &mut p)? as usize;
    let mut sizes = Vec::with_capacity(sample_count);
    if sample_uniform == 0 {
        for _ in 0..sample_count { sizes.push(read_u32(stsz, &mut p)?); }
    } else {
        for _ in 0..sample_count { sizes.push(sample_uniform); }
    }

    let mut p = 4;
    let entry_count = read_u32(stco, &mut p)? as usize;
    let mut chunk_offsets = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let off = if use_co64 { read_u64(stco, &mut p)? } else { read_u32(stco, &mut p)? as u64 };
        chunk_offsets.push(off);
    }

    let mut p = 4;
    let entry_count = read_u32(stsc, &mut p)? as usize;
    let mut stsc_entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let first_chunk = read_u32(stsc, &mut p)?;
        let samples_per_chunk = read_u32(stsc, &mut p)?;
        let desc_index = read_u32(stsc, &mut p)?;
        stsc_entries.push((first_chunk, samples_per_chunk, desc_index));
    }

    let entries = parse_stts_entries(stts)?;
    let mut durs = Vec::new();
    for (count, delta) in entries { for _ in 0..count { durs.push(delta); } }
    if durs.len() != sizes.len() { return None; }

    let target = (seconds * timescale as f64) as u64;
    let (_, mdat_start, mdat_end) = find_box_range(root, "mdat")?;
    let mdat_slice = &root[mdat_start..mdat_end];

    extract_sample_from_tables(
        mdat_slice,
        mdat_start as u64,
        &chunk_offsets,
        &stsc_entries,
        &sizes,
        &durs,
        target,
    )
}

fn extract_sample_from_tables(
    mdat: &[u8],
    base_offset: u64,
    chunk_offsets: &[u64],
    stsc_entries: &[(u32, u32, u32)],
    sizes: &[u32],
    durs: &[u32],
    target_time: u64,
) -> Option<Vec<u8>> {
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
                if sample_index >= sizes.len() { return None; }
                if decode_time >= target_time {
                    let size = sizes[sample_index] as usize;
                    let absolute = chunk_offset + offset_in_chunk;
                    if absolute >= base_offset {
                        let start = (absolute - base_offset) as usize;
                        let end = start + size;
                        if end <= mdat.len() {
                            return Some(mdat[start..end].to_vec());
                        } else { return None; }
                    }
                }
                offset_in_chunk += sizes[sample_index] as u64;
                decode_time += durs[sample_index] as u64;
                sample_index += 1;
            }
        }
    }
    None
}

fn write_chunk<W: Write>(w: &mut W, name: &[u8;4], data: &[u8]) -> io::Result<()> {
    let len = data.len() as u32;
    w.write_all(&len.to_be_bytes())?;
    w.write_all(name)?;
    w.write_all(data)?;
    let mut crc_data = Vec::with_capacity(name.len() + data.len());
    crc_data.extend_from_slice(name);
    crc_data.extend_from_slice(data);
    let crc = crc32(&crc_data);
    w.write_all(&crc.to_be_bytes())?;
    Ok(())
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffffffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn write_black_png(path: &Path, width: u16, height: u16) -> io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(&[137,80,78,71,13,10,26,10])?; // signature
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type truecolor
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut f, b"IHDR", &ihdr)?;

    let row_len = width as usize * 3 + 1;
    let mut raw = Vec::with_capacity(row_len * height as usize);
    for _ in 0..height {
        raw.push(0); // filter type
        for _ in 0..width { raw.extend_from_slice(&[0,0,0]); }
    }

    let mut comp = Vec::new();
    comp.extend_from_slice(&[0x78, 0x01]); // zlib header, no compression
    comp.push(0x01); // final block, uncompressed
    let len = raw.len() as u16;
    comp.extend_from_slice(&len.to_le_bytes());
    comp.extend_from_slice(&(!len).to_le_bytes());
    comp.extend_from_slice(&raw);
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for &b in raw.iter() {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    comp.extend_from_slice(&adler.to_be_bytes());
    write_chunk(&mut f, b"IDAT", &comp)?;
    write_chunk(&mut f, b"IEND", &[])?;
    Ok(())
}
