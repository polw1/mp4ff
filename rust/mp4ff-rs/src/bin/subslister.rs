use std::env;
use std::fs::File;
use std::io::{self, Read};

use mp4ff::subs;
use mp4ff::subs::SubtitleVariant;

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

    for (i, sample) in track.samples.iter().enumerate() {
        println!("Sample {}", i + 1);
        match track.variant {
            SubtitleVariant::Wvtt => subs::print_wvtt_sample(sample),
            SubtitleVariant::Stpp => subs::print_stpp_sample(sample),
            SubtitleVariant::Tx3g => subs::print_tx3g_sample(sample),
        }
    }
    Ok(())
}
