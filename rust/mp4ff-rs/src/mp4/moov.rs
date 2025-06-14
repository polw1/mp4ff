use std::io::{self, Cursor, Seek, SeekFrom, Read};

use crate::bits::reader::{read_u8, read_u24, read_u32, read_u32_be, read_u64_be};
use super::r#box::read_box_header;

/// Parse `mdhd` box and return timescale
pub fn parse_mdhd_timescale(mdhd: &[u8]) -> Option<u32> {
    if mdhd.len() < 12 {
        return None;
    }
    let mut p = 0usize;
    let ver = mdhd[p];
    p += if ver == 1 { 4 + 8 + 8 } else { 4 + 4 + 4 };
    if p + 4 > mdhd.len() {
        return None;
    }
    let ts = u32::from_be_bytes([mdhd[p], mdhd[p + 1], mdhd[p + 2], mdhd[p + 3]]);
    Some(ts)
}

/// Parse `stts` box entries
pub fn parse_stts_entries(stts: &[u8]) -> Option<Vec<(u32, u32)>> {
    if stts.len() < 8 {
        return None;
    }
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

/// Parse the `moov` box extracting optional title and duration
pub fn parse_moov(
    data: &[u8],
    title: &mut Option<String>,
    artist: &mut Option<String>,
    album: &mut Option<String>,
    copyright: &mut Option<String>,
    duration: &mut Option<f64>,
) -> io::Result<()> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut pos = 0u64;
    while pos < len {
        let header = read_box_header(&mut cursor)?;
        let payload = header.size.saturating_sub(header.header_size);
        match header.name.as_str() {
            "mvhd" => {
                let version = read_u8(&mut cursor)?;
                let _flags = read_u24(&mut cursor)?;
                if version == 1 {
                    cursor.seek(SeekFrom::Current(16))?;
                    let timescale = read_u32_be(&mut cursor)?;
                    let dur = read_u64_be(&mut cursor)?;
                    *duration = Some(dur as f64 / timescale as f64);
                } else {
                    cursor.seek(SeekFrom::Current(8))?;
                    let timescale = read_u32_be(&mut cursor)?;
                    let dur = read_u32_be(&mut cursor)? as u64;
                    *duration = Some(dur as f64 / timescale as f64);
                }
                cursor.seek(SeekFrom::Current((payload - (cursor.position() - (pos + header.header_size))) as i64))?;
            }
            "udta" => {
                let mut buf = vec![0u8; payload as usize];
                cursor.read_exact(&mut buf)?;
                parse_udta(&buf, title, artist, album, copyright)?;
            }
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_udta(
    data: &[u8],
    title: &mut Option<String>,
    artist: &mut Option<String>,
    album: &mut Option<String>,
    copyright: &mut Option<String>,
) -> io::Result<()> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut pos = 0u64;
    while pos < len {
        let header = read_box_header(&mut cursor)?;
        let payload = header.size.saturating_sub(header.header_size);
        match header.name.as_str() {
            "meta" => {
                let start = cursor.position();
                let mut look = [0u8; 4];
                cursor.read_exact(&mut look)?;
                if &look != b"hdlr" {
                    cursor.seek(SeekFrom::Current(0))?; // version+flags present
                } else {
                    cursor.seek(SeekFrom::Current(-4))?;
                }
                let mut buf = vec![0u8; (payload - (cursor.position() - start)) as usize];
                cursor.read_exact(&mut buf)?;
                parse_meta(&buf, title, artist, album, copyright)?;
            }
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_meta(
    data: &[u8],
    title: &mut Option<String>,
    artist: &mut Option<String>,
    album: &mut Option<String>,
    copyright: &mut Option<String>,
) -> io::Result<()> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut pos = 0u64;
    while pos < len {
        let header = read_box_header(&mut cursor)?;
        let payload = header.size.saturating_sub(header.header_size);
        match header.name.as_str() {
            "ilst" => {
                let mut buf = vec![0u8; payload as usize];
                cursor.read_exact(&mut buf)?;
                parse_ilst(&buf, title, artist, album, copyright)?;
            }
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_ilst(
    data: &[u8],
    title: &mut Option<String>,
    artist: &mut Option<String>,
    album: &mut Option<String>,
    copyright: &mut Option<String>,
) -> io::Result<()> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut pos = 0u64;
    while pos < len {
        let header = read_box_header(&mut cursor)?;
        let payload = header.size.saturating_sub(header.header_size);
        let dest = match header.name.as_bytes() {
            b"\xa9nam" => &mut *title,
            b"\xa9ART" => &mut *artist,
            b"\xa9alb" => &mut *album,
            b"cprt" => &mut *copyright,
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
                pos += header.size;
                continue;
            }
        };
        let mut buf = vec![0u8; payload as usize];
        cursor.read_exact(&mut buf)?;
        parse_name_box(&buf, dest)?;
        pos += header.size;
    }
    Ok(())
}

fn parse_name_box(data: &[u8], dest: &mut Option<String>) -> io::Result<()> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut pos = 0u64;
    while pos < len {
        let header = read_box_header(&mut cursor)?;
        let payload = header.size.saturating_sub(header.header_size);
        if header.name.as_str() == "data" {
            cursor.seek(SeekFrom::Current(8))?; // type and locale
            let mut buf = vec![0u8; (payload - 8) as usize];
            cursor.read_exact(&mut buf)?;
            if let Ok(s) = String::from_utf8(buf) {
                *dest = Some(s);
            }
            break;
        } else {
            cursor.seek(SeekFrom::Current(payload as i64))?;
        }
        pos += header.size;
    }
    Ok(())
}
