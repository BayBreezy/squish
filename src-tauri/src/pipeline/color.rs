//! Colour management: convert pixels with an embedded ICC profile to sRGB,
//! because the encoders write untagged (i.e. sRGB) files.

use image::RgbaImage;
use moxcms::{ColorProfile, Layout, TransformOptions};
use rayon::prelude::*;

/// Convert `rgba` from the colour space described by `icc` to sRGB, in place.
/// Anything that can't be converted (unknown or unsupported profile) is left
/// untouched rather than failing the file.
pub fn to_srgb(rgba: &mut RgbaImage, icc: &[u8]) {
    let Ok(source) = ColorProfile::new_from_slice(icc) else {
        return;
    };
    // Profiles with sRGB primaries (the vast majority) need no work.
    if has_srgb_primaries(&source) {
        return;
    }
    convert(rgba, &source);
}

/// True when the profile's red/green/blue colorants match sRGB's.
fn has_srgb_primaries(profile: &ColorProfile) -> bool {
    let srgb = ColorProfile::new_srgb();
    let close = |a: moxcms::Xyzd, b: moxcms::Xyzd| {
        (a.x - b.x).abs() < 0.003 && (a.y - b.y).abs() < 0.003 && (a.z - b.z).abs() < 0.003
    };
    close(profile.red_colorant, srgb.red_colorant)
        && close(profile.green_colorant, srgb.green_colorant)
        && close(profile.blue_colorant, srgb.blue_colorant)
}

/// Display P3 → sRGB, for HEIC files that signal P3 by code instead of by profile.
pub fn p3_to_srgb(rgba: &mut RgbaImage) {
    convert(rgba, &ColorProfile::new_display_p3());
}

fn convert(rgba: &mut RgbaImage, source: &ColorProfile) {
    let srgb = ColorProfile::new_srgb();
    let Ok(transform) = source.create_transform_8bit(
        Layout::Rgba,
        &srgb,
        Layout::Rgba,
        TransformOptions::default(),
    ) else {
        return;
    };
    let row_bytes = rgba.width() as usize * 4;
    if row_bytes == 0 {
        return;
    }
    let buf: &mut [u8] = rgba;
    buf.par_chunks_mut(row_bytes).for_each(|row| {
        let src = row.to_vec();
        let _ = transform.transform(&src, row);
    });
}
