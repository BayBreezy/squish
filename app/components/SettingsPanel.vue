<template>
  <div class="divide-y px-6 [&_label]:text-[13px] [&_label]:font-normal">
    <section class="space-y-4 py-6">
      <div class="flex items-center justify-between gap-4">
        <h3 class="text-sm font-semibold">Preset</h3>
        <UiButton variant="link" size="xs" class="h-auto p-0" @click="$emit('reset')"
          >Reset</UiButton
        >
      </div>
      <div class="grid grid-cols-3 gap-2">
        <button
          v-for="(preset, key) in PRESETS"
          :key="key"
          type="button"
          class="rounded-lg border p-2 text-left transition-colors hover:border-primary/60"
          @click="$emit('preset', key)"
        >
          <span class="block text-xs font-medium">{{ preset.label }}</span>
          <span class="block text-[11px] leading-tight text-muted-foreground">{{
            preset.hint
          }}</span>
        </button>
      </div>
    </section>

    <section class="space-y-4 py-6">
      <div class="flex items-center justify-between gap-4">
        <h3 class="text-sm font-semibold">Formats</h3>
        <UiButton variant="link" size="xs" class="h-auto p-0" @click="selectAllFormats">
          Select all
        </UiButton>
      </div>
      <UiToggleGroup
        type="multiple"
        :model-value="settings.formats"
        class="flex-wrap justify-start"
        @update:model-value="setFormats"
      >
        <UiToggleGroupItem
          v-for="chip in FORMAT_CHIPS"
          :key="chip.value"
          :value="chip.value"
          variant="outline"
          size="sm"
          class="text-xs"
        >
          {{ chip.label }}
        </UiToggleGroupItem>
      </UiToggleGroup>
      <p class="text-xs text-muted-foreground">
        Pick one or more - every image is converted to each selected format. "Original" keeps each
        file's own format.
      </p>
      <p v-if="settings.formats.includes('avif')" class="text-xs text-muted-foreground">
        AVIF gives the smallest files but is the slowest to encode.
      </p>
    </section>

    <section class="space-y-4 py-6">
      <div class="flex items-center justify-between gap-4">
        <UiLabel>Quality</UiLabel>
        <span class="text-[13px] text-muted-foreground tabular-nums">{{ settings.quality }}</span>
      </div>
      <UiSlider
        :model-value="[settings.quality]"
        :min="1"
        :max="100"
        :step="1"
        :disabled="isLossless"
        @update:model-value="(v) => v && v[0] != null && (settings.quality = v[0])"
      />
      <p class="text-xs text-muted-foreground">
        {{
          isLossless
            ? "Lossless: pixels are preserved exactly."
            : settings.lossless
              ? "Lossless applies to PNG and WebP; this slider still controls JPEG and AVIF."
              : "Lower is smaller. 75–85 is usually indistinguishable."
        }}
      </p>
      <div v-if="supportsLossless" class="flex items-center justify-between pt-1">
        <UiLabel for="lossless">Lossless</UiLabel>
        <UiSwitch id="lossless" v-model="settings.lossless" />
      </div>
    </section>

    <section v-if="showPng" class="space-y-4 py-6">
      <h3 class="text-sm font-semibold">PNG</h3>
      <div class="flex items-center justify-between gap-4">
        <UiLabel for="quantize">Reduce colours (like TinyPNG)</UiLabel>
        <UiSwitch id="quantize" v-model="settings.pngQuantize" :disabled="settings.lossless" />
      </div>
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-4">
          <UiLabel>Optimisation effort</UiLabel>
          <span class="text-[13px] text-muted-foreground tabular-nums">{{
            settings.pngOptimizeLevel
          }}</span>
        </div>
        <UiSlider
          :model-value="[settings.pngOptimizeLevel]"
          :min="0"
          :max="6"
          @update:model-value="(v) => v && v[0] != null && (settings.pngOptimizeLevel = v[0])"
        />
      </div>
    </section>

    <section v-if="showJpeg" class="space-y-4 py-6">
      <h3 class="text-sm font-semibold">JPEG</h3>
      <div class="flex items-center justify-between gap-4">
        <UiLabel for="progressive">Progressive</UiLabel>
        <UiSwitch id="progressive" v-model="settings.jpegProgressive" />
      </div>
      <div class="flex items-center justify-between gap-4">
        <UiLabel for="chroma">Sharper colour edges (4:4:4)</UiLabel>
        <UiSwitch id="chroma" v-model="settings.jpegChroma444" />
      </div>
      <div class="flex items-center justify-between gap-4">
        <UiLabel for="bg">Background for transparency</UiLabel>
        <input
          id="bg"
          v-model="settings.background"
          type="color"
          class="h-8 w-12 cursor-pointer rounded border bg-transparent p-0.5"
        />
      </div>
    </section>

    <section v-if="showWebp" class="space-y-4 py-6">
      <h3 class="text-sm font-semibold">WebP</h3>
      <div class="flex items-center justify-between gap-4">
        <UiLabel>Compression effort</UiLabel>
        <span class="text-[13px] text-muted-foreground tabular-nums">{{
          settings.webpMethod
        }}</span>
      </div>
      <UiSlider
        :model-value="[settings.webpMethod]"
        :min="0"
        :max="6"
        @update:model-value="(v) => v && v[0] != null && (settings.webpMethod = v[0])"
      />
    </section>

    <section v-if="showAvif" class="space-y-4 py-6">
      <h3 class="text-sm font-semibold">AVIF</h3>
      <div class="flex items-center justify-between gap-4">
        <UiLabel>Speed (low = smaller, slower)</UiLabel>
        <span class="text-[13px] text-muted-foreground tabular-nums">{{ settings.avifSpeed }}</span>
      </div>
      <UiSlider
        :model-value="[settings.avifSpeed]"
        :min="1"
        :max="10"
        @update:model-value="(v) => v && v[0] != null && (settings.avifSpeed = v[0])"
      />
    </section>

    <section class="space-y-4 py-6">
      <h3 class="text-sm font-semibold">Resize</h3>
      <p class="-mt-1 text-xs text-muted-foreground">Fit within a box. Never enlarges.</p>
      <div class="grid grid-cols-2 gap-2">
        <div class="space-y-2">
          <UiLabel for="mw" class="text-xs">Max width (px)</UiLabel>
          <UiInput
            id="mw"
            type="number"
            placeholder="Original"
            :model-value="settings.maxWidth ?? ''"
            @update:model-value="(v: any) => (settings.maxWidth = toDimension(v))"
          />
        </div>
        <div class="space-y-2">
          <UiLabel for="mh" class="text-xs">Max height (px)</UiLabel>
          <UiInput
            id="mh"
            type="number"
            placeholder="Original"
            :model-value="settings.maxHeight ?? ''"
            @update:model-value="(v: any) => (settings.maxHeight = toDimension(v))"
          />
        </div>
      </div>
    </section>

    <section class="space-y-4 py-6">
      <h3 class="text-sm font-semibold">Output</h3>
      <div class="space-y-2">
        <UiLabel for="suffix" class="text-xs">File name suffix</UiLabel>
        <UiInput id="suffix" v-model="settings.suffix" placeholder="-squished" />
        <p class="text-xs text-muted-foreground">
          Originals are never overwritten. Existing names get “(2)”, “(3)”…
        </p>
      </div>
      <div class="flex items-start justify-between gap-4">
        <div class="space-y-1">
          <UiLabel for="keep-metadata" class="text-xs">Keep metadata</UiLabel>
          <p class="text-xs text-muted-foreground">
            Copies EXIF (camera, date and GPS location) into JPEG, PNG and WebP files. Off by
            default for smaller, more private files. AVIF never carries metadata.
          </p>
        </div>
        <UiSwitch id="keep-metadata" v-model="settings.keepMetadata" />
      </div>
      <div class="flex items-start justify-between gap-3">
        <div class="space-y-1">
          <UiLabel for="include-larger" class="text-xs">Also write larger outputs</UiLabel>
          <p class="text-xs text-muted-foreground">
            By default, a converted file that would be bigger than the original is skipped.
          </p>
        </div>
        <UiSwitch id="include-larger" v-model="settings.includeLarger" />
      </div>
      <div class="space-y-2">
        <UiLabel class="text-xs">Save to</UiLabel>
        <div class="flex items-center gap-2">
          <UiButton
            variant="outline"
            size="sm"
            class="min-w-0 flex-1 justify-start"
            @click="$emit('chooseOutputDir')"
          >
            <Icon name="lucide:folder" />
            <span class="truncate">{{ settings.outputDir ?? "Next to the original" }}</span>
          </UiButton>
          <IconButton
            v-if="settings.outputDir"
            icon="lucide:x"
            label="Save next to originals"
            @click="settings.outputDir = null"
          />
        </div>
      </div>
    </section>
  </div>
