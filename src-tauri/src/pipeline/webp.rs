use std::panic::{AssertUnwindSafe, catch_unwind};

use image::RgbaImage;

use crate::{error::AppError, types::Settings};

pub fn encode(img: &RgbaImage, s: &Settings) -> Result<Vec<u8>, AppError> {
    let (w, h) = (img.width(), img.height());
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut config = webp::WebPConfig::new().map_err(|_| "could not initialise libwebp".to_string())?;
        config.lossless = s.lossless as i32;
        config.quality = s.quality as f32;
        config.method = s.webp_method as i32;
        // Keep colour data under fully transparent pixels only when lossless.
        config.exact = s.lossless as i32;
        let encoder = webp::Encoder::from_rgba(img.as_raw(), w, h);
        encoder
            .encode_advanced(&config)
            .map(|m| m.to_vec())
            .map_err(|e| format!("{e:?}"))
    }));
    match result {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(e)) => Err(AppError::Encode(e)),
        Err(_) => Err(AppError::Encode("WebP encoder failed".into())),
    }
}
