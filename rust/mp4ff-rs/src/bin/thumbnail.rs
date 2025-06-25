use std::env;
use std::path::PathBuf;
// use mp4ff::avc::decoder::save_thumbnail; // not yet ported in Rust

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file>", args[0]);
        return;
    }
    let path = PathBuf::from(&args[1]);
    // Thumbnail creation is not yet implemented in the Rust port.
    let _out_path = path.with_file_name("thumbnail.png");
    eprintln!("Thumbnail creation not implemented");
}