</template>

<script lang="ts" setup>
  import type { PresetName } from "~/composables/useSettings";
  import type { OutputFormat, Settings } from "~/types/squish";

  const settings = defineModel<Settings>({ required: true });
  defineEmits<{ preset: [name: PresetName]; reset: []; chooseOutputDir: [] }>();

  const FORMAT_CHIPS: { value: OutputFormat; label: string }[] = [
    { value: "original", label: "Original" },
    { value: "webp", label: "WebP" },
    { value: "avif", label: "AVIF" },
    { value: "jpeg", label: "JPEG" },
    { value: "png", label: "PNG" },
  ];

  /** Ignore a change that would leave nothing selected: at least one format is required. */
  function setFormats(value: unknown) {
    const next = (Array.isArray(value) ? value : []) as OutputFormat[];
    if (next.length > 0) settings.value.formats = next;
  }

  function selectAllFormats() {
    settings.value.formats = FORMAT_CHIPS.map((c) => c.value);
  }

  const formats = computed(() => settings.value.formats);
  // "Original" can resolve to PNG, JPEG or WebP depending on the file.
  const showPng = computed(() => formats.value.some((f) => f === "png" || f === "original"));
  const showJpeg = computed(() => formats.value.some((f) => f === "jpeg" || f === "original"));
  const showWebp = computed(() => formats.value.some((f) => f === "webp" || f === "original"));
  const showAvif = computed(() => formats.value.includes("avif"));
  const supportsLossless = computed(() => showPng.value || showWebp.value);
  // The quality slider only becomes irrelevant when every selected format is lossless-capable.
  const allLosslessCapable = computed(() =>
    formats.value.every((f) => f === "png" || f === "webp"),
  );
  const isLossless = computed(() => settings.value.lossless && allLosslessCapable.value);

  function toDimension(v: unknown): number | null {
    const n = Math.floor(Number(v));
    return Number.isFinite(n) && n > 0 ? n : null;
  }
</script>
