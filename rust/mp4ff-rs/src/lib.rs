pub mod bits;

pub use bits::reader::{BitReader, mask};

mod bit_writer;
pub use bit_writer::BitWriter;

mod metadata;
pub use metadata::{Metadata, read_mp4_metadata};

pub mod mp4;

pub mod subs;
pub use subs::*;

#[cfg(test)]
mod metadata_tests {
    use super::read_mp4_metadata;
    use std::path::PathBuf;

    #[test]
    fn test_read_mp4_metadata_prog_8s() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../mp4/testdata/prog_8s.mp4");
        let md = read_mp4_metadata(&p).expect("metadata");
        assert_eq!(md.size, std::fs::metadata(&p).unwrap().len());
        assert!(md.duration.is_some());
    }
}
