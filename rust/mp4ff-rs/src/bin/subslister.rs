use std::env;
use std::fs::File;
use std::io::{self, Read};

use mp4ff::subs;
<<<<<<< HEAD
use mp4ff::subs::SubtitleVariant;
=======
>>>>>>> master

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
<<<<<<< HEAD
        Err(_) => match subs::find_stpp_track(&data) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                return Ok(());
            }
        },
=======
        Err(e) => {
            eprintln!("{e}");
            return Ok(());
        }
>>>>>>> master
    };

    for (i, sample) in track.samples.iter().enumerate() {
        println!("Sample {}", i + 1);
<<<<<<< HEAD
        match track.variant {
            SubtitleVariant::Wvtt => subs::print_wvtt_sample(sample),
            SubtitleVariant::Stpp => subs::print_stpp_sample(sample),
        }
=======
        subs::print_wvtt_sample(sample);
>>>>>>> master
    }
    Ok(())
}
