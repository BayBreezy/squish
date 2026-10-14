use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, atomic::AtomicBool},
};

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};

use super::*;

/// Deterministic "photo-like" content: smooth gradients plus pseudo-noise.
fn photo(w: u32, h: u32) -> RgbaImage {
    let mut seed = 0x1234_5678u32;
    ImageBuffer::from_fn(w, h, |x, y| {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let n = ((seed >> 24) as i32 % 16) - 8;
        let r = (x * 255 / w) as i32 + n;
        let g = (y * 255 / h) as i32 + n;
        let b = ((x + y) * 255 / (w + h)) as i32 + n;
        Rgba([r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8, 255])
    })
}

/// Flat UI-style graphic with a transparent corner.
fn graphic(w: u32, h: u32) -> RgbaImage {
    ImageBuffer::from_fn(w, h, |x, y| {
        if x < w / 4 && y < h / 4 {
            Rgba([0, 0, 0, 0])
        } else if (x / 16 + y / 16) % 2 == 0 {
            Rgba([244, 63, 94, 255])
        } else {
            Rgba([30, 41, 59, 255])
        }
    })
}

fn save(img: &RgbaImage, dir: &Path, name: &str, fmt: ImageFormat) -> PathBuf {
    let path = dir.join(name);
    let dynimg = DynamicImage::ImageRgba8(img.clone());
    match fmt {
        ImageFormat::Jpeg => dynimg.to_rgb8().save_with_format(&path, fmt).unwrap(),
        _ => dynimg.save_with_format(&path, fmt).unwrap(),
    }
    path
}

fn with_formats(formats: &[OutputFormat]) -> Settings {
    Settings {
        formats: formats.to_vec(),
        avif_speed: 10,
        ..Settings::default()
    }
}

/// What happened to one input: written outputs and skipped formats.
#[derive(Debug)]
struct Out {
    format: OutputFormat,
    output_path: PathBuf,
    original_bytes: u64,
    output_bytes: u64,
    kept_original: bool,
}

#[derive(Default, Debug)]
struct Run {
    outputs: Vec<Out>,
    skipped: Vec<OutputFormat>,
    failed: Vec<OutputFormat>,
}

fn collect(events: Vec<ProcessEvent>) -> Run {
    let mut run = Run::default();
    for e in events {
        match e {
            ProcessEvent::Output {
                format,
                output_path,
                original_bytes,
                output_bytes,
                kept_original,
                ..
            } => run.outputs.push(Out {
                format,
                output_path: PathBuf::from(output_path),
                original_bytes,
                output_bytes,
                kept_original,
            }),
            ProcessEvent::Skipped { format, .. } => run.skipped.push(format),
            ProcessEvent::OutputFailed { format, .. } => run.failed.push(format),
            _ => {}
        }
    }
    run
}

fn run_one(path: &Path, s: &Settings) -> Run {
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    process_one(path, s, &cancel, &|e| events.lock().unwrap().push(e)).unwrap();
    collect(events.into_inner().unwrap())
}

fn only(run: Run) -> Out {
    assert_eq!(run.outputs.len(), 1, "expected exactly one output: {run:?}");
    run.outputs.into_iter().next().unwrap()
}

#[test]
fn photo_png_to_every_format() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(320, 240), dir.path(), "photo.png", ImageFormat::Png);
    let src_bytes = fs::metadata(&src).unwrap().len();

    for (fmt, ext) in [
        (OutputFormat::Jpeg, "jpg"),
        (OutputFormat::Webp, "webp"),
        (OutputFormat::Avif, "avif"),
        (OutputFormat::Png, "png"),
    ] {
        let out = only(run_one(&src, &with_formats(&[fmt])));
        assert_eq!(out.output_path.extension().unwrap(), ext);
        assert!(out.output_path.exists());
        assert!(out.output_bytes > 0);
        if fmt != OutputFormat::Png {
            // lossy codecs must beat a lossless PNG of a noisy photo
            assert!(out.output_bytes < src_bytes, "{ext}: {} >= {src_bytes}", out.output_bytes);
        }
        if fmt != OutputFormat::Avif {
            let back = image::open(&out.output_path).unwrap();
            assert_eq!((back.width(), back.height()), (320, 240), "{ext}");
        }
    }
}

