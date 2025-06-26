use super::sps::Sps;

/// Minimal RGB image used only for this stub implementation.
pub struct RgbImage {
    /// Pixel data in RGB format.
    pub data: Vec<u8>,
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
}

impl RgbImage {
    /// Create an image filled with a single RGB color.
    pub fn from_pixel(width: u32, height: u32, pixel: Rgb) -> Self {
        let mut data = Vec::with_capacity((width * height * 3) as usize);
        for _ in 0..(width * height) {
            data.extend_from_slice(&pixel.0);
        }
        Self { data, width, height }
    }
}

/// Simple RGB triplet used by [`RgbImage`].
pub struct Rgb(pub [u8; 3]);

/// Decode an IDR slice to RGB. This is a stub implementation which
/// simply returns a black frame with the size specified in the SPS.
/// A full H.264 decoder is outside the scope of this example.
pub fn decode_idr_to_rgb(_nalus: &[Vec<u8>], sps: &Sps) -> RgbImage {
    let width = sps.width as u32;
    let height = sps.height as u32;
    RgbImage::from_pixel(width, height, Rgb([0, 0, 0]))
}
