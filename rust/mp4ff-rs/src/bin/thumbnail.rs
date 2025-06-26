//! Minimal example decoding a H.264 file using `openh264-sys2` only.
//!
//! The example reads an Annex‑B formatted H.264 bitstream and saves the first
//! decoded frame as a PNG image.  It implements the few helpers required
//! directly so the file is self contained.


use mp4ff::{Decoder, H264Error};

use image::RgbImage;
use std::fs::File;
use std::io::Read;


// How many zeros we must see before a `1` indicates a NAL start.
const NAL_MIN_0_COUNT: usize = 2;

/// Return the index of the `nth` NAL prefix in `stream`.
fn nth_nal_index(stream: &[u8], nth: usize) -> Option<usize> {
    let mut count_0 = 0;
    let mut n = 0;
    for (i, byte) in stream.iter().enumerate() {
        match byte {
            0 => count_0 += 1,
            1 if count_0 >= NAL_MIN_0_COUNT => {
                if n == nth {
                    return Some(i - NAL_MIN_0_COUNT);
                }
                count_0 = 0;
                n += 1;
            }
            _ => count_0 = 0,
        }
    }
    None
}

/// Split a H.264 Annex‑B stream into NAL units.
fn nal_units(mut stream: &[u8]) -> impl Iterator<Item = &[u8]> {
    std::iter::from_fn(move || {
        let first = nth_nal_index(stream, 0);
        let next = nth_nal_index(stream, 1);
        match (first, next) {
            (Some(f), Some(n)) => {
                let val = &stream[f..n];
                stream = &stream[n..];
                Some(val)
            }
            (Some(f), None) => {
                let val = &stream[f..];
                stream = &stream[f + NAL_MIN_0_COUNT..];
                Some(val)
            }
            _ => None,
        }
    })
}


fn main() -> Result<(), H264Error> {
    let mut args = std::env::args();
    let bin = args.next().unwrap_or_else(|| "sample_decode".into());
    let input = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: {bin} <h264-file> [output.png]");
            std::process::exit(1);
        }
    };
    let output = args.next().unwrap_or_else(|| "frame.png".into());

    let mut data = Vec::new();
    File::open(&input)?.read_to_end(&mut data)?;

    let mut decoder = Decoder::new()?;

    let mut rgb = Vec::new();
    let mut width = 0usize;
    let mut height = 0usize;

    for packet in nal_units(&data) {
        match decoder.decode(packet)? {
            Some(image) => {
                let dim = image.dimensions();
                width = dim.0;
                height = dim.1;
                rgb.resize(width * height * 3, 0);
                image.write_rgb8(&mut rgb);
                break;
            }
            None => continue,
        }
    }

    if width == 0 || height == 0 {
        eprintln!("No frame decoded");
        return Ok(());
    }

    let img = RgbImage::from_vec(width as u32, height as u32, rgb)
        .ok_or_else(|| H264Error::msg("Failed to create image"))?;
    img.save(&output)?;
    println!("Wrote first frame to {output}");

    Ok(())
}