#[test]
fn several_formats_from_one_input() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(200, 150), dir.path(), "multi.png", ImageFormat::Png);
    let s = with_formats(&[OutputFormat::Avif, OutputFormat::Webp, OutputFormat::Jpeg]);
    let run = run_one(&src, &s);

    assert!(run.failed.is_empty(), "{run:?}");
    let formats: Vec<_> = run.outputs.iter().map(|o| o.format).collect();
    // cheap codecs first, regardless of selection order
    assert_eq!(formats, [OutputFormat::Jpeg, OutputFormat::Webp, OutputFormat::Avif]);
    let paths: std::collections::HashSet<_> = run.outputs.iter().map(|o| &o.output_path).collect();
    assert_eq!(paths.len(), 3);
    assert!(run.outputs.iter().all(|o| o.output_path.exists()));
    assert!(src.exists());
}

#[test]
fn original_plus_same_codec_is_one_output() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(96, 96), dir.path(), "same.webp", ImageFormat::WebP);
    let run = run_one(&src, &with_formats(&[OutputFormat::Original, OutputFormat::Webp]));
    assert_eq!(run.outputs.len(), 1, "{run:?}");
    assert_eq!(run.outputs[0].format, OutputFormat::Webp);
}

#[test]
fn larger_conversions_are_skipped_unless_included() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(160, 120), dir.path(), "snap.jpg", ImageFormat::Jpeg);

    // A lossless PNG of a noisy JPEG photo is far bigger than the JPEG.
    let run = run_one(&src, &with_formats(&[OutputFormat::Png]));
    assert!(run.outputs.is_empty(), "{run:?}");
    assert_eq!(run.skipped, [OutputFormat::Png]);
    assert!(!dir.path().join("snap-squished.png").exists());

    let s = Settings { include_larger: true, ..with_formats(&[OutputFormat::Png]) };
    let out = only(run_one(&src, &s));
    assert!(out.output_bytes > out.original_bytes);
    assert!(out.output_path.exists());
}

#[test]
fn quantized_png_is_smaller_and_keeps_transparency() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&graphic(256, 256), dir.path(), "ui.png", ImageFormat::Png);
    let out = only(run_one(&src, &Settings::default()));
    let back = image::open(&out.output_path).unwrap().to_rgba8();
    assert_eq!(back.get_pixel(0, 0)[3], 0, "transparent corner must stay transparent");
    assert_eq!(back.get_pixel(200, 200)[3], 255);
    assert!(out.output_bytes <= out.original_bytes);
}

#[test]
fn never_grows_when_format_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&graphic(64, 64), dir.path(), "tiny.png", ImageFormat::Png);
    // First pass shrinks it; feed the result back in.
    let first = only(run_one(&src, &Settings::default()));
    let again = only(run_one(&first.output_path, &Settings::default()));
    assert!(again.output_bytes <= again.original_bytes);
    assert!(again.kept_original || again.output_bytes < again.original_bytes);
}

#[test]
fn resize_never_upscales() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(200, 100), dir.path(), "p.png", ImageFormat::Png);
    let s = Settings { max_width: Some(100), ..with_formats(&[OutputFormat::Jpeg]) };
    let back = image::open(&only(run_one(&src, &s)).output_path).unwrap();
    assert_eq!((back.width(), back.height()), (100, 50));

    let s = Settings { max_width: Some(5000), ..with_formats(&[OutputFormat::Jpeg]) };
    let back = image::open(&only(run_one(&src, &s)).output_path).unwrap();
    assert_eq!((back.width(), back.height()), (200, 100));
}

