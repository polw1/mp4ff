use std::env;
use std::path::PathBuf;

use mp4ff::read_mp4_metadata;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file>", args[0]);
        return;
    }
    let path = PathBuf::from(&args[1]);
    match read_mp4_metadata(&path) {
        Ok(md) => {
            println!("Metadata:");
            println!("  title: {}", md.title.unwrap_or_default());
            println!("  artist: {}", md.artist.unwrap_or_default());
            println!("  album: {}", md.album.unwrap_or_default());
            println!("  copyright: {}", md.copyright.unwrap_or_default());
        }
        Err(e) => eprintln!("Failed to read metadata: {e}"),
    }
}
