import type { OutputFormat, Settings } from "~/types/squish";

export const DEFAULT_SETTINGS: Settings = {
  formats: ["original"],
  quality: 80,
  lossless: false,
  pngQuantize: true,
  pngMaxColors: 256,
  pngOptimizeLevel: 3,
  jpegProgressive: true,
  jpegChroma444: false,
  background: "#ffffff",
  webpMethod: 4,
  avifSpeed: 6,
  maxWidth: null,
  maxHeight: null,
  suffix: "-squished",
  outputDir: null,
  keepMetadata: false,
  includeLarger: false,
};

export type PresetName = "balanced" | "smallest" | "quality";

export const PRESETS: Record<
  PresetName,
  { label: string; hint: string; values: Partial<Settings> }
> = {
  balanced: {
    label: "Balanced",
    hint: "Great for the web",
    values: {
      quality: 80,
      pngOptimizeLevel: 3,
      webpMethod: 4,
      avifSpeed: 6,
      jpegChroma444: false,
      lossless: false,
    },
  },
  smallest: {
    label: "Smallest",
    hint: "Max savings",
    values: {
      quality: 60,
      pngOptimizeLevel: 5,
      webpMethod: 6,
      avifSpeed: 4,
      jpegChroma444: false,
      lossless: false,
    },
  },
  quality: {
    label: "Best quality",
    hint: "Near-lossless",
    values: {
      quality: 92,
      pngOptimizeLevel: 4,
      webpMethod: 5,
      avifSpeed: 5,
      jpegChroma444: true,
      lossless: false,
    },
  },
};

/** Persisted settings (localStorage). Call once and pass the ref down. */
export function useSettings() {
  const settings = useLocalStorage<Settings>(
    "squish-settings",
    { ...DEFAULT_SETTINGS },
    {
      mergeDefaults: true,
    },
  );

  // Migrate settings saved before multi-format output (`format: "webp"` → `formats: ["webp"]`).
  const legacy = settings.value as Settings & { format?: OutputFormat };
  if ("format" in legacy) {
    const { format, ...rest } = legacy;
    settings.value = { ...rest, formats: format ? [format] : ["original"] };
  }
  if (!Array.isArray(settings.value.formats) || settings.value.formats.length === 0) {
    settings.value = { ...settings.value, formats: ["original"] };
  }

  function applyPreset(name: PresetName) {
    settings.value = { ...settings.value, ...PRESETS[name].values };
  }

  function reset() {
    settings.value = { ...DEFAULT_SETTINGS };
  }

  return { settings, applyPreset, reset };
}