#[test]
fn exif_orientation_is_applied() {
    let dir = tempfile::tempdir().unwrap();
    let plain = save(&photo(40, 20), dir.path(), "plain.jpg", ImageFormat::Jpeg);
    let mut bytes = fs::read(&plain).unwrap();
    // APP1/Exif segment with Orientation = 6 (rotate 90° CW), spliced after SOI.
    let exif: [u8; 36] = [
        0xFF, 0xE1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0, 0x2A, 0, 0, 0, 8, 0, 1,
        0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0, 0, 0, 0, 0,
    ];
    bytes.splice(2..2, exif);
    let rotated = dir.path().join("rotated.jpg");
    fs::write(&rotated, bytes).unwrap();

    let s = Settings { quality: 90, ..Settings::default() };
    let back = image::open(&only(run_one(&rotated, &s)).output_path).unwrap();
    assert_eq!((back.width(), back.height()), (20, 40));
}

#[test]
fn does_not_overwrite_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(64, 64), dir.path(), "a.png", ImageFormat::Png);
    let s = with_formats(&[OutputFormat::Webp]);
    let first = only(run_one(&src, &s));
    let second = only(run_one(&src, &s));
    assert_ne!(first.output_path, second.output_path);
    assert!(first.output_path.exists() && second.output_path.exists());
    assert!(src.exists());
}

#[test]
fn parallel_files_with_the_same_stem_get_distinct_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut inputs = Vec::new();
    for i in 0..6 {
        // Same stem, different source extension, same target name.
        let (name, fmt) = if i % 2 == 0 {
            (format!("x{}.png", i / 2), ImageFormat::Png)
        } else {
            (format!("x{}.jpg", i / 2), ImageFormat::Jpeg)
        };
        inputs.push(save(&photo(48, 48), dir.path(), &name, fmt));
    }
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    run_batch(&inputs, &with_formats(&[OutputFormat::Webp]), &cancel, |e| {
        events.lock().unwrap().push(e)
    });
    let run = collect(events.into_inner().unwrap());
    assert_eq!(run.outputs.len(), 6, "{run:?}");
    let unique: std::collections::HashSet<_> = run.outputs.iter().map(|o| &o.output_path).collect();
    assert_eq!(unique.len(), 6, "output names collided");
    assert!(run.outputs.iter().all(|o| o.output_path.exists()));
}

#[test]
fn cancel_stops_before_encoding_more_formats() {
    let dir = tempfile::tempdir().unwrap();
    let src = save(&photo(64, 64), dir.path(), "c.png", ImageFormat::Png);
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(true);
    let s = with_formats(&[OutputFormat::Jpeg, OutputFormat::Webp]);
    process_one(&src, &s, &cancel, &|e| events.lock().unwrap().push(e)).unwrap();
    assert!(collect(events.into_inner().unwrap()).outputs.is_empty());
}

#[test]
fn bad_file_fails_without_aborting_batch() {
    let dir = tempfile::tempdir().unwrap();
    let good = save(&photo(32, 32), dir.path(), "ok.png", ImageFormat::Png);
    let bad = dir.path().join("broken.png");
    fs::write(&bad, b"definitely not a png").unwrap();

    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    let s = with_formats(&[OutputFormat::Jpeg]);
    run_batch(&[bad.clone(), good.clone()], &s, &cancel, |e| events.lock().unwrap().push(e));
    let events = events.into_inner().unwrap();
    assert!(events.iter().any(|e| matches!(e, ProcessEvent::Failed { .. })));
    assert!(events.iter().any(|e| matches!(e, ProcessEvent::Done { .. })));
    assert!(events.iter().any(|e| matches!(e, ProcessEvent::Output { .. })));
}

#[test]
fn inspect_expands_folders_and_skips_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    save(&photo(10, 10), dir.path(), "x.png", ImageFormat::Png);
    fs::write(dir.path().join("notes.txt"), "hi").unwrap();
    fs::write(dir.path().join(".DS_Store"), "hidden").unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    save(&photo(12, 8), &sub, "y.jpg", ImageFormat::Jpeg);
    fs::write(sub.join("doc.pdf"), "pdf").unwrap();

    let r = inspect(&[dir.path().to_string_lossy().into_owned()]);
    assert_eq!(r.files.len(), 2);
    // notes.txt and doc.pdf are counted; the hidden file is not.
    assert_eq!(r.ignored, 2);
    let txt = dir.path().join("notes.txt").to_string_lossy().into_owned();
    let r2 = inspect(&[txt]);
    assert_eq!(r2.skipped.len(), 1);
}

