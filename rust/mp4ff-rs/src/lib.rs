pub mod bits;

pub use bits::reader::{BitReader, mask};

mod bit_writer;
pub use bit_writer::BitWriter;

mod metadata;
pub use metadata::{Metadata, read_mp4_metadata};

mod videoinfo;
pub use videoinfo::{VideoInfo, read_mp4_video_info};

pub mod mp4;

pub mod avc;

pub mod subs;
pub use subs::*;

mod video_track;
pub use video_track::{extract_avc_track, Sample as VideoSample, Error as VideoError};

mod h264decoder;
pub use h264decoder::{Decoder, DecodedYUV, H264Error};

#[cfg(test)]
mod metadata_tests {
    use super::read_mp4_metadata;
    use super::read_mp4_video_info;
    use std::path::PathBuf;

    #[test]
    fn test_read_mp4_metadata_prog_8s() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../mp4/testdata/prog_8s.mp4");
        let md = read_mp4_metadata(&p).expect("metadata");
        assert_eq!(md.size, std::fs::metadata(&p).unwrap().len());
        assert!(md.duration.is_some());
    }

    #[test]
    fn test_video_info_prog_8s() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../mp4/testdata/prog_8s.mp4");
        let info = read_mp4_video_info(&p).expect("info").unwrap();
        assert_eq!(info.codec, "avc1");
        assert_eq!(info.width, 640);
        assert_eq!(info.height, 360);
    }
}
