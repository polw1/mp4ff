use std::env;
use std::path::PathBuf;
use std::process::Command;

use mp4ff::read_mp4_video_info;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4 file> [output.png]", args[0]);
        return;
    }
    let path = PathBuf::from(&args[1]);
    let out_path = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        path.with_extension("png")
    };

    let info = match read_mp4_video_info(&path) {
        Ok(Some(info)) => info,
        Ok(None) => {
            eprintln!("no video track found");
            return;
        }
        Err(e) => {
            eprintln!("Failed to read file: {e}");
            return;
        }
    };

    if info.codec != "avc1" && info.codec != "avc3" {
        eprintln!("unsupported codec: {}", info.codec);
        return;
    }

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg("5")
        .arg("-i")
        .arg(path)
        .arg("-frames:v")
        .arg("1")
        .arg("-vf")
        .arg("scale=320:-1")
        .arg(&out_path)
        .status();

    match status {
        Ok(s) if s.success() => println!("Saved thumbnail to {}", out_path.display()),
        Ok(s) => eprintln!("ffmpeg exited with status {}", s),
        Err(e) => eprintln!("Failed to run ffmpeg: {e}"),
    }
}