#[test]
fn settings_sanitize_formats() {
    let empty = Settings { formats: vec![], ..Settings::default() }.sanitized();
    assert_eq!(empty.formats, [OutputFormat::Original]);

    let dup = Settings {
        formats: vec![OutputFormat::Webp, OutputFormat::Png, OutputFormat::Webp],
        ..Settings::default()
    }
    .sanitized();
    assert_eq!(dup.formats, [OutputFormat::Webp, OutputFormat::Png]);
}

// ---------------------------------------------------------------------------
// Colour management, metadata, animation, HEIC
// ---------------------------------------------------------------------------

use img_parts::{Bytes, DynImage, ImageEXIF, ImageICC};

/// A 40×20 JPEG whose EXIF says "rotate 90° CW" (so it displays as 20×40).
fn orientation6_jpeg(dir: &Path) -> PathBuf {
    let plain = save(&photo(40, 20), dir, "o6.jpg", ImageFormat::Jpeg);
    let mut bytes = fs::read(&plain).unwrap();
    let exif: [u8; 36] = [
        0xFF, 0xE1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0, 0x2A, 0, 0, 0, 8, 0, 1,
        0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0, 0, 0, 0, 0,
    ];
    bytes.splice(2..2, exif);
    let out = dir.join("rotated6.jpg");
    fs::write(&out, bytes).unwrap();
    out
}

/// Orientation value (tag 0x0112) found in a raw EXIF TIFF block.
fn exif_orientation(tiff: &[u8]) -> Option<u16> {
    let le = tiff.starts_with(b"II");
    let r16 = |o: usize| -> u16 {
        let s = [tiff[o], tiff[o + 1]];
        if le { u16::from_le_bytes(s) } else { u16::from_be_bytes(s) }
    };
    let r32 = |o: usize| -> u32 {
        let s = [tiff[o], tiff[o + 1], tiff[o + 2], tiff[o + 3]];
        if le { u32::from_le_bytes(s) } else { u32::from_be_bytes(s) }
    };
    let ifd = r32(4) as usize;
    (0..r16(ifd) as usize)
        .map(|i| ifd + 2 + i * 12)
        .find(|&e| r16(e) == 0x0112)
        .map(|e| r16(e + 8))
}

fn exif_of(path: &Path) -> Option<Vec<u8>> {
    let image = DynImage::from_bytes(Bytes::from(fs::read(path).unwrap())).ok()??;
    image.exif().map(|b| b.to_vec())
}

#[test]
fn metadata_is_stripped_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let src = orientation6_jpeg(dir.path());
    let out = only(run_one(&src, &Settings::default()));
    assert!(exif_of(&out.output_path).is_none(), "EXIF should be stripped");
}

#[test]
fn kept_metadata_has_normal_orientation_in_every_format() {
    let dir = tempfile::tempdir().unwrap();
    let src = orientation6_jpeg(dir.path());
    let s = Settings {
        keep_metadata: true,
        include_larger: true,
        ..with_formats(&[OutputFormat::Jpeg, OutputFormat::Png, OutputFormat::Webp])
    };
    let run = run_one(&src, &s);
    assert_eq!(run.outputs.len(), 3, "{run:?}");
    for out in &run.outputs {
        let exif = exif_of(&out.output_path)
            .unwrap_or_else(|| panic!("{:?} lost its EXIF", out.format));
        assert_eq!(
            exif_orientation(&exif),
            Some(1),
            "{:?}: pixels are already rotated, so orientation must read 'normal'",
            out.format
        );
        let img = image::open(&out.output_path).unwrap();
        assert_eq!((img.width(), img.height()), (20, 40), "{:?}", out.format);
    }
}

