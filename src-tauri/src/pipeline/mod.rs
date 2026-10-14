//! The image pipeline: decode → orient → resize → encode → write.
//! Pure Rust functions with no Tauri types, so they are easy to unit test.

mod avif;
mod color;
mod decode;
mod jpeg;
mod metadata;
mod png;
mod webp;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use image::RgbaImage;
use rayon::prelude::*;

use decode::SourceFormat;

use crate::{
    error::AppError,
    types::{FileInfo, InspectResult, OutputFormat, ProcessEvent, Settings, SkippedFile},
};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "heic", "heif", "hif",
];

fn has_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Expand dropped folders (recursively) into the image files inside them.
/// Returns the image files, files rejected when passed directly, and how many
/// non-image files were found (and ignored) inside folders.
pub fn collect_images(paths: &[String]) -> (Vec<PathBuf>, Vec<SkippedFile>, u32) {
    fn walk(dir: &Path, depth: u8, out: &mut Vec<PathBuf>, ignored: &mut u32) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let hidden = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'));
            if hidden {
                continue;
            }
            if p.is_dir() {
                walk(&p, depth + 1, out, ignored);
            } else if has_supported_extension(&p) {
                out.push(p);
            } else {
                *ignored += 1;
            }
        }
    }

    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut ignored = 0;
    for raw in paths {
        let p = PathBuf::from(raw);
        if p.is_dir() {
            walk(&p, 0, &mut files, &mut ignored);
        } else if has_supported_extension(&p) {
            files.push(p);
        } else {
            skipped.push(SkippedFile {
                path: raw.clone(),
                reason: "Unsupported file type".into(),
            });
        }
    }
    files.dedup();
    (files, skipped, ignored)
}

/// Read headers only (no full decode) so adding hundreds of files is instant.
pub fn inspect(paths: &[String]) -> InspectResult {
    let (candidates, mut skipped, ignored) = collect_images(paths);
    let mut files = Vec::new();
    for path in candidates {
        let display = path.to_string_lossy().into_owned();
        let probe = || -> Result<FileInfo, AppError> {
            let bytes = fs::metadata(&path)?.len();
            let (format, width, height) = if decode::is_heic(&path) {
                let (w, h) = decode::probe_heic(&path)?;
                (SourceFormat::Heic, w, h)
            } else {
                let reader = image::ImageReader::open(&path)?.with_guessed_format()?;
                let format = reader
                    .format()
                    .ok_or_else(|| AppError::Unsupported("Unrecognised image format".into()))?;
                let (w, h) = reader.into_dimensions()?;
                (SourceFormat::from(format), w, h)
            };
            Ok(FileInfo {
                path: display.clone(),
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                bytes,
                width,
                height,
                format: format.name().into(),
                animated: decode::is_animated(&path, format),
            })
        };
        match probe() {
            Ok(info) => files.push(info),
            Err(e) => skipped.push(SkippedFile {
                path: display,
                reason: e.to_string(),
            }),
        }
    }
    InspectResult {
        files,
        skipped,
        ignored,
    }
}

/// Small JPEG preview, orientation-corrected.
pub fn thumbnail(path: &Path, max_px: u32) -> Result<Vec<u8>, AppError> {
    let loaded = decode::load(path)?;
    let thumb = image::DynamicImage::ImageRgba8(loaded.rgba)
        .thumbnail(max_px, max_px)
        .to_rgb8();
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 78);
    thumb
        .write_with_encoder(enc)
        .map_err(|e| AppError::Encode(e.to_string()))?;
    Ok(out)
}

/// Bytes for the before/after viewer.
///
/// Formats a webview can display (PNG, JPEG, WebP, GIF, BMP, AVIF) are returned
/// untouched, so the comparison shows exactly what is on disk. Anything else
/// (HEIC, TIFF) is decoded and returned as a high-quality JPEG no wider than
/// `max_px`. Only image files are served.
pub fn preview_bytes(path: &Path, max_px: u32) -> Result<Vec<u8>, AppError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let known = SUPPORTED_EXTENSIONS.contains(&ext.as_str()) || ext == "avif";
    if !known {
        return Err(AppError::Unsupported("Not an image file".into()));
    }
    if matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "avif"
    ) {
        return Ok(fs::read(path)?);
    }
    let loaded = decode::load(path)?;
    let img = image::DynamicImage::ImageRgba8(loaded.rgba);
    let img = decode::fit(img, Some(max_px), Some(max_px)).to_rgb8();
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
    img.write_with_encoder(enc)
        .map_err(|e| AppError::Encode(e.to_string()))?;
    Ok(out)
}

