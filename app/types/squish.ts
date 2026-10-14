// Mirrors src-tauri/src/types.rs (serde camelCase).

export type OutputFormat = "original" | "png" | "jpeg" | "webp" | "avif";
/** A concrete codec ("original" is resolved per file by the backend). */
export type ConcreteFormat = Exclude<OutputFormat, "original">;

export interface Settings {
  /** Every selected output format; each input is encoded once per entry. */
  formats: OutputFormat[];
  /** 1-100 */
  quality: number;
  lossless: boolean;
  pngQuantize: boolean;
  pngMaxColors: number;
  /** 0-6 */
  pngOptimizeLevel: number;
  jpegProgressive: boolean;
  jpegChroma444: boolean;
  /** #rrggbb */
  background: string;
  /** 0-6 */
  webpMethod: number;
  /** 1-10 */
  avifSpeed: number;
  maxWidth: number | null;
  maxHeight: number | null;
  suffix: string;
  outputDir: string | null;
  /** Copy EXIF (camera, date, GPS) into PNG/JPEG/WebP outputs. */
  keepMetadata: boolean;
  /** Also write converted files that are larger than the original. */
  includeLarger: boolean;
}

export interface FileInfo {
  path: string;
  name: string;
  bytes: number;
  width: number;
  height: number;
  format: string;
  /** Animated GIF / WebP / APNG: only the first frame is processed. */
  animated: boolean;
}

export interface SkippedFile {
  path: string;
  reason: string;
}

export interface InspectResult {
  files: FileInfo[];
  /** Files passed directly that can't be processed. */
  skipped: SkippedFile[];
  /** Non-image files found inside dropped folders (left untouched). */
  ignored: number;
}

export type ProcessEvent =
  | { event: "started"; data: { path: string } }
  | {
      event: "output";
      data: {
        path: string;
        format: ConcreteFormat;
        outputPath: string;
        originalBytes: number;
        outputBytes: number;
        savedPercent: number;
        keptOriginal: boolean;
      };
    }
  | { event: "skipped"; data: { path: string; format: ConcreteFormat; wouldBeBytes: number } }
  | { event: "outputFailed"; data: { path: string; format: ConcreteFormat; error: string } }
  | { event: "done"; data: { path: string } }
  | { event: "failed"; data: { path: string; error: string } }
  | { event: "finished"; data: { cancelled: boolean } };

export type ItemStatus = "idle" | "queued" | "processing" | "done" | "error";

/** Result for one format of one input. */
export type OutputState =
  | {
      kind: "written";
      outputPath: string;
      bytes: number;
      savedPercent: number;
      keptOriginal: boolean;
    }
  | { kind: "skipped"; wouldBeBytes: number }
  | { kind: "failed"; error: string };

export interface QueueItem extends FileInfo {
  status: ItemStatus;
  thumb?: string;
  /** Per-format results, filled in as they stream from the backend. */
  outputs: Partial<Record<ConcreteFormat, OutputState>>;
  error?: string;
}
