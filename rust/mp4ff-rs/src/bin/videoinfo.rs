use std::env;
use std::path::PathBuf;

use mp4ff::read_mp4_video_info;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file>", args[0]);
        return;
    }
    let path = PathBuf::from(&args[1]);
    match read_mp4_video_info(&path) {
        Ok(Some(info)) => {
            println!("codec: {}", info.codec);
            println!("width: {}", info.width);
            println!("height: {}", info.height);
        }
        Ok(None) => println!("no video track found"),
        Err(e) => eprintln!("Failed to read file: {e}"),
    }
}
