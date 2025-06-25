//! Minimal OpenH264 decoder wrapper implemented directly in this file.

use std::os::raw::{
    c_char, c_int, c_long, c_uchar, c_uint, c_void,
};

// ---------------------------------------------------------------------------
// FFI definitions extracted from `openh264-sys2`
//
//


impl Default for SDecodingParam {
    fn default() -> Self {
        Self {
            pFileNameRestructed: std::ptr::null_mut(),
            uiCpuLoad: 0,
            uiTargetDqLayer: 0,
            eEcActiveIdc: 0,
            bParseOnly: false,
            sVideoProperty: SVideoProperty::default(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct SVideoProperty {
    pub size: c_uint,
    pub eVideoBsType: c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SDecodingParam {
    pub pFileNameRestructed: *mut c_char,
    pub uiCpuLoad: c_uint,
    pub uiTargetDqLayer: c_uchar,
    pub eEcActiveIdc: c_int,
    pub bParseOnly: bool,
    pub sVideoProperty: SVideoProperty,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct SSysMEMBuffer {
    pub iWidth: c_int,
    pub iHeight: c_int,
    pub iFormat: c_int,
    pub iStride: [c_int; 2],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union BufferInfoData {
    pub sSystemBuffer: SSysMEMBuffer,
}

impl Default for BufferInfoData {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SBufferInfo {
    pub iBufferStatus: c_int,
    pub uiInBsTimeStamp: u64,
    pub uiOutYuvTimeStamp: u64,
    pub UsrData: BufferInfoData,
    pub pDst: [*mut c_uchar; 3],
}

impl Default for SBufferInfo {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub const DECODER_OPTION_TRACE_LEVEL: c_int = 9;
pub type DECODER_OPTION = c_int;

pub type ISVCDecoder = *const ISVCDecoderVtbl;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ISVCDecoderVtbl {
    pub Initialize: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            pParam: *const SDecodingParam,
        ) -> c_long,
    >,
    pub Uninitialize: Option<unsafe extern "C" fn(arg1: *mut ISVCDecoder) -> c_long>,
    pub DecodeFrame: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            pSrc: *const c_uchar,
            iSrcLen: c_int,
            ppDst: *mut *mut c_uchar,
            pStride: *mut c_int,
            iWidth: *mut c_int,
            iHeight: *mut c_int,
        ) -> c_int,
    >,
    pub DecodeFrameNoDelay: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            pSrc: *const c_uchar,
            iSrcLen: c_int,
            ppDst: *mut *mut c_uchar,
            pDstInfo: *mut SBufferInfo,
        ) -> c_int,
    >,
    pub DecodeFrame2: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            pSrc: *const c_uchar,
            iSrcLen: c_int,
            ppDst: *mut *mut c_uchar,
            pDstInfo: *mut SBufferInfo,
        ) -> c_int,
    >,
    pub FlushFrame: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            ppDst: *mut *mut c_uchar,
            pDstInfo: *mut SBufferInfo,
        ) -> c_int,
    >,
    pub DecodeParser: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            pSrc: *const c_uchar,
            iSrcLen: c_int,
            pDstInfo: *mut c_void,
        ) -> c_int,
    >,
    pub DecodeFrameEx: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            pSrc: *const c_uchar,
            iSrcLen: c_int,
            pDst: *mut c_uchar,
            iDstStride: c_int,
            iDstLen: *mut c_int,
            iWidth: *mut c_int,
            iHeight: *mut c_int,
            iColorFormat: *mut c_int,
        ) -> c_int,
    >,
    pub SetOption: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            eOptionId: DECODER_OPTION,
            pOption: *mut c_void,
        ) -> c_long,
    >,
    pub GetOption: Option<
        unsafe extern "C" fn(
            arg1: *mut ISVCDecoder,
            eOptionId: DECODER_OPTION,
            pOption: *mut c_void,
        ) -> c_long,
    >,
}

extern "C" {
    fn WelsCreateDecoder(ppDecoder: *mut *mut ISVCDecoder) -> c_long;
    fn WelsDestroyDecoder(pDecoder: *mut ISVCDecoder);
}

/// Minimal API wrapper that calls the linked OpenH264 C functions.
pub struct DynamicAPI;

impl DynamicAPI {
    pub const fn from_source() -> Self {
        Self
    }

    pub unsafe fn WelsCreateDecoder(&self, pp: *mut *mut ISVCDecoder) -> c_long {
        WelsCreateDecoder(pp)
    }

