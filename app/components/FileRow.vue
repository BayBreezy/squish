<template>
  <div
    class="group flex items-center gap-3 rounded-lg px-3 py-2 transition-colors hover:bg-muted/40"
  >
    <div
      class="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-md bg-muted"
    >
      <img v-if="item.thumb" :src="item.thumb" alt="" class="size-full object-cover" />
      <Icon v-else name="lucide:image" class="size-5 text-muted-foreground" />
    </div>

    <div class="min-w-0 flex-1">
      <p class="truncate text-sm font-medium" :title="item.path">{{ item.name }}</p>
      <p class="truncate text-xs text-muted-foreground">
        {{ item.format.toUpperCase() }} · {{ item.width }}×{{ item.height }} ·
        {{ formatBytes(item.bytes) }}
      </p>
      <p
        v-if="item.animated"
        class="mt-0.5 flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400"
      >
        <Icon name="lucide:film" class="size-3" />
        Animated: only the first frame is processed
      </p>

      <div v-if="chips.length" class="mt-1.5 flex flex-wrap gap-1.5">
        <span
          v-for="chip in chips"
          :key="chip.format"
          :class="[
            'inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-xs',
            chip.tone === 'best' && 'border-primary/40 bg-primary/10 font-medium text-primary',
            chip.tone === 'normal' && 'border-border text-foreground',
            chip.tone === 'muted' && 'border-dashed text-muted-foreground',
            chip.tone === 'error' && 'border-destructive/40 text-destructive',
          ]"
          :title="chip.title"
        >
          <Icon v-if="chip.tone === 'best'" name="lucide:check" class="size-3" />
          <span class="font-semibold">{{ chip.label }}</span>
          <span>{{ chip.text }}</span>
        </span>
      </div>

      <p
        v-if="item.status === 'error'"
        class="mt-1 truncate text-xs text-destructive"
        :title="item.error"
      >
        {{ item.error }}
      </p>
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <Icon
        v-if="item.status === 'processing'"
        name="line-md:loading-loop"
        class="size-5 text-primary"
      />
      <span v-else-if="item.status === 'queued'" class="text-xs text-muted-foreground">Queued</span>
      <UiBadge v-else-if="item.status === 'error'" variant="destructive">Failed</UiBadge>

      <IconButton
        v-if="comparable"
        icon="lucide:columns-2"
        label="Compare with original"
        @click="$emit('compare', item.path)"
      />
      <IconButton
        v-if="best"
        icon="lucide:folder-search"
        :label="written.length > 1 ? 'Show smallest in Finder' : 'Show in Finder'"
        @click="$emit('reveal', best.outputPath)"
      />
      <IconButton
        v-else-if="!running"
        icon="lucide:x"
        label="Remove from list"
        class="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        @click="$emit('remove', item.path)"
      />
    </div>
  </div>
</template>

<script lang="ts" setup>
  import type { ConcreteFormat, QueueItem } from "~/types/squish";

  interface Chip {
    format: ConcreteFormat;
    label: string;
    text: string;
    tone: "best" | "normal" | "muted" | "error";
    title?: string;
  }

  const props = defineProps<{ item: QueueItem; running?: boolean }>();
  defineEmits<{ reveal: [path: string]; remove: [path: string]; compare: [path: string] }>();

  const written = computed(() => writtenOutputs(props.item));
  const best = computed(() => bestOutput(props.item));
  // Re-saved-as-is files have nothing to compare.
  const comparable = computed(() => written.value.some((o) => !o.keptOriginal));

  /** One chip per format that has a result, in a stable order. */
  const chips = computed<Chip[]>(() =>
    FORMAT_ORDER.flatMap((format): Chip[] => {
      const o = props.item.outputs[format];
      if (!o) return [];
      const label = FORMAT_LABEL[format];
      if (o.kind === "skipped") {
        return [
          {
            format,
            label,
            text: "skipped (larger)",
            tone: "muted",
            title: `Would be ${formatBytes(o.wouldBeBytes)}, larger than the original (${formatBytes(props.item.bytes)}).`,
          },
        ];
      }
      if (o.kind === "failed") {
        return [{ format, label, text: "failed", tone: "error", title: o.error }];
      }
      const isBest = best.value?.format === format;
      return [
        {
          format,
          label,
          text: o.keptOriginal
            ? `${formatBytes(o.bytes)} · already optimal`
            : `${formatBytes(o.bytes)} · ${formatPercent(o.savedPercent)}`,
          tone: isBest ? "best" : "normal",
          title: o.keptOriginal
            ? "Re-compressing wouldn’t help, so the original bytes were kept."
            : o.outputPath,
        },
      ];
    }),
  );
</script>
