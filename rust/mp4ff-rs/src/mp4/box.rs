use std::io::{self, Read};

use crate::bits::reader::{read_u32, read_u32_be, read_u64, read_u64_be};

/// Box header information
#[derive(Debug)]
pub struct BoxHeader {
    pub name: String,
    pub name_bytes: [u8; 4],
    pub size: u64,
    pub header_size: u64,
}

/// Read a box header from an io source
pub fn read_box_header<R: Read>(r: &mut R) -> io::Result<BoxHeader> {
    let size32 = read_u32_be(r)?;
    let mut name_buf = [0u8; 4];
    r.read_exact(&mut name_buf)?;
    let mut size = size32 as u64;
    let mut header_size = 8u64;
    if size32 == 1 {
        size = read_u64_be(r)?;
        header_size = 16;
    }
    Ok(BoxHeader {
        name: String::from_utf8_lossy(&name_buf).into_owned(),
        name_bytes: name_buf,
        size,
        header_size,
    })
}

/// Parse a box header from a byte slice advancing the cursor
pub fn parse_box_header(data: &[u8], pos: &mut usize) -> Option<(String, u64)> {
    if *pos + 8 > data.len() {
        return None;
    }
    let size = read_u32(data, pos)? as u64;
    let name = &data[*pos..*pos + 4];
    *pos += 4;
    let mut real_size = size;
    if size == 1 {
        if *pos + 8 > data.len() {
            return None;
        }
        real_size = read_u64(data, pos)?;
    }
    Some((std::str::from_utf8(name).ok()?.to_string(), real_size))
}

/// Find a box and return the contained slice
pub fn find_box<'a>(data: &'a [u8], name: &str) -> Option<&'a [u8]> {
    let (_, start, end) = find_box_range(data, name)?;
    Some(&data[start..end])
}

/// Find a box and return the start and end indices of its payload
pub fn find_box_range<'a>(data: &'a [u8], name: &str) -> Option<(usize, usize, usize)> {
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let start = pos;
        let (box_name, size) = parse_box_header(data, &mut pos)?;
        if size as usize > data.len() - start {
            return None;
        }
        let payload_start = pos;
        let payload_end = start + size as usize;
        if box_name == name {
            return Some((start, payload_start, payload_end));
        }
        pos = payload_end;
    }
    None
}
