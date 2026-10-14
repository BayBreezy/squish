//! EXIF handling for the optional "keep metadata" setting.

use img_parts::{Bytes, DynImage, ImageEXIF};

use crate::types::OutputFormat;

/// Accept a raw TIFF block (optionally prefixed with `Exif\0\0`) and return it
/// with the Orientation tag reset to 1. Orientation is baked into the pixels,
/// so leaving the original value would rotate the image twice.
pub fn normalise_exif(raw: &[u8]) -> Option<Vec<u8>> {
    let tiff = raw.strip_prefix(b"Exif\0\0").unwrap_or(raw);
    let valid = tiff.len() >= 8 && (tiff.starts_with(b"II*\0") || tiff.starts_with(b"MM\0*"));
    if !valid {
        return None;
    }
    let mut out = tiff.to_vec();
    reset_orientation(&mut out);
    Some(out)
}

fn reset_orientation(tiff: &mut [u8]) {
    let little = tiff.starts_with(b"II");
    let read16 = |b: &[u8], o: usize| -> Option<u16> {
        let s: [u8; 2] = b.get(o..o + 2)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(s)
        } else {
            u16::from_be_bytes(s)
        })
    };
    let read32 = |b: &[u8], o: usize| -> Option<u32> {
        let s: [u8; 4] = b.get(o..o + 4)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(s)
        } else {
            u32::from_be_bytes(s)
        })
    };
    let Some(ifd) = read32(tiff, 4).map(|v| v as usize) else {
        return;
    };
    let Some(count) = read16(tiff, ifd) else {
        return;
    };
    for i in 0..count as usize {
        let entry = ifd + 2 + i * 12;
        if read16(tiff, entry) == Some(0x0112) {
            let one = if little {
                1u16.to_le_bytes()
            } else {
                1u16.to_be_bytes()
            };
            if let Some(slot) = tiff.get_mut(entry + 8..entry + 10) {
                slot.copy_from_slice(&one);
            }
            return;
        }
    }
}

/// Write `exif` into an already-encoded PNG, JPEG or WebP. Other formats (AVIF)
/// and any failure leave the encoded bytes unchanged.
pub fn embed_exif(encoded: Vec<u8>, format: OutputFormat, exif: &[u8]) -> Vec<u8> {
    if !matches!(
        format,
        OutputFormat::Png | OutputFormat::Jpeg | OutputFormat::Webp
    ) {
        return encoded;
    }
    let bytes = Bytes::from(encoded);
    let Ok(Some(mut image)) = DynImage::from_bytes(bytes.clone()) else {
        return bytes.to_vec();
    };
    image.set_exif(Some(Bytes::copy_from_slice(exif)));
    let mut out = Vec::with_capacity(bytes.len() + exif.len() + 64);
    match image.encoder().write_to(&mut out) {
        Ok(_) => out,
        Err(_) => bytes.to_vec(),
    }
}