/// Resolve "keep original" to a concrete codec based on the source format.
fn resolve_format(requested: OutputFormat, source: SourceFormat) -> OutputFormat {
    match requested {
        OutputFormat::Original => match source {
            // HEIC can't be written, and JPEG is the universally readable stand-in.
            SourceFormat::Jpeg | SourceFormat::Heic => OutputFormat::Jpeg,
            SourceFormat::WebP => OutputFormat::Webp,
            // PNG, plus formats we can read but not write (gif/bmp/tiff).
            _ => OutputFormat::Png,
        },
        other => other,
    }
}

/// One concrete output for an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Target {
    format: OutputFormat,
    /// True when the user ticked this format explicitly (not via "Original").
    /// Only explicit conversions may be skipped for being larger.
    explicit: bool,
}

/// Resolve the selected formats for one source file: "Original" becomes the
/// source's own codec, duplicates collapse, cheap codecs come first.
fn targets(formats: &[OutputFormat], source: SourceFormat) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::new();
    for &requested in formats {
        let format = resolve_format(requested, source);
        let explicit = requested != OutputFormat::Original;
        match out.iter_mut().find(|t| t.format == format) {
            // Same codec requested twice (e.g. Original + WebP on a WebP file):
            // the "keep original" meaning wins, so it is never skipped.
            Some(existing) => existing.explicit &= explicit,
            None => out.push(Target { format, explicit }),
        }
    }
    out.sort_by_key(|t| t.format.cost_rank());
    out
}

fn encode(rgba: &RgbaImage, format: OutputFormat, s: &Settings) -> Result<Vec<u8>, AppError> {
    match format {
        OutputFormat::Png => png::encode(rgba, s),
        OutputFormat::Jpeg => jpeg::encode(rgba, s),
        OutputFormat::Webp => webp::encode(rgba, s),
        OutputFormat::Avif => avif::encode(rgba, s),
        OutputFormat::Original => unreachable!("resolved before encoding"),
    }
}

/// Claim `<dir>/<stem><suffix>.<ext>` (adding " (2)", " (3)"…) by creating it
/// exclusively, so nothing is overwritten and parallel workers can't pick the
/// same name. The returned path exists as an empty placeholder.
fn reserve_output_path(source: &Path, s: &Settings, ext: &str) -> Result<PathBuf, AppError> {
    let dir = s
        .output_dir
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| source.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    if !dir.as_os_str().is_empty() {
        fs::create_dir_all(&dir)?;
    }
    let stem = source
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".into());
    let base = format!("{stem}{}", s.suffix);

    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{base}.{ext}")
        } else {
            format!("{base} ({n}).{ext}")
        };
        let candidate = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(_) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e.into()),
        }
    }
}

/// Write `bytes` over the reserved placeholder via a temp file + rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".squish-part");
    let tmp = PathBuf::from(tmp);
    let result = fs::write(&tmp, bytes).and_then(|_| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(result?)
}

/// The image decoded, oriented and resized once, shared by every target.
struct Prepared {
    rgba: RgbaImage,
    source_format: SourceFormat,
    /// EXIF to copy into outputs when "keep metadata" is on.
    exif: Option<Vec<u8>>,
    original_bytes: u64,
    resized: bool,
}

fn prepare(path: &Path, settings: &Settings) -> Result<Prepared, AppError> {
    let original_bytes = fs::metadata(path)?.len();
    let loaded = decode::load(path)?;
    let img = decode::fit(
        image::DynamicImage::ImageRgba8(loaded.rgba),
        settings.max_width,
        settings.max_height,
    );
    Ok(Prepared {
        rgba: img.into_rgba8(),
        source_format: loaded.format,
        exif: loaded.exif,
        original_bytes,
        resized: settings.max_width.is_some() || settings.max_height.is_some(),
    })
}

fn saved_percent(original: u64, output: u64) -> f64 {
    if original == 0 {
        0.0
    } else {
        (1.0 - output as f64 / original as f64) * 100.0
    }
}

