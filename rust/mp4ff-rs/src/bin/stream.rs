use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use mp4ff::read_mp4_metadata;

fn handle_client(mut stream: TcpStream, html: &[u8], video: &[u8]) {
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
    if path.ends_with(".mp4") {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\n\r\n");
        let _ = stream.write_all(video);
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
    let html_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("files/stream.html");
    let html = fs::read(&html_path)?;
    let video = fs::read(&mp4_path)?;
    // Use the parser to validate metadata before serving
    let _ = read_mp4_metadata(&mp4_path);

    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Serving {} on http://localhost:8080", mp4_path.display());
    for stream in listener.incoming() {
        match stream {
            Ok(s) => handle_client(s, &html, &video),
            Err(e) => eprintln!("Connection failed: {e}"),
        }
    }
    Ok(())
}
