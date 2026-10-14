use image::RgbaImage;

use crate::{error::AppError, types::Settings};

fn encode_rgba(img: &RgbaImage) -> Result<Vec<u8>, AppError> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width(), img.height());
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut writer = enc.write_header().map_err(|e| AppError::Encode(e.to_string()))?;
        writer
            .write_image_data(img.as_raw())
            .map_err(|e| AppError::Encode(e.to_string()))?;
    }
    Ok(out)
}

/// Palette-quantize with imagequant (the pngquant engine). Returns `None` when
/// the requested minimum quality can't be reached, so the caller falls back to
/// lossless instead of producing an ugly image.
fn quantize(img: &RgbaImage, s: &Settings) -> Result<Option<Vec<u8>>, AppError> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut attr = imagequant::new();
    let max_q = s.quality;
    let min_q = max_q.saturating_sub(25);
    let err = |e: imagequant::Error| AppError::Encode(e.to_string());
    attr.set_quality(min_q, max_q).map_err(err)?;
    attr.set_speed(4).map_err(err)?;
    attr.set_max_colors(s.png_max_colors as u32).map_err(err)?;

    let pixels: Vec<imagequant::RGBA> = img
        .pixels()
        .map(|p| imagequant::RGBA::new(p[0], p[1], p[2], p[3]))
        .collect();
    let mut qimg = attr.new_image(pixels, w, h, 0.0).map_err(err)?;
    let mut res = match attr.quantize(&mut qimg) {
        Ok(r) => r,
        Err(imagequant::Error::QualityTooLow) => return Ok(None),
        Err(e) => return Err(err(e)),
    };
    res.set_dithering_level(1.0).map_err(err)?;
    let (palette, indices) = res.remapped(&mut qimg).map_err(err)?;

    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width(), img.height());
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        enc.set_palette(palette.iter().flat_map(|c| [c.r, c.g, c.b]).collect::<Vec<u8>>());
        if palette.iter().any(|c| c.a != 255) {
            enc.set_trns(palette.iter().map(|c| c.a).collect::<Vec<u8>>());
        }
        let mut writer = enc.write_header().map_err(|e| AppError::Encode(e.to_string()))?;
        writer
            .write_image_data(&indices)
            .map_err(|e| AppError::Encode(e.to_string()))?;
    }
    Ok(Some(out))
}

pub fn encode(img: &RgbaImage, s: &Settings) -> Result<Vec<u8>, AppError> {
    let base = if s.png_quantize && !s.lossless {
        match quantize(img, s)? {
            Some(bytes) => bytes,
            None => encode_rgba(img)?,
        }
    } else {
        encode_rgba(img)?
    };
    // Lossless squeeze (filter selection, deflate tuning, colour-type reduction).
    let opts = oxipng::Options::from_preset(s.png_optimize_level);
    match oxipng::optimize_from_memory(&base, &opts) {
        Ok(smaller) => Ok(smaller),
        Err(_) => Ok(base),
    }
}