#[test]
fn display_p3_pixels_are_converted_to_srgb() {
    let dir = tempfile::tempdir().unwrap();
    let flat = RgbaImage::from_pixel(16, 16, Rgba([200, 100, 50, 255]));
    let plain = save(&flat, dir.path(), "p3-plain.png", ImageFormat::Png);

    // Tag the PNG with a Display P3 profile.
    let icc = moxcms::ColorProfile::new_display_p3().encode().unwrap();
    let mut png = DynImage::from_bytes(Bytes::from(fs::read(&plain).unwrap()))
        .unwrap()
        .unwrap();
    png.set_icc_profile(Some(Bytes::from(icc)));
    let src = dir.path().join("p3.png");
    let mut file = Vec::new();
    png.encoder().write_to(&mut file).unwrap();
    fs::write(&src, file).unwrap();

    let s = Settings {
        lossless: true,
        include_larger: true,
        ..with_formats(&[OutputFormat::Webp])
    };
    let out = only(run_one(&src, &s));
    let px = *image::open(&out.output_path).unwrap().to_rgba8().get_pixel(8, 8);
    let delta = (0..3).map(|c| (px[c] as i32 - [200, 100, 50][c]).abs()).max().unwrap();
    assert!(delta >= 6, "expected a P3→sRGB shift, got {px:?}");
    // The converted file is plain sRGB: no embedded profile any more.
    let written = DynImage::from_bytes(Bytes::from(fs::read(&out.output_path).unwrap()))
        .unwrap()
        .unwrap();
    assert!(written.icc_profile().is_none());
}

#[test]
fn srgb_tagged_images_are_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let flat = RgbaImage::from_pixel(16, 16, Rgba([200, 100, 50, 255]));
    let plain = save(&flat, dir.path(), "srgb-plain.png", ImageFormat::Png);
    let icc = moxcms::ColorProfile::new_srgb().encode().unwrap();
    let mut png = DynImage::from_bytes(Bytes::from(fs::read(&plain).unwrap()))
        .unwrap()
        .unwrap();
    png.set_icc_profile(Some(Bytes::from(icc)));
    let src = dir.path().join("srgb.png");
    let mut file = Vec::new();
    png.encoder().write_to(&mut file).unwrap();
    fs::write(&src, file).unwrap();

    let s = Settings { lossless: true, include_larger: true, ..with_formats(&[OutputFormat::Webp]) };
    let out = only(run_one(&src, &s));
    let px = *image::open(&out.output_path).unwrap().to_rgba8().get_pixel(8, 8);
    assert_eq!([px[0], px[1], px[2]], [200, 100, 50]);
}

#[test]
fn animated_images_are_flagged() {
    use image::codecs::gif::GifEncoder;
    let dir = tempfile::tempdir().unwrap();

    let animated = dir.path().join("anim.gif");
    {
        let mut enc = GifEncoder::new(fs::File::create(&animated).unwrap());
        let frames = (0..3).map(|i| {
            let shade = 60 * i as u8;
            image::Frame::new(RgbaImage::from_pixel(8, 8, Rgba([shade, 10, 10, 255])))
        });
        enc.encode_frames(frames).unwrap();
    }
    let still = save(&photo(8, 8), dir.path(), "still.gif", ImageFormat::Gif);

    let r = inspect(&[
        animated.to_string_lossy().into_owned(),
        still.to_string_lossy().into_owned(),
    ]);
    assert_eq!(r.files.len(), 2, "{r:?}");
    let by_name = |n: &str| r.files.iter().find(|f| f.name == n).unwrap();
    assert!(by_name("anim.gif").animated);
    assert!(!by_name("still.gif").animated);

    // It is still processed (first frame only), not rejected.
    let out = only(run_one(&animated, &Settings::default()));
    assert!(out.output_path.exists());
}

