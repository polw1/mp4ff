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
            println!("  duration: {}", md.duration.map_or("unknown".to_string(), format_duration));
            println!("  artist: {}", md.artist.unwrap_or_default());
            println!("  album: {}", md.album.unwrap_or_default());
            println!("  copyright: {}", md.copyright.unwrap_or_default());
            println!("  size: {}", format_size(md.size));
        }
        Err(e) => eprintln!("Failed to read metadata: {e}"),
    }
}


fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, secs)
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.2} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
