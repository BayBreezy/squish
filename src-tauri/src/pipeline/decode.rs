//! Reading images: any supported format (including HEIC), with colour
//! management and orientation baked into the pixels.

use std::{fs, io::BufReader, path::Path};

use image::{
    AnimationDecoder, DynamicImage, GenericImageView, ImageDecoder, ImageFormat, ImageReader,
    RgbaImage, imageops::FilterType,
};

use super::{color, metadata};
use crate::error::AppError;

/// What an input file was, as far as the pipeline cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Png,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Tiff,
    Heic,
    Other,
}

impl SourceFormat {
    pub fn name(self) -> &'static str {
        match self {
            SourceFormat::Png => "png",
            SourceFormat::Jpeg => "jpeg",
            SourceFormat::WebP => "webp",
            SourceFormat::Gif => "gif",
            SourceFormat::Bmp => "bmp",
            SourceFormat::Tiff => "tiff",
            SourceFormat::Heic => "heic",
            SourceFormat::Other => "other",
        }
    }
}

impl From<ImageFormat> for SourceFormat {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Png => SourceFormat::Png,
            ImageFormat::Jpeg => SourceFormat::Jpeg,
            ImageFormat::WebP => SourceFormat::WebP,
            ImageFormat::Gif => SourceFormat::Gif,
            ImageFormat::Bmp => SourceFormat::Bmp,
            ImageFormat::Tiff => SourceFormat::Tiff,
            _ => SourceFormat::Other,
        }
    }
}

/// A decoded image, ready for resizing and encoding.
pub struct Loaded {
    /// sRGB pixels with EXIF orientation / HEIF transforms already applied.
    pub rgba: RgbaImage,
    pub format: SourceFormat,
    /// Raw EXIF (TIFF block) with its orientation reset to "normal" because the
    /// pixels are already rotated. `None` when the file has none.
    pub exif: Option<Vec<u8>>,
}

/// HEIC/HEIF by extension or by sniffing the `ftyp` box.
pub fn is_heic(path: &Path) -> bool {
    let by_extension = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "heic" | "heif" | "hif"));
    if by_extension {
        return true;
    }
    use std::io::Read;
    let mut head = [0u8; 64];
    let Ok(n) = fs::File::open(path).and_then(|mut f| f.read(&mut head)) else {
        return false;
    };
    heic_rs::io::looks_like_heif(&head[..n])
}

pub fn load(path: &Path) -> Result<Loaded, AppError> {
    if is_heic(path) {
        return load_heic(path);
    }
    load_regular(path)
}

fn load_regular(path: &Path) -> Result<Loaded, AppError> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader
        .format()
        .map(SourceFormat::from)
        .ok_or_else(|| AppError::Unsupported("Unrecognised image format".into()))?;
    let mut decoder = reader.into_decoder()?;
    let icc = decoder.icc_profile().ok().flatten();
    let exif = decoder.exif_metadata().ok().flatten();
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);

    let mut rgba = img.into_rgba8();
    if let Some(icc) = icc {
        color::to_srgb(&mut rgba, &icc);
    }
    Ok(Loaded {
        rgba,
        format,
        exif: exif.as_deref().and_then(metadata::normalise_exif),
    })
}

fn load_heic(path: &Path) -> Result<Loaded, AppError> {
    let bytes = fs::read(path)?;
    match decode_heic(&bytes) {
        Ok(loaded) => Ok(loaded),
        // The pure-Rust decoder is strict about HEVC features it doesn't
        // implement; on macOS the system decoder handles every variant.
        Err(first) => sips_fallback(path).map_err(|_| first),
    }
}