#[test]
fn preview_returns_web_formats_untouched_and_only_images() {
    let dir = tempfile::tempdir().unwrap();
    let png = save(&photo(32, 32), dir.path(), "p.png", ImageFormat::Png);
    assert_eq!(preview_bytes(&png, 1000).unwrap(), fs::read(&png).unwrap());

    // TIFF is not displayable by a webview: it comes back as a JPEG.
    let tiff = save(&photo(64, 48), dir.path(), "t.tiff", ImageFormat::Tiff);
    let bytes = preview_bytes(&tiff, 1000).unwrap();
    assert_eq!(&bytes[..2], &[0xFF, 0xD8], "expected JPEG magic");

    // Not an image: refused, so the command can't be used to read arbitrary files.
    let txt = dir.path().join("secret.txt");
    fs::write(&txt, "hi").unwrap();
    assert!(preview_bytes(&txt, 1000).is_err());
}

#[cfg(target_os = "macos")]
mod heic {
    use super::*;
    use std::process::Command;

    /// Encode a PNG to HEIC with Apple's own encoder, so the fixture is a real
    /// HEVC/HEIF file in the shape macOS and iPhones produce.
    fn make_heic(dir: &Path, w: u32, h: u32) -> (PathBuf, RgbaImage) {
        let original = photo(w, h);
        let png = save(&original, dir, "src.png", ImageFormat::Png);
        let heic = dir.join("photo.heic");
        let status = Command::new("/usr/bin/sips")
            .args(["-s", "format", "heic"])
            .arg(&png)
            .arg("--out")
            .arg(&heic)
            .output()
            .expect("sips available on macOS");
        assert!(status.status.success(), "sips could not write HEIC");
        (heic, original)
    }

    fn mean_abs_diff(a: &RgbaImage, b: &RgbaImage) -> f64 {
        assert_eq!(a.dimensions(), b.dimensions());
        let sum: u64 = a
            .pixels()
            .zip(b.pixels())
            .map(|(p, q)| (0..3).map(|c| (p[c] as i32 - q[c] as i32).unsigned_abs() as u64).sum::<u64>())
            .sum();
        sum as f64 / (a.width() as f64 * a.height() as f64 * 3.0)
    }

    #[test]
    fn heic_is_inspected_and_decoded_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let (heic, original) = make_heic(dir.path(), 320, 240);

        let info = inspect(&[heic.to_string_lossy().into_owned()]);
        assert_eq!(info.files.len(), 1, "{info:?}");
        assert_eq!(info.files[0].format, "heic");
        assert_eq!((info.files[0].width, info.files[0].height), (320, 240));

        // Decode to lossless WebP and compare pixels with the source image.
        let s = Settings { lossless: true, include_larger: true, ..with_formats(&[OutputFormat::Webp]) };
        let out = only(run_one(&heic, &s));
        let decoded = image::open(&out.output_path).unwrap().to_rgba8();
        let diff = mean_abs_diff(&original, &decoded);
        assert!(diff < 12.0, "HEIC pixels differ too much from the source: {diff:.2}");
    }

    /// The pure-Rust decoder must work on its own: the `sips` fallback only
    /// exists on macOS, and a passing pipeline test could be hiding it.
    #[test]
    fn pure_rust_decoder_reads_apple_heic() {
        let dir = tempfile::tempdir().unwrap();
        let (heic, original) = make_heic(dir.path(), 256, 192);
        let bytes = fs::read(&heic).unwrap();
        let opts = heic_rs::DecodeOptions::default().with_layout(heic_rs::PixelLayout::Rgba8);
        let img = heic_rs::decode(&bytes, &opts).expect("heic-rs should decode Apple's HEIC");
        assert_eq!((img.width, img.height), (256, 192));
        let rgba = RgbaImage::from_raw(img.width, img.height, img.data).unwrap();
        assert!(mean_abs_diff(&original, &rgba) < 12.0);
    }

    #[test]
    fn heic_with_original_becomes_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let (heic, _) = make_heic(dir.path(), 128, 96);
        let out = only(run_one(&heic, &Settings::default()));
        assert_eq!(out.format, OutputFormat::Jpeg);
        assert_eq!(out.output_path.extension().unwrap(), "jpg");
    }
}
