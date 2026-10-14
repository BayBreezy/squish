<template>
  <UiDialog v-model:open="open">
    <UiDialogContent class="flex max-h-[94vh] flex-col gap-4 sm:max-w-5xl">
      <UiDialogHeader>
        <UiDialogTitle class="truncate pr-8">{{ item?.name }}</UiDialogTitle>
        <UiDialogDescription>
          Drag the divider (or use the arrow keys) to compare the original with the result.
        </UiDialogDescription>
      </UiDialogHeader>

      <div v-if="options.length > 1" class="flex flex-wrap items-center gap-2">
        <span class="text-xs text-muted-foreground">Compare with</span>
        <UiButton
          v-for="o in options"
          :key="o.format"
          size="xs"
          :variant="o.format === selected ? 'default' : 'outline'"
          @click="selected = o.format"
        >
          {{ FORMAT_LABEL[o.format] }} · {{ formatBytes(o.bytes) }}
        </UiButton>
      </div>

      <div class="flex min-h-48 items-center justify-center">
        <Icon
          v-if="loading && !beforeUrl"
          name="line-md:loading-loop"
          class="size-8 text-muted-foreground"
        />
        <p v-else-if="error" class="text-sm text-destructive">
          Couldn’t load a preview: {{ error }}
        </p>

        <div
          v-else-if="beforeUrl"
          ref="stage"
          tabindex="0"
          role="slider"
          aria-label="Before and after divider"
          :aria-valuenow="Math.round(position)"
          aria-valuemin="0"
          aria-valuemax="100"
          class="relative mx-auto w-fit touch-none overflow-hidden rounded-lg border bg-muted outline-none select-none focus-visible:ring-2 focus-visible:ring-ring"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="dragging = false"
          @pointercancel="dragging = false"
          @keydown.left.prevent="position = Math.max(0, position - 2)"
          @keydown.right.prevent="position = Math.min(100, position + 2)"
        >
          <!-- Original underneath: it sets the size of the stage. -->
          <img
            :src="beforeUrl"
            alt="Original"
            draggable="false"
            class="block max-h-[60vh] max-w-full"
          />

          <!-- Result on top, visible from the divider to the right. -->
          <img
            v-if="afterUrl"
            :src="afterUrl"
            alt="Result"
            draggable="false"
            class="absolute inset-0 size-full"
            :style="{ clipPath: `inset(0 0 0 ${position}%)` }"
          />

          <div
            class="pointer-events-none absolute inset-y-0 w-0.5 -translate-x-1/2 bg-white shadow-[0_0_0_1px_rgba(0,0,0,0.35)]"
            :style="{ left: `${position}%` }"
          >
            <span
              class="absolute top-1/2 left-1/2 flex size-8 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-white text-neutral-800 shadow-md"
            >
              <Icon name="lucide:chevrons-left-right" class="size-4" />
            </span>
          </div>

          <span
            class="pointer-events-none absolute top-2 left-2 rounded-md bg-black/60 px-2 py-1 text-xs text-white"
          >
            Original · {{ formatBytes(item?.bytes ?? 0) }}
          </span>
          <span
            v-if="current"
            class="pointer-events-none absolute top-2 right-2 rounded-md bg-black/60 px-2 py-1 text-xs text-white"
          >
            {{ FORMAT_LABEL[current.format] }} · {{ formatBytes(current.bytes) }} ·
            {{ formatPercent(current.savedPercent) }}
          </span>
        </div>
      </div>
    </UiDialogContent>
  </UiDialog>
</template>

<script lang="ts" setup>
  import { invoke } from "@tauri-apps/api/core";

  import type { ConcreteFormat, QueueItem } from "~/types/squish";

  const props = defineProps<{ item: QueueItem | null }>();
  const open = defineModel<boolean>("open", { default: false });

  const MIME: Record<string, string> = {
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    webp: "image/webp",
    gif: "image/gif",
    bmp: "image/bmp",
    avif: "image/avif",
    // Decoded to JPEG by the backend because a webview can't show these.
    heic: "image/jpeg",
    heif: "image/jpeg",
    hif: "image/jpeg",
    tif: "image/jpeg",
    tiff: "image/jpeg",
  };

  // Only outputs that differ from the original are worth comparing.
  const options = computed(() =>
    props.item ? writtenOutputs(props.item).filter((o) => !o.keptOriginal) : [],
  );
  const selected = ref<ConcreteFormat | null>(null);
  const current = computed(() => options.value.find((o) => o.format === selected.value));

  const beforeUrl = ref<string | null>(null);
  const afterUrl = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const position = ref(50);
  const dragging = ref(false);
  const stage = useTemplateRef<HTMLElement>("stage");

  let generation = 0;

  async function fetchUrl(path: string): Promise<string> {
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    const bytes = await invoke<ArrayBuffer>("read_preview", { path, maxPx: 3000 });
    return URL.createObjectURL(new Blob([bytes], { type: MIME[ext] ?? "image/jpeg" }));
  }

  function revoke(which: "before" | "after" | "both") {
    if (which !== "after" && beforeUrl.value) URL.revokeObjectURL(beforeUrl.value);
    if (which !== "before" && afterUrl.value) URL.revokeObjectURL(afterUrl.value);
    if (which !== "after") beforeUrl.value = null;
    if (which !== "before") afterUrl.value = null;
  }

  // (Re)load the original when the dialog opens for an item.
  watch(
    () => [open.value, props.item?.path] as const,
    async ([isOpen, path]) => {
      const run = ++generation;
      revoke("both");
      error.value = null;
      if (!isOpen || !path || !props.item) return;
      position.value = 50;
      selected.value =
        (bestOutput(props.item)?.keptOriginal ? options.value[0] : bestOutput(props.item))
          ?.format ??
        options.value[0]?.format ??
        null;
      loading.value = true;
      try {
        const url = await fetchUrl(path);
        if (run !== generation) return URL.revokeObjectURL(url);
        beforeUrl.value = url;
      } catch (e) {
        if (run === generation) error.value = String(e);
      } finally {
        if (run === generation) loading.value = false;
      }
    },
    { immediate: true },
  );

  // (Re)load the result whenever the chosen format changes.
  watch(
    () => [open.value, current.value?.outputPath] as const,
    async ([isOpen, outputPath]) => {
      const run = generation;
      revoke("after");
      if (!isOpen || !outputPath) return;
      try {
        const url = await fetchUrl(outputPath);
        if (run !== generation || !open.value) return URL.revokeObjectURL(url);
        afterUrl.value = url;
      } catch (e) {
        error.value = String(e);
      }
    },
    { immediate: true },
  );

  function setFromPointer(e: PointerEvent) {
    const el = stage.value;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    position.value = Math.min(100, Math.max(0, ((e.clientX - rect.left) / rect.width) * 100));
  }
  function onPointerDown(e: PointerEvent) {
    dragging.value = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    setFromPointer(e);
  }
  function onPointerMove(e: PointerEvent) {
    if (dragging.value) setFromPointer(e);
  }

  onBeforeUnmount(() => revoke("both"));
</script>
