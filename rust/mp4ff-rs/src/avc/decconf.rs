
/// AVCDecoderConfigurationRecord extracted from avcC box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecConfRec {
    pub profile_indication: u8,
    pub profile_compatibility: u8,
    pub level_indication: u8,
    pub sps: Vec<Vec<u8>>,
    pub pps: Vec<Vec<u8>>,
}

/// Parse the AVCDecoderConfigurationRecord as defined in ISO/IEC 14496-15.
pub fn decode_avc_decoder_config(data: &[u8]) -> Option<DecConfRec> {
    if data.len() < 6 { return None; }
    if data[0] != 1 { return None; }
    let profile = data[1];
    let compat = data[2];
    let level = data[3];
    let length_size_minus1 = data[4] & 0x03;
    if length_size_minus1 != 3 { return None; }
    let mut pos = 5usize;
    let num_sps = data[pos] & 0x1f;
    pos += 1;
    let mut sps_vec = Vec::new();
    for _ in 0..num_sps {
        if pos + 2 > data.len() { return None; }
        let len = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
        pos += 2;
        if pos + len > data.len() { return None; }
        sps_vec.push(data[pos..pos+len].to_vec());
        pos += len;
    }
    if pos >= data.len() { return None; }
    let num_pps = data[pos];
    pos += 1;
    let mut pps_vec = Vec::new();
    for _ in 0..num_pps {
        if pos + 2 > data.len() { return None; }
        let len = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
        pos += 2;
        if pos + len > data.len() { return None; }
        pps_vec.push(data[pos..pos+len].to_vec());
        pos += len;
    }
    Some(DecConfRec {
        profile_indication: profile,
        profile_compatibility: compat,
        level_indication: level,
        sps: sps_vec,
        pps: pps_vec,
    })
}

impl DecConfRec {
    /// Return the size in bytes when encoded.
    pub fn size(&self) -> usize {
        let mut total = 7usize;
        for n in &self.sps { total += 2 + n.len(); }
        for n in &self.pps { total += 2 + n.len(); }
        total
    }

    /// Encode the configuration record and return the byte vector.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.size());
        out.push(1); // configurationVersion
        out.push(self.profile_indication);
        out.push(self.profile_compatibility);
        out.push(self.level_indication);
        out.push(0xff); // length size minus 1 == 3
        out.push(0xe0 | (self.sps.len() as u8));
        for n in &self.sps {
            out.extend_from_slice(&(n.len() as u16).to_be_bytes());
            out.extend_from_slice(n);
        }
        out.push(self.pps.len() as u8);
        for n in &self.pps {
            out.extend_from_slice(&(n.len() as u16).to_be_bytes());
            out.extend_from_slice(n);
        }
        out
    }
}