fn decode_heic(bytes: &[u8]) -> Result<Loaded, AppError> {
    let err = |e: heic_rs::Error| AppError::Decode(format!("HEIC: {e}"));
    let options = heic_rs::DecodeOptions::default().with_layout(heic_rs::PixelLayout::Rgba8);
    let image = heic_rs::decode(bytes, &options).map_err(err)?;
    let mut rgba = RgbaImage::from_raw(image.width, image.height, image.data)
        .ok_or_else(|| AppError::Decode("HEIC: unexpected pixel buffer size".into()))?;

    // Colour space and EXIF live on the primary item.
    let mut exif = None;
    if let Ok(ctx) = heic_rs::context::Context::open(bytes) {
        let primary = ctx.meta.primary;
        if let Ok(props) = ctx.props(primary) {
            match (props.icc, props.nclx) {
                (Some(icc), _) => color::to_srgb(&mut rgba, icc),
                // CICP primaries 12 = Display P3 (what iPhones write without ICC).
                (None, Some(nclx)) if nclx.primaries == 12 => color::p3_to_srgb(&mut rgba),
                _ => {}
            }
        }
        exif = ctx
            .exif(primary)
            .ok()
            .flatten()
            .and_then(metadata::normalise_exif);
    }

    Ok(Loaded {
        rgba,
        format: SourceFormat::Heic,
        exif,
    })
}

/// Width and height of a HEIC without decoding the pixels.
pub fn probe_heic(path: &Path) -> Result<(u32, u32), AppError> {
    let bytes = fs::read(path)?;
    match heic_rs::probe(&bytes) {
        Ok(info) => Ok((info.width, info.height)),
        Err(e) => sips_dimensions(path)
            .ok_or_else(|| AppError::Unsupported(format!("Unsupported HEIC variant ({e})"))),
    }
}

/// True for animated GIF / WebP / APNG, which are reduced to their first frame.
pub fn is_animated(path: &Path, format: SourceFormat) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let reader = BufReader::new(file);
    match format {
        SourceFormat::Gif => image::codecs::gif::GifDecoder::new(reader)
            .map(|d| d.into_frames().take(2).count() > 1)
            .unwrap_or(false),
        SourceFormat::WebP => image::codecs::webp::WebPDecoder::new(reader)
            .map(|d| d.has_animation())
            .unwrap_or(false),
        SourceFormat::Png => image::codecs::png::PngDecoder::new(reader)
            .and_then(|d| d.is_apng())
            .unwrap_or(false),
        _ => false,
    }
}

#[cfg(target_os = "macos")]
fn sips_fallback(path: &Path) -> Result<Loaded, AppError> {
    use std::{
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let dir = std::env::temp_dir().join(format!(
        "squish-sips-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir)?;
    let out = dir.join("decoded.png");
    let status = Command::new("/usr/bin/sips")
        .args(["-s", "format", "png"])
        .arg(path)
        .arg("--out")
        .arg(&out)
        .output();
    let result = match status {
        Ok(o) if o.status.success() => load_regular(&out).map(|mut l| {
            l.format = SourceFormat::Heic;
            l
        }),
        _ => Err(AppError::Decode("HEIC: system decoder failed".into())),
    };
    let _ = fs::remove_dir_all(&dir);
    result
}

#[cfg(not(target_os = "macos"))]
fn sips_fallback(_path: &Path) -> Result<Loaded, AppError> {
    Err(AppError::Unsupported("no fallback decoder".into()))
}

#[cfg(target_os = "macos")]
fn sips_dimensions(path: &Path) -> Option<(u32, u32)> {
    let out = std::process::Command::new("/usr/bin/sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |key: &str| -> Option<u32> {
        text.lines()
            .find_map(|l| l.trim().strip_prefix(key))
            .and_then(|v| v.trim_start_matches(':').trim().parse().ok())
    };
    Some((field("pixelWidth")?, field("pixelHeight")?))
}

#[cfg(not(target_os = "macos"))]
fn sips_dimensions(_path: &Path) -> Option<(u32, u32)> {
    None
}

/// Downscale to fit within the given box. Never upscales; keeps aspect ratio.
pub fn fit(img: DynamicImage, max_w: Option<u32>, max_h: Option<u32>) -> DynamicImage {
    let (w, h) = img.dimensions();
    let sx = max_w.map_or(1.0, |m| m as f64 / w as f64);
    let sy = max_h.map_or(1.0, |m| m as f64 / h as f64);
    let scale = sx.min(sy).min(1.0);
    if scale >= 1.0 {
        return img;
    }
    let nw = ((w as f64 * scale).round() as u32).max(1);
    let nh = ((h as f64 * scale).round() as u32).max(1);
    img.resize_exact(nw, nh, FilterType::Lanczos3)
}
