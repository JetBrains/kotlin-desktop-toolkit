use core::ffi::c_void;

use anyhow::Context;
use windows::Win32::{
    Graphics::{
        Gdi::{BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, HBITMAP, HGDIOBJ},
        Imaging::{
            CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, IWICImagingFactory, WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom,
            WICDecodeMetadataCacheOnDemand,
        },
    },
    System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
};

use super::geometry::PhysicalSize;

// A decoded image as a top-down, straight (non-premultiplied) 32bpp BGRA DIB section. Owns the
// bitmap and frees it on drop unless `into_handle` hands the handle off.
pub(crate) struct WicBitmap {
    bitmap: HBITMAP,
    width: i32,
    height: i32,
}

impl WicBitmap {
    /// Decodes an encoded image into a top-down, straight-alpha 32bpp BGRA DIB section.
    ///
    /// Accepts any encoded image WIC can decode — the codec is selected by sniffing the stream, so
    /// PNG, JPEG, BMP, GIF, TIFF, etc. all work. The result is a top-down DIB (negative `biHeight`)
    /// with straight (non-premultiplied) BGRA, the layout the Shell drag-image helper expects.
    pub(crate) fn decode_from_bytes(image_bytes: &[u8]) -> anyhow::Result<Self> {
        // `InitializeFromMemory` borrows without copying; `image_bytes` outlives every stream/decoder
        // use here (all of which happens before this function returns), so no owned copy is needed.
        let factory: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }.context("WIC factory")?;

        let stream = unsafe { factory.CreateStream() }.context("WIC stream")?;
        unsafe { stream.InitializeFromMemory(image_bytes) }.context("WIC stream init")?;

        let decoder = unsafe { factory.CreateDecoderFromStream(&stream, core::ptr::null(), WICDecodeMetadataCacheOnDemand) }
            .context("image decode")?;
        let frame = unsafe { decoder.GetFrame(0) }.context("image frame")?;

        // Convert to straight (non-premultiplied) BGRA: the Shell drag-image helper multiplies the
        // color channels by alpha itself, so premultiplying here would apply that step twice.
        let converter = unsafe { factory.CreateFormatConverter() }.context("format converter")?;
        unsafe {
            converter.Initialize(
                &frame,
                &GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
        }
        .context("format converter init")?;

        let (mut width, mut height) = (0u32, 0u32);
        unsafe { converter.GetSize(&raw mut width, &raw mut height) }.context("image size")?;
        anyhow::ensure!(width != 0 && height != 0, "image has a zero dimension");
        // Stride is the byte length of one pixel row: 4 bytes per pixel for 32bpp BGRA. A 32bpp row
        // is inherently DWORD-aligned, so the DIB needs no extra end-of-row padding.
        let stride = width.checked_mul(4).context("stride overflow")?;
        let buf_len = (stride as usize).checked_mul(height as usize).context("pixel buffer overflow")?;
        // The BITMAPINFOHEADER takes the dimensions as i32.
        let width_i32 = i32::try_from(width)?;
        let height_i32 = i32::try_from(height)?;

        // Top-down (negative height) 32bpp BI_RGB DIB: the Shell drag-image helper expects top-down.
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>().try_into()?,
                biWidth: width_i32,
                biHeight: -height_i32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = core::ptr::null_mut();
        let bitmap = unsafe { CreateDIBSection(None, &raw const bmi, DIB_RGB_COLORS, &raw mut bits, None, 0) }.context("DIB section")?;
        // From here the returned value owns the bitmap and frees it on any early return below.
        let image = Self {
            bitmap,
            width: width_i32,
            height: height_i32,
        };
        anyhow::ensure!(!bits.is_null(), "DIB section has no backing memory");

        // Copy the converted pixels into the DIB.
        unsafe {
            converter.CopyPixels(
                core::ptr::null(),
                stride,
                core::slice::from_raw_parts_mut(bits.cast::<u8>(), buf_len),
            )
        }
        .context("copy pixels into DIB")?;

        Ok(image)
    }

    // Release ownership without freeing: the handle now belongs to the caller (e.g. an `SHDRAGIMAGE`).
    pub(crate) const fn into_handle(self) -> HBITMAP {
        let handle = self.bitmap;
        core::mem::forget(self);
        handle
    }

    // The image's size in physical pixels.
    pub(crate) const fn size(&self) -> PhysicalSize {
        PhysicalSize::new(self.width, self.height)
    }
}