    pub unsafe fn WelsDestroyDecoder(&self, p: *mut ISVCDecoder) {
        WelsDestroyDecoder(p)
    }
}


/// Simple error wrapper used by the examples.
#[derive(Debug)]
pub struct H264Error(Box<dyn std::error::Error>);


impl<E> From<E> for H264Error
where
    E: std::error::Error + 'static,
    E: 'static,
    E: std::fmt::Debug + std::fmt::Display,
    E: Send + Sync,
    E: std::any::Any,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    E: std::cmp::PartialEq + std::cmp::Eq,
    E: std::marker::Sized,
    E: std::ops::Not<Output = bool>, // hack para evitar recursão reflexiva
{
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

impl std::fmt::Display for H264Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for H264Error {}

impl H264Error {
    pub fn msg(msg: &str) -> Self {
        Self(msg.to_string().into())
    }
}

/// Minimal OpenH264 decoder wrapper.
pub struct Decoder {
    api: DynamicAPI,
    inner: *mut *const ISVCDecoderVtbl,
}

impl Decoder {
    /// Create a new decoder using the built in OpenH264 source.
    pub fn new() -> Result<Self, H264Error> {
        let api = DynamicAPI::from_source();
        unsafe {
            let mut ptr = std::ptr::null::<ISVCDecoderVtbl>() as *mut *const ISVCDecoderVtbl;
            let rv = api.WelsCreateDecoder(std::ptr::from_mut(&mut ptr));
            if rv != 0 {
                return Err(H264Error::msg("WelsCreateDecoder failed"));
            }
            let params = SDecodingParam::default();
            let init = (*(*ptr)).Initialize.ok_or_else(|| H264Error::msg("Missing Initialize"))?;
            if init(ptr as *mut ISVCDecoder, &params as *const _) != 0 {
                return Err(H264Error::msg("Initialize failed"));
            }
            // Quiet logging.
            if let Some(set_opt) = (*(*ptr)).SetOption {
                let mut level: c_int = 0; // WELS_LOG_QUIET
                set_opt(ptr as *mut ISVCDecoder, DECODER_OPTION_TRACE_LEVEL, &mut level as *mut _ as *mut c_void);
            }
            Ok(Self { api, inner: ptr })
        }
    }

    /// Decode a single NAL unit and return an image if available.
    pub fn decode<'a>(&mut self, packet: &[u8]) -> Result<Option<DecodedYUV<'a>>, H264Error> {
        unsafe {
            let mut dst = [std::ptr::null_mut::<u8>(); 3];
            let mut info = SBufferInfo::default();
            let decode = (*(*self.inner)).DecodeFrameNoDelay.ok_or_else(|| H264Error::msg("DecodeFrameNoDelay missing"))?;
            decode(self.inner as *mut ISVCDecoder, packet.as_ptr(), packet.len() as c_int, dst.as_mut_ptr(), &mut info);
            if info.iBufferStatus != 0 {
                let buf: SSysMEMBuffer = info.UsrData.sSystemBuffer;
                if dst[0].is_null() || dst[1].is_null() || dst[2].is_null() {
                    return Ok(None);
                }
                let y = std::slice::from_raw_parts(dst[0], (buf.iHeight * buf.iStride[0]) as usize);
                let u = std::slice::from_raw_parts(dst[1], (buf.iHeight * buf.iStride[1] / 2) as usize);
                let v = std::slice::from_raw_parts(dst[2], (buf.iHeight * buf.iStride[1] / 2) as usize);
                Ok(Some(DecodedYUV { info: buf, y, u, v }))
            } else {
                Ok(None)
            }
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            if let Some(uninit) = (*(*self.inner)).Uninitialize {
                uninit(self.inner as *mut ISVCDecoder);
            }
            self.api.WelsDestroyDecoder(self.inner);
        }
    }
}

/// Decoded YUV image returned from the decoder.
pub struct DecodedYUV<'a> {
    pub info: SSysMEMBuffer,
    pub y: &'a [u8],
    pub u: &'a [u8],
    pub v: &'a [u8],
}

impl<'a> DecodedYUV<'a> {
    pub fn dimensions(&self) -> (usize, usize) {
        (self.info.iWidth as usize, self.info.iHeight as usize)
    }

    pub fn write_rgb8(&self, target: &mut [u8]) {
        let (w, h) = self.dimensions();
        let stride_y = self.info.iStride[0] as usize;
        let stride_uv = self.info.iStride[1] as usize;
        for j in 0..h {
            for i in 0..w {
                let yv = self.y[j * stride_y + i] as f32;
                let uv_index = (j / 2) * stride_uv + (i / 2);
                let u = self.u[uv_index] as f32;
                let v = self.v[uv_index] as f32;

                let c = (yv - 16.0) * 1.164;
                let d = u - 128.0;
                let e = v - 128.0;

                let r = (c + 1.596 * e).clamp(0.0, 255.0);
                let g = (c - 0.392 * d - 0.813 * e).clamp(0.0, 255.0);
                let b = (c + 2.017 * d).clamp(0.0, 255.0);

                let idx = (j * w + i) * 3;
                target[idx] = r as u8;
                target[idx + 1] = g as u8;
                target[idx + 2] = b as u8;
            }
        }
    }
}
