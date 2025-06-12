

//! Simple binary that prints metadata for an MP4 file.
//!
//! Usage: `cargo run -- [PATH_TO_MP4]`
//!
//! If no path is provided, the bundled `files/video.mp4` is used.

use std::env;
use std::path::PathBuf;

use mp4ff::read_mp4_metadata;

fn main() {
    // Determine the file to read from the first CLI argument or fall back to
    // the bundled sample file.
    let args: Vec<String> = env::args().collect();
    let path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("files/video.mp4");
        p
    };

    match read_mp4_metadata(&path) {
        Ok(md) => {
            println!("Title: {:?}", md.title);
            println!("Duration: {:?}", md.duration);
            println!("Size: {} bytes", md.size);
        }
        Err(e) => eprintln!("Failed to read metadata: {e}"),
    }
}
