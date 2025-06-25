use image::{RgbImage, Rgb};
use super::sps::Sps;

/// Decode an IDR slice to RGB. This is a stub implementation which
/// simply returns a black frame with the size specified in the SPS.
/// A full H.264 decoder is outside the scope of this example.
pub fn decode_idr_to_rgb(_nalus: &[Vec<u8>], sps: &Sps) -> RgbImage {
    let width = sps.width as u32;
    let height = sps.height as u32;
    RgbImage::from_pixel(width, height, Rgb([0, 0, 0]))
}
