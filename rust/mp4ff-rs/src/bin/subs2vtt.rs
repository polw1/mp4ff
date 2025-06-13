use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use mp4ff::subs::{self, SubtitleVariant};

fn timestamp(ts: u64, timescale: u32) -> String {
    let millis = ts * 1000 / timescale as u64;
    let h = millis / 3_600_000;
    let m = (millis % 3_600_000) / 60_000;
    let s = (millis % 60_000) / 1000;
    let ms = millis % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file>", args[0]);
        return Ok(());
    }
    let mut file = File::open(&args[1])?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let track = match subs::find_wvtt_track(&data) {
        Ok(t) => t,
        Err(_) => match subs::find_stpp_track(&data) {
            Ok(t) => t,
            Err(_) => match subs::find_tx3g_track(&data) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{e}");
                    return Ok(());
                }
            },
        },
    };

    let out_path = Path::new(&args[1]).with_extension("vtt");
    let mut out = File::create(out_path)?;
    writeln!(out, "WEBVTT\n")?;
    for (i, sample) in track.samples.iter().enumerate() {
        let start = timestamp(sample.start, track.timescale);
        let end = timestamp(sample.start + sample.dur as u64, track.timescale);
        writeln!(out, "{}", i + 1)?;
        writeln!(out, "{} --> {}", start, end)?;
        if let Some(text) = subs::extract_text(track.variant, &sample.bytes) {
            writeln!(out, "{}\n", text)?;
        } else {
            writeln!(out, "[binary]\n")?;
        }
    }
    Ok(())
}

