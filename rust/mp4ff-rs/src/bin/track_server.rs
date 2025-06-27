use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use mp4ff::{avc, extract_avc_track};
use mp4ff::avc::{DecConfRec, decode_avc_decoder_config, get_parameter_sets};
use mp4ff::mp4::r#box::{find_box, parse_box_header};
use mp4ff::mp4::moov::parse_mdhd_timescale;

#[derive(Clone)]
struct SampleInfo {
    start: u64,
    dur: u32,
    offset: usize,
    len: usize,
}

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

fn get_video_timescale(data: &[u8]) -> Option<u32> {
    fn parse_trak_ts(trak: &[u8]) -> Option<u32> {
        let mdia = find_box(trak, "mdia")?;
        let hdlr = find_box(mdia, "hdlr")?;
        if hdlr.len() < 12 || &hdlr[8..12] != b"vide" { return None; }
        let mdhd = find_box(mdia, "mdhd")?;
        parse_mdhd_timescale(mdhd)
    }

    let moov = find_box(data, "moov")?;
    let mut pos = 0usize;
    while pos + 8 <= moov.len() {
        let start = pos;
        let (name, size) = parse_box_header(moov, &mut pos)?;
        if size as usize > moov.len() - start { return None; }
        let payload = &moov[pos..start + size as usize];
        if name == "trak" {
            if let Some(ts) = parse_trak_ts(payload) { return Some(ts); }
        }
        pos = start + size as usize;
    }
    None
}

fn handle_client(
    mut stream: TcpStream,
    mp4_data: &Arc<Vec<u8>>,
    track: &Arc<Vec<u8>>,
    infos: &Arc<Vec<SampleInfo>>,
    param_len: usize,
    start_time: Instant,
    timescale: u32,
) {
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
        let elapsed = start_time.elapsed();
        let elapsed_ts = (elapsed.as_secs_f64() * timescale as f64) as u64;
        let mut idx = infos.len();
        for (i, info) in infos.iter().enumerate() {
            if info.start + info.dur as u64 > elapsed_ts {
                idx = i;
                break;
            }
        }
        if idx == infos.len() {
            let _ = stream.write_all(b"HTTP/1.1 410 GONE\r\nConnection: close\r\n\r\n");
            return;
        }
        let total_len = param_len + (track.len() - infos[idx].offset);
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: video/h264\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            total_len
        );
        if stream.write_all(header.as_bytes()).is_err() { return; }
        if stream.write_all(&track[..param_len]).is_err() { return; }
        for info in &infos[idx..] {
            let abs = start_time
                + Duration::from_secs_f64(info.start as f64 / timescale as f64);
            let now = Instant::now();
            if abs > now {
                thread::sleep(abs - now);
            }
            let end = info.offset + info.len;
            if stream.write_all(&track[info.offset..end]).is_err() { return; }
        }
    } else if path.ends_with("video.mp4") {
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            mp4_data.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(mp4_data);
    } else {
        let header = b"HTTP/1.1 404 NOT FOUND\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(header);
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mp4_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("files/video.mp4")
    };
    let file_data = fs::read(&mp4_path)?;
    let samples = extract_avc_track(&file_data).expect("failed to parse avc track");
    let timescale = get_video_timescale(&file_data).expect("no video timescale");

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
    let param_len = track_bytes.len();

    let mut infos = Vec::new();
    for s in &samples {
        let offset = track_bytes.len();
        let bytes = avc::convert_sample_to_bytestream(&s.bytes);
        let len = bytes.len();
        track_bytes.extend_from_slice(&bytes);
        infos.push(SampleInfo { start: s.start, dur: s.dur, offset, len });
    }

    let duration_ts = infos
        .last()
        .map(|i| i.start + i.dur as u64)
        .unwrap_or(0);
    let duration = Duration::from_secs_f64(duration_ts as f64 / timescale as f64);

    let mp4_data = Arc::new(file_data);
    let track_bytes = Arc::new(track_bytes);
    let infos = Arc::new(infos);

    let start_time = Instant::now();
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    listener.set_nonblocking(true)?;
    println!("Serving {} on http://localhost:8080", mp4_path.display());
    loop {
        if start_time.elapsed() >= duration { break; }
        match listener.accept() {
            Ok((s, _)) => {
                let mp4 = Arc::clone(&mp4_data);
                let track = Arc::clone(&track_bytes);
                let info = Arc::clone(&infos);
                let st = start_time;
                std::thread::spawn(move || {
                    handle_client(s, &mp4, &track, &info, param_len, st, timescale);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => eprintln!("Connection failed: {e}"),
        }
    }
    Ok(())
}
