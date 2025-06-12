use std::io::{self, Read};

/// `BitReader` reads bits from an underlying reader and accumulates the first
/// error that occurs. This is a minimal translation of the Go `bits.Reader`.
#[derive(Debug)]
pub struct BitReader<R: Read> {
    rd: R,
    err: Option<io::Error>,
    n: u32,
    value: u64,
    pos: i64,
}

/// Mask for the `n` least significant bits.
pub fn mask(n: u32) -> u32 {
    if n == 32 {
        u32::MAX
    } else {
        (1u32 << n) - 1
    }
}

impl<R: Read> BitReader<R> {
    /// Create a new `BitReader` that starts accumulating errors.
    pub fn new(rd: R) -> Self {
        Self { rd, err: None, n: 0, value: 0, pos: -1 }
    }

    /// Return the accumulated error if any.
    pub fn acc_error(&self) -> Option<&io::Error> {
        self.err.as_ref()
    }

    /// Read `n` bits and return them as the lowest bits of a `u32`.
    /// If an error has occurred, 0 is returned.
    pub fn read(&mut self, n: u32) -> u32 {
        if self.err.is_some() {
            return 0;
        }
        while self.n < n {
            let mut buf = [0u8; 1];
            match self.rd.read_exact(&mut buf) {
                Ok(()) => {
                    self.pos += 1;
                    self.value = (self.value << 8) | u64::from(buf[0]);
                    self.n += 8;
                }
                Err(e) => {
                    self.err = Some(e);
                    return 0;
                }
            }
        }
        let value = (self.value >> (self.n - n)) as u32;
        self.n -= n;
        self.value &= (1u64 << self.n) - 1;
        value
    }

    /// Read `n` bits and interpret as a signed integer.
    pub fn read_signed(&mut self, n: u32) -> i32 {
        let v = self.read(n);
        if n == 0 {
            return 0;
        }
        let first = v >> (n - 1);
        if first == 1 {
            (v as i32) | (!0 << n)
        } else {
            v as i32
        }
    }

    /// Read a single bit interpreted as a boolean flag.
    pub fn read_flag(&mut self) -> bool {
        self.read(1) == 1
    }

    /// Read remaining bytes if currently byte-aligned.
    pub fn read_remaining_bytes(&mut self) -> Option<Vec<u8>> {
        if self.err.is_some() {
            return None;
        }
        if self.n != 0 {
            self.err = Some(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "{} bit instead of byte alignment when reading remaining bytes",
                    self.n
                ),
            ));
            return None;
        }
        let mut rest = Vec::new();
        if let Err(e) = self.rd.read_to_end(&mut rest) {
            self.err = Some(e);
            return None;
        }
        Some(rest)
    }

    /// Number of bytes read from the underlying reader.
    pub fn nr_bytes_read(&self) -> i64 {
        self.pos + 1
    }

    /// Total number of bits read.
    pub fn nr_bits_read(&self) -> i64 {
        let mut nr = self.nr_bytes_read() * 8;
        if self.nr_bits_read_in_current_byte() != 8 {
            nr += self.nr_bits_read_in_current_byte() - 8;
        }
        nr
    }

    /// Number of bits consumed in the current byte.
    pub fn nr_bits_read_in_current_byte(&self) -> i64 {
        8 - self.n as i64
    }
}

#[cfg(test)]
mod tests {
    use super::{BitReader, mask};
    use std::io::Cursor;

    #[test]
    fn test_read_bits() {
        let data = [0xffu8, 0x0f];
        let mut r = BitReader::new(Cursor::new(&data));
        assert_eq!(r.read(2), 3); // 11
        assert_eq!(r.read(3), 7); // 111
        assert_eq!(r.read(5), 28); // 11100
        assert_eq!(r.read(3), 1); // 001
        assert_eq!(r.read(3), 7); // 111
        assert!(r.acc_error().is_none());
    }

    #[test]
    fn test_read_signed_bits() {
        let data = [0xffu8, 0x0c];
        let mut r = BitReader::new(Cursor::new(&data));
        assert_eq!(r.read_signed(2), -1);
        assert_eq!(r.read_signed(3), -1);
        assert_eq!(r.read_signed(5), -4);
        assert_eq!(r.read_signed(3), 1);
        assert_eq!(r.read_signed(3), -4);
        assert!(r.acc_error().is_none());
    }
    #[test]
    fn test_writer_mask() {
        assert_eq!(mask(8), 0xff);
        assert_eq!(mask(4), 0x0f);
    }
}

mod bit_writer;
pub use bit_writer::BitWriter;

mod metadata;
pub use metadata::{Metadata, read_mp4_metadata};

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
