/// Utilities for AVC related MIME types.
use super::Sps;

/// Return the codec string suitable for the `codecs` parameter in MIME types.
/// Equivalent to Go's `CodecString` helper.
pub fn codec_string(sample_entry: &str, sps: &Sps) -> String {
    format!(
        "{sample_entry}.{:02X}{:02X}{:02X}",
        sps.profile,
        sps.profile_compatibility,
        sps.level
    )
}
