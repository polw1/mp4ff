use std::io;
use std::path::Path;
use std::process::Command;

/// Save a thumbnail image extracted at 5 seconds using the builtin decoder.
///
/// Currently this function is a thin wrapper around `ffmpeg` as no
/// Rust-based H.264 decoder is provided.
pub fn save_thumbnail(mp4: &Path, out: &Path) -> io::Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-y", "-ss", "5", "-i"])
        .arg(mp4)
        .args(["-frames:v", "1"])
        .arg(out)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, format!("ffmpeg exited with {status}")))
    }
}
