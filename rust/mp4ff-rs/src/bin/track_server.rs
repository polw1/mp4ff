use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use mp4ff::{avc, extract_avc_track};
use mp4ff::avc::{DecConfRec, decode_avc_decoder_config, get_parameter_sets};
use mp4ff::mp4::r#box::{find_box, parse_box_header};

fn extract_decoder_config(data: &[u8]) -> Option<DecConfRec> {
    fn parse_trak_avcc(trak: &[u8]) -> Option<DecConfRec> {
        let mdia = find_box(trak, "mdia")?;
        let hdlr = find_box(mdia, "hdlr")?;
        if hdlr.len() < 12 || &hdlr[8..12] != b"vide" { return None; }
        let minf = find_box(mdia, "minf")?;
        let stbl = find_box(minf, "stbl")?;
        let stsd = find_box(stbl, "stsd")?;
        let mut p = 0usize;
        let _ = parse_box_header(stsd, &mut p)?; // stsd header
        if p + 8 > stsd.len() { return None; }
        let entry_size = u32::from_be_bytes([stsd[p], stsd[p+1], stsd[p+2], stsd[p+3]]) as usize;
        p += 4;
        if p + 4 > stsd.len() { return None; }
        let format = &stsd[p..p+4];
        let entry_start = p - 4;
        p += 4;
        if entry_start + entry_size > stsd.len() { return None; }
        if format != b"avc1" && format != b"avc3" { return None; }
        let entry = &stsd[entry_start..entry_start + entry_size];
        let mut q = 78usize;
        while q + 8 <= entry.len() {
            let start = q;
            let (name, size) = parse_box_header(entry, &mut q)?;
            if size as usize > entry.len() - start { return None; }
            if name == "avcC" {
                return decode_avc_decoder_config(&entry[q..start + size as usize]);
            }
            q = start + size as usize;
        }
        None
    }

    let moov = find_box(data, "moov")?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos)?;
        if size as usize > moov.len() - start { return None; }
        let payload = &moov[pos..start + size as usize];
        if name == "trak" {
            if let Some(cfg) = parse_trak_avcc(payload) { return Some(cfg); }
        }
        pos = start + size as usize;
    }
    None
}

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
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: video/h264\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            track.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(track);
    } else {
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            html.len()
        );
        let _ = stream.write_all(header.as_bytes());
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
    let html_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/files/track.html");
    let html = fs::read(&html_path)?;
    let file_data = fs::read(&mp4_path)?;
    let samples = extract_avc_track(&file_data).expect("failed to parse avc track");

    // collect SPS/PPS from first sample or avcC
    let mut sps = Vec::new();
    let mut pps = Vec::new();
    if let Some(first) = samples.get(0) {
        let (fsps, fpps) = get_parameter_sets(&first.bytes);
        sps = fsps;
        pps = fpps;
    }
    if sps.is_empty() || pps.is_empty() {
        if let Some(conf) = extract_decoder_config(&file_data) {
            if sps.is_empty() { sps = conf.sps; }
            if pps.is_empty() { pps = conf.pps; }
        }
    }

    let mut track_bytes = Vec::new();
    for nalu in &sps {
        track_bytes.extend_from_slice(&[0, 0, 0, 1]);
        track_bytes.extend_from_slice(nalu);
    }
    for nalu in &pps {
        track_bytes.extend_from_slice(&[0, 0, 0, 1]);
        track_bytes.extend_from_slice(nalu);
    }
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
