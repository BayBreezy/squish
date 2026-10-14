use image::RgbaImage;
use ravif::{Encoder, Img, RGBA8};

use crate::{error::AppError, types::Settings};

pub fn encode(img: &RgbaImage, s: &Settings) -> Result<Vec<u8>, AppError> {
    let pixels: Vec<RGBA8> = img
        .pixels()
        .map(|p| RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let buf = Img::new(pixels.as_slice(), img.width() as usize, img.height() as usize);
    let quality = s.quality as f32;
    let encoded = Encoder::new()
        .with_quality(quality)
        .with_alpha_quality(quality)
        .with_speed(s.avif_speed)
        .encode_rgba(buf)
        .map_err(|e| AppError::Encode(e.to_string()))?;
    Ok(encoded.avif_file)
}