/// Encode one target and emit its event. Errors are per-format.
fn write_target(
    path: &Path,
    display: &str,
    prep: &Prepared,
    target: Target,
    settings: &Settings,
    emit: &(dyn Fn(ProcessEvent) + Sync),
) -> Result<(), AppError> {
    let format = target.format;
    let same_format = matches!(
        (prep.source_format, format),
        (SourceFormat::Png, OutputFormat::Png)
            | (SourceFormat::Jpeg, OutputFormat::Jpeg)
            | (SourceFormat::WebP, OutputFormat::Webp)
    );
    let mut encoded = encode(&prep.rgba, format, settings)?;
    if settings.keep_metadata
        && let Some(exif) = &prep.exif
    {
        encoded = metadata::embed_exif(encoded, format, exif);
    }
    let size = encoded.len() as u64;

    // Recompressing the same format can come out bigger (already-optimised
    // input). Never make files larger: hand back the original bytes instead.
    // A requested resize is the point of the operation, so it is exempt.
    if same_format && !prep.resized && size >= prep.original_bytes {
        let output_path = reserve_output_path(path, settings, format.extension())?;
        fs::copy(path, &output_path)?;
        emit(ProcessEvent::Output {
            path: display.to_owned(),
            format,
            output_path: output_path.to_string_lossy().into_owned(),
            original_bytes: prep.original_bytes,
            output_bytes: prep.original_bytes,
            saved_percent: 0.0,
            kept_original: true,
        });
        return Ok(());
    }

    // A conversion the user asked for that came out larger than the source.
    if !same_format
        && target.explicit
        && !prep.resized
        && !settings.include_larger
        && size >= prep.original_bytes
    {
        emit(ProcessEvent::Skipped {
            path: display.to_owned(),
            format,
            would_be_bytes: size,
        });
        return Ok(());
    }

    let output_path = reserve_output_path(path, settings, format.extension())?;
    if let Err(e) = write_atomic(&output_path, &encoded) {
        let _ = fs::remove_file(&output_path);
        return Err(e);
    }
    emit(ProcessEvent::Output {
        path: display.to_owned(),
        format,
        output_path: output_path.to_string_lossy().into_owned(),
        original_bytes: prep.original_bytes,
        output_bytes: size,
        saved_percent: saved_percent(prep.original_bytes, size),
        kept_original: false,
    });
    Ok(())
}

/// Process one input into every selected format. Decode failures return `Err`;
/// per-format failures are reported as `OutputFailed` events and the remaining
/// formats still run. Stops between formats once `cancel` is set.
pub fn process_one(
    path: &Path,
    settings: &Settings,
    cancel: &AtomicBool,
    emit: &(dyn Fn(ProcessEvent) + Sync),
) -> Result<(), AppError> {
    let display = path.to_string_lossy().into_owned();
    let prep = prepare(path, settings)?;
    for target in targets(&settings.formats, prep.source_format) {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Err(e) = write_target(path, &display, &prep, target, settings, emit) {
            emit(ProcessEvent::OutputFailed {
                path: display.clone(),
                format: target.format,
                error: e.to_string(),
            });
        }
    }
    Ok(())
}

/// Process a batch on a bounded rayon pool, reporting progress through `emit`.
/// Returns `true` if the run was cancelled before every file was handled.
pub fn run_batch<F>(paths: &[PathBuf], settings: &Settings, cancel: &AtomicBool, emit: F) -> bool
where
    F: Fn(ProcessEvent) + Sync,
{
    let work = || {
        paths.par_iter().for_each(|path| {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let display = path.to_string_lossy().into_owned();
            emit(ProcessEvent::Started {
                path: display.clone(),
            });
            match process_one(path, settings, cancel, &emit) {
                Ok(()) => emit(ProcessEvent::Done { path: display }),
                Err(e) => emit(ProcessEvent::Failed {
                    path: display,
                    error: e.to_string(),
                }),
            }
        });
    };

    // Each worker holds a decoded RGBA image plus encoder buffers (AVIF is
    // hungry), so cap concurrency instead of using every core.
    let workers = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, 4);
    match rayon::ThreadPoolBuilder::new().num_threads(workers).build() {
        Ok(pool) => pool.install(work),
        Err(_) => work(),
    }
    cancel.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests;
