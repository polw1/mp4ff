use std::env;
use std::path::PathBuf;
use mp4ff::avc::decoder::save_thumbnail;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file>", args[0]);
        return;
    }
    let path = PathBuf::from(&args[1]);
    let out_path = path.with_file_name("thumbnail.png");
    match save_thumbnail(&path, &out_path) {
        Ok(()) => println!("Saved {}", out_path.display()),
        Err(e) => eprintln!("Failed to create thumbnail: {e}"),
    }
}
