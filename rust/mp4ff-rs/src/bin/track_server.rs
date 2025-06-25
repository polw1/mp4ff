use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use mp4ff::{avc, extract_avc_track};

fn handle_client(mut stream: TcpStream, html: &[u8], track: &[u8]) {
    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");
    if path.ends_with(".h264") {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/h264\r\n\r\n");
        let _ = stream.write_all(track);
    } else {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n");
        let _ = stream.write_all(html);
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mp4_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("files/video.mp4")
    };
    let html_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("files/track.html");
    let html = fs::read(&html_path)?;
    let file_data = fs::read(&mp4_path)?;
    let samples = extract_avc_track(&file_data).expect("failed to parse avc track");
    let mut track_bytes = Vec::new();
    for s in &samples {
        track_bytes.extend_from_slice(&avc::convert_sample_to_bytestream(&s.bytes));
    }

    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Serving {} on http://localhost:8080", mp4_path.display());
    for stream in listener.incoming() {
        match stream {
            Ok(s) => handle_client(s, &html, &track_bytes),
            Err(e) => eprintln!("Connection failed: {e}"),
        }
    }
    Ok(())
}
