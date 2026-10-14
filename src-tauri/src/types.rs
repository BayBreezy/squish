//! Types shared with the frontend. Field names are camelCase on the JS side
//! (see `app/types/squish.ts`, which mirrors this file).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Keep whatever format the source file has.
    Original,
    Png,
    Jpeg,
    Webp,
    Avif,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Webp => "webp",
            OutputFormat::Avif => "avif",
            OutputFormat::Original => "",
        }
    }

    /// Cheap codecs first so their results appear before slow AVIF finishes.
    pub fn cost_rank(self) -> u8 {
        match self {
            OutputFormat::Original => 0,
            OutputFormat::Png => 1,
            OutputFormat::Jpeg => 2,
            OutputFormat::Webp => 3,
            OutputFormat::Avif => 4,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Every selected output format; each input is encoded once per entry.
    pub formats: Vec<OutputFormat>,
    /// 1-100, interpreted per codec.
    pub quality: u8,
    /// Lossless mode for PNG and WebP.
    pub lossless: bool,

    // PNG
    pub png_quantize: bool,
    pub png_max_colors: u16,
    /// oxipng preset, 0 (fast) to 6 (slow, smallest).
    pub png_optimize_level: u8,

    // JPEG
    pub jpeg_progressive: bool,
    /// true = 4:4:4 (sharper colour edges), false = 4:2:0 (smaller).
    pub jpeg_chroma_444: bool,
    /// `#rrggbb`, used when flattening transparency for JPEG.
    pub background: String,

    // WebP
    /// 0 (fast) to 6 (slow, smallest).
    pub webp_method: u8,

    // AVIF
    /// 1 (slow, smallest) to 10 (fast).
    pub avif_speed: u8,

    // Resize (never upscales)
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,

    // Output
    pub suffix: String,
    pub output_dir: Option<String>,
    /// Copy EXIF (camera, date, GPS, copyright) into PNG/JPEG/WebP outputs.
    /// Off by default: stripped files are smaller and don't leak location.
    pub keep_metadata: bool,
    /// Also write converted files that come out larger than the original.
    pub include_larger: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            formats: vec![OutputFormat::Original],
            quality: 80,
            lossless: false,
            png_quantize: true,
            png_max_colors: 256,
            png_optimize_level: 3,
            jpeg_progressive: true,
            jpeg_chroma_444: false,
            background: "#ffffff".into(),
            webp_method: 4,
            avif_speed: 6,
            max_width: None,
            max_height: None,
            suffix: "-squished".into(),
            output_dir: None,
            keep_metadata: false,
            include_larger: false,
        }
    }
}

impl Settings {
    /// Clamp everything the UI could send into ranges the codecs accept.
    pub fn sanitized(mut self) -> Self {
        self.quality = self.quality.clamp(1, 100);
        self.png_max_colors = self.png_max_colors.clamp(2, 256);
        self.png_optimize_level = self.png_optimize_level.min(6);
        self.webp_method = self.webp_method.min(6);
        self.avif_speed = self.avif_speed.clamp(1, 10);
        self.max_width = self.max_width.filter(|v| *v > 0);
        self.max_height = self.max_height.filter(|v| *v > 0);
        // Strip anything that could escape the directory when used in a file name.
        self.suffix = self
            .suffix
            .chars()
            .filter(|c| !matches!(c, '/' | '\\' | '\0' | ':'))
            .collect();
        self.output_dir = self.output_dir.filter(|d| !d.trim().is_empty());
        // De-duplicate, keep at least one target.
        let mut seen = Vec::new();
        for f in self.formats.drain(..) {
            if !seen.contains(&f) {
                seen.push(f);
            }
        }
        if seen.is_empty() {
            seen.push(OutputFormat::Original);
        }
        self.formats = seen;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    /// Lower-case format name: png, jpeg, webp, gif, bmp, tiff, heic.
    pub format: String,
    /// Animated GIF / WebP / APNG: only the first frame is processed.
    pub animated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectResult {
    pub files: Vec<FileInfo>,
    /// Files that were passed directly but can't be processed (with a reason).
    pub skipped: Vec<SkippedFile>,
    /// Non-image files found inside dropped folders. They are left alone.
    pub ignored: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum ProcessEvent {
    #[serde(rename_all = "camelCase")]
    Started { path: String },
    /// One output file was written (or the original bytes were kept).
    #[serde(rename_all = "camelCase")]
    Output {
        path: String,
        format: OutputFormat,
        output_path: String,
        original_bytes: u64,
        output_bytes: u64,
        /// Negative when the output grew (only possible when the format changed).
        saved_percent: f64,
        /// True when recompressing did not help, so the original bytes were kept.
        kept_original: bool,
    },
    /// A conversion was skipped because it would be larger than the original.
    #[serde(rename_all = "camelCase")]
    Skipped {
        path: String,
        format: OutputFormat,
        would_be_bytes: u64,
    },
    /// One format failed; the other formats for this file continue.
    #[serde(rename_all = "camelCase")]
    OutputFailed {
        path: String,
        format: OutputFormat,
        error: String,
    },
    /// Every target for this input has been handled.
    #[serde(rename_all = "camelCase")]
    Done { path: String },
    /// The input could not be read at all.
    #[serde(rename_all = "camelCase")]
    Failed { path: String, error: String },
    #[serde(rename_all = "camelCase")]
    Finished { cancelled: bool },
}
