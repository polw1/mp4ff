use std::io::{self, Write};

/// `BitWriter` accumulates bits and writes them to an underlying writer.
#[derive(Debug)]
pub struct BitWriter<W: Write> {
    wr: W,
    err: Option<io::Error>,
    out: [u8; 1],
    n: u32,
    v: u32,
}

impl<W: Write> BitWriter<W> {
    /// Create a new `BitWriter` that accumulates errors.
    pub fn new(wr: W) -> Self {
        Self { wr, err: None, out: [0], n: 0, v: 0 }
    }

    /// Write `n` bits from `bits`.
    pub fn write(&mut self, bits: u32, n: u32) {
        if self.err.is_some() {
            return;
        }
        self.v <<= n;
        self.v |= bits & super::mask(n);
        self.n += n;
        while self.n >= 8 {
            let b = (self.v >> (self.n - 8)) & super::mask(8);
            self.out[0] = b as u8;
            if let Err(e) = self.wr.write_all(&self.out) {
                self.err = Some(e);
                return;
            }
            self.n -= 8;
        }
        self.v &= super::mask(8);
    }

    /// Flush remaining bits to the writer, padding with zeros.
    pub fn flush(&mut self) {
        if self.err.is_some() {
            return;
        }
        if self.n != 0 {
            let b = (self.v << (8 - self.n)) & super::mask(8);
            self.out[0] = b as u8;
            if let Err(e) = self.wr.write_all(&self.out) {
                self.err = Some(e);
                return;
            }
            self.n = 0;
            self.v = 0;
        }
    }

    /// Return the accumulated error if any.
    pub fn acc_error(&self) -> Option<&io::Error> {
        self.err.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::BitWriter;
    use std::io::Cursor;

    #[test]
    fn test_writer() {
        let cases = vec![
            (vec![255u32], vec![0xffu8], 8),
            (vec![15, 15], vec![0xffu8], 4),
            (vec![3, 3, 3, 3], vec![0xffu8], 2),
            (vec![1, 1, 1, 1, 1, 1, 1, 1], vec![0xffu8], 1),
            (vec![15, 15, 15], vec![0xffu8, 0xf0u8], 4),
            (vec![3, 3, 3, 3, 3, 3], vec![0xffu8, 0xf0u8], 2),
        ];
        for (inputs, want, size) in cases {
            let mut buf = Cursor::new(Vec::new());
            let mut w = BitWriter::new(&mut buf);
            for bits in inputs {
                w.write(bits, size);
            }
            assert!(w.acc_error().is_none());
            w.flush();
            assert!(w.acc_error().is_none());
            assert_eq!(buf.into_inner(), want);
        }
    }
}
