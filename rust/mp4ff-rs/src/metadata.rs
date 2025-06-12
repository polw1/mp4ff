use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, BufReader, Cursor};
use std::path::Path;

#[derive(Debug, Default, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub size: u64,
}

struct BoxHeader {
    name: String,
    size: u64,
    header_size: u64,
}

fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u24<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 3];
    r.read_exact(&mut buf)?;
    Ok(((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | buf[2] as u32)
}

fn read_u32_be<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u64_be<R: Read>(r: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_be_bytes(buf))
}

fn read_box_header<R: Read>(r: &mut R) -> io::Result<BoxHeader> {
    let size32 = read_u32_be(r)?;
    let mut name_buf = [0u8; 4];
    r.read_exact(&mut name_buf)?;
    let mut size = size32 as u64;
    let mut header_size = 8u64;
    if size32 == 1 {
        size = read_u64_be(r)?;
        header_size = 16;
    }
    Ok(BoxHeader { name: String::from_utf8_lossy(&name_buf).into_owned(), size, header_size })
}

pub fn read_mp4_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let file = File::open(&path)?;
    let size = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut title = None;
    let mut duration = None;
    let mut pos = 0u64;
    while pos < size {
        let header = match read_box_header(&mut reader) {
            Ok(h) => h,
            Err(_) => break,
        };
        let payload = header.size.saturating_sub(header.header_size);
        match header.name.as_str() {
            "moov" => {
                let mut buf = vec![0u8; payload as usize];
                reader.read_exact(&mut buf)?;
                parse_moov(&buf, &mut title, &mut duration)?;
            }
            _ => {
                reader.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(Metadata { title, duration, size })
}

fn parse_moov(data: &[u8], title: &mut Option<String>, duration: &mut Option<f64>) -> io::Result<()> {
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
                parse_udta(&buf, title)?;
            }
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_udta(data: &[u8], title: &mut Option<String>) -> io::Result<()> {
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
                    // version + flags present
                    cursor.seek(SeekFrom::Current(0))?; // look already consumed
                } else {
                    cursor.seek(SeekFrom::Current(-4))?;
                }
                let mut buf = vec![0u8; (payload - (cursor.position() - start)) as usize];
                cursor.read_exact(&mut buf)?;
                parse_meta(&buf, title)?;
            }
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_meta(data: &[u8], title: &mut Option<String>) -> io::Result<()> {
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
                parse_ilst(&buf, title)?;
            }
            _ => {
                cursor.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_ilst(data: &[u8], title: &mut Option<String>) -> io::Result<()> {
    let mut cursor = Cursor::new(data);
    let len = data.len() as u64;
    let mut pos = 0u64;
    while pos < len {
        let header = read_box_header(&mut cursor)?;
        let payload = header.size.saturating_sub(header.header_size);
        if header.name.as_bytes() == b"\xa9nam" {
            let mut buf = vec![0u8; payload as usize];
            cursor.read_exact(&mut buf)?;
            parse_name_box(&buf, title)?;
        } else {
            cursor.seek(SeekFrom::Current(payload as i64))?;
        }
        pos += header.size;
    }
    Ok(())
}

fn parse_name_box(data: &[u8], title: &mut Option<String>) -> io::Result<()> {
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
                *title = Some(s);
            }
            break;
        } else {
            cursor.seek(SeekFrom::Current(payload as i64))?;
        }
        pos += header.size;
    }
    Ok(())
}
