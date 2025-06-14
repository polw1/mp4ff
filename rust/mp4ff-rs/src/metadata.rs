use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, BufReader};
use std::path::Path;

use crate::mp4::r#box::read_box_header;
use crate::mp4::moov::parse_moov;

/// Basic metadata extracted from an MP4 file.
#[derive(Debug, Default, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub copyright: Option<String>,
    pub duration: Option<f64>,
    pub size: u64,
}


/// Read the `moov` box of an MP4 file and return basic [`Metadata`].
///
/// This helper is used by the binary in `main.rs` to demonstrate metadata
/// extraction.
pub fn read_mp4_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let file = File::open(&path)?;
    let size = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut title = None;
    let mut artist = None;
    let mut album = None;
    let mut copyright = None;
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
                parse_moov(
                    &buf,
                    &mut title,
                    &mut artist,
                    &mut album,
                    &mut copyright,
                    &mut duration,
                )?;
            }
            _ => {
                reader.seek(SeekFrom::Current(payload as i64))?;
            }
        }
        pos += header.size;
    }
    Ok(Metadata {
        title,
        artist,
        album,
        copyright,
        duration,
        size,
    })
}