impl Drop for WicBitmap {
    fn drop(&mut self) {
        let _ = unsafe { DeleteObject(HGDIOBJ(self.bitmap.0)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{
        Foundation::{RPC_E_CHANGED_MODE, S_FALSE},
        Graphics::Gdi::{DIBSECTION, GetObjectW},
        System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
    };

    // 2x2 RGBA PNG. Top-down reading order:
    //   (0,0) half-transparent red   RGBA = (255,   0,   0, 128)
    //   (1,0) opaque green           RGBA = (  0, 255,   0, 255)
    //   (0,1) opaque blue            RGBA = (  0,   0, 255, 255)
    //   (1,1) fully transparent white RGBA = (255, 255, 255,   0)
    // The half-transparent red and fully-transparent white pixels distinguish straight from
    // premultiplied BGRA.
    const TEST_PNG_2X2: [u8; 79] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
        0x00, 0x02, 0x08, 0x06, 0x00, 0x00, 0x00, 0x72, 0xb6, 0x0d, 0x24, 0x00, 0x00, 0x00, 0x16, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63,
        0xf8, 0xcf, 0xc0, 0xd0, 0xc0, 0xf0, 0x1f, 0x08, 0x19, 0x18, 0xfe, 0x83, 0x00, 0x03, 0x00, 0x41, 0xd7, 0x08, 0x79, 0x08, 0xed, 0x0b,
        0x44, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn ensure_com_initialized() {
        // Cargo unit tests have no `OleInitialize`; production relies on application.rs. Tolerate the
        // "already initialized" returns so repeated tests on one thread don't fail.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        assert!(
            hr.is_ok() || hr == S_FALSE || hr == RPC_E_CHANGED_MODE,
            "CoInitializeEx failed: {hr:?}"
        );
    }

    fn read_dibsection(bitmap: HBITMAP) -> DIBSECTION {
        let mut dib = DIBSECTION::default();
        let written = unsafe {
            GetObjectW(
                HGDIOBJ(bitmap.0),
                size_of::<DIBSECTION>().try_into().unwrap(),
                Some((&raw mut dib).cast()),
            )
        };
        assert_eq!(
            usize::try_from(written).unwrap(),
            size_of::<DIBSECTION>(),
            "GetObjectW did not fill a DIBSECTION"
        );
        dib
    }

    #[test]
    fn decode_from_bytes_matches_image_dimensions() {
        ensure_com_initialized();
        let dib = WicBitmap::decode_from_bytes(&TEST_PNG_2X2).unwrap();
        assert_eq!(dib.size(), PhysicalSize::new(2, 2));
    }

    #[test]
    fn decode_from_bytes_is_32bpp() {
        ensure_com_initialized();
        let dib = WicBitmap::decode_from_bytes(&TEST_PNG_2X2).unwrap();
        let section = read_dibsection(dib.bitmap);
        assert_eq!(section.dsBmih.biBitCount, 32);
    }

    #[test]
    fn decode_from_bytes_lays_out_top_down() {
        ensure_com_initialized();
        let dib = WicBitmap::decode_from_bytes(&TEST_PNG_2X2).unwrap();
        let section = read_dibsection(dib.bitmap);
        // GetObjectW reports `dsBmih.biHeight` as the absolute height, so orientation is only
        // observable from pixel memory. For 2x2 @ 32bpp the row stride is 8 bytes; in a top-down DIB
        // the first scanline is the top row (half-transparent red) and the second is the bottom row
        // (opaque blue), distinct in BGRA — a bottom-up DIB would swap them.
        let pixels = unsafe { core::slice::from_raw_parts(section.dsBm.bmBits.cast::<u8>(), 16) };
        let top_left = [pixels[0], pixels[1], pixels[2], pixels[3]];
        let bottom_left = [pixels[8], pixels[9], pixels[10], pixels[11]];
        assert_eq!(top_left, [0, 0, 255, 128], "first scanline must be the top row");
        assert_eq!(bottom_left, [255, 0, 0, 255], "second scanline must be the bottom row");
    }

    #[test]
    fn decode_from_bytes_keeps_straight_alpha() {
        ensure_com_initialized();
        let dib = WicBitmap::decode_from_bytes(&TEST_PNG_2X2).unwrap();
        let section = read_dibsection(dib.bitmap);
        // Sample the bottom-right pixel (fully transparent white, RGBA 255,255,255,0). Straight BGRA
        // keeps the color channels at full intensity; premultiplied alpha (×0) would zero them all.
        // It sits at row 1, column 1: byte offset stride(8) + 4 = 12.
        let pixels = unsafe { core::slice::from_raw_parts(section.dsBm.bmBits.cast::<u8>(), 16) };
        let bottom_right = [pixels[12], pixels[13], pixels[14], pixels[15]];
        assert_eq!(bottom_right, [255, 255, 255, 0], "expected straight (non-premultiplied) BGRA");
    }

    #[test]
    fn decode_from_bytes_with_invalid_bytes_returns_err() {
        ensure_com_initialized();
        let result = WicBitmap::decode_from_bytes(&[0u8, 1, 2, 3, 4, 5, 6, 7]);
        assert!(result.is_err());
    }
}
