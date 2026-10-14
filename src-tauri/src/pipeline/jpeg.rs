use std::panic::{AssertUnwindSafe, catch_unwind};

use image::RgbaImage;

use crate::{error::AppError, types::Settings};

pub fn parse_hex(color: &str) -> [u8; 3] {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() == 6
        && let Ok(v) = u32::from_str_radix(hex, 16)
    {
        return [(v >> 16) as u8, (v >> 8) as u8, v as u8];
    }
    [255, 255, 255]
}

/// JPEG has no alpha: composite onto the chosen background colour.
pub fn flatten(img: &RgbaImage, bg: [u8; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(img.width() as usize * img.height() as usize * 3);
    for p in img.pixels() {
        let a = p[3] as u32;
        for c in 0..3 {
            let v = (p[c] as u32 * a + bg[c] as u32 * (255 - a) + 127) / 255;
            out.push(v as u8);
        }
    }
    out
}

pub fn encode(img: &RgbaImage, s: &Settings) -> Result<Vec<u8>, AppError> {
    let rgb = flatten(img, parse_hex(&s.background));
    let (w, h) = (img.width() as usize, img.height() as usize);

    // mozjpeg is a C library that reports errors by panicking; contain that so
    // one bad image can't take down the whole batch.
    let result = catch_unwind(AssertUnwindSafe(|| -> std::io::Result<Vec<u8>> {
        let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        comp.set_size(w, h);
        comp.set_quality(s.quality as f32);
        if s.jpeg_progressive {
            comp.set_progressive_mode();
        }
        comp.set_optimize_scans(s.jpeg_progressive);
        if s.jpeg_chroma_444 {
            comp.set_chroma_sampling_pixel_sizes((1, 1), (1, 1));
        }
        let mut started = comp.start_compress(Vec::new())?;
        started.write_scanlines(&rgb)?;
        started.finish()
    }));

    match result {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(e)) => Err(AppError::Encode(e.to_string())),
        Err(_) => Err(AppError::Encode("JPEG encoder failed".into())),
    }
}
