<template>
  <div class="flex h-screen flex-col overflow-hidden">
    <header class="flex h-14 shrink-0 items-center justify-between border-b px-5">
      <div class="flex items-center gap-2.5">
        <AppLogo class="size-7" />
        <span class="text-base font-semibold tracking-tight">Squish</span>
        <span class="hidden text-xs text-muted-foreground sm:inline">
          Local image compressor &amp; converter
        </span>
      </div>
      <div class="flex items-center gap-1">
        <IconButton
          size="icon"
          side="bottom"
          icon="lucide:circle-help"
          label="How to use Squish"
          @click="helpOpen = true"
        />
        <IconButton
          size="icon"
          side="bottom"
          :icon="$colorMode.value === 'dark' ? 'lucide:sun' : 'lucide:moon'"
          :label="$colorMode.value === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'"
          @click="$colorMode.preference = $colorMode.value === 'dark' ? 'light' : 'dark'"
        />
      </div>
    </header>

    <!-- Percentages are of the window width. The settings panel may take 30–50%. -->
    <UiSplitter auto-save-id="squish-layout" class="min-h-0 flex-1">
      <UiSplitterPanel :default-size="66" :min-size="50" class="min-w-0">
        <main class="relative flex h-full min-w-0 flex-col">
          <div v-if="!queue.items.value.length" class="flex-1 p-6">
            <DropZone
              :active="isOver"
              :in-tauri="queue.inTauri"
              @pick="queue.pickFiles"
              @pick-folder="queue.pickFolder"
            />
          </div>

          <template v-else>
            <!-- Summary / actions bar -->
            <div class="flex flex-wrap items-center gap-3 border-b px-5 py-3">
              <div class="min-w-0 flex-1">
                <template v-if="queue.running.value">
                  <p class="text-sm font-medium">
                    Squishing {{ queue.completed.value }} / {{ queue.items.value.length }}…
                  </p>
                  <UiProgress
                    class="mt-2 h-1.5"
                    :model-value="(queue.completed.value / queue.items.value.length) * 100"
                  />
                </template>
                <template v-else-if="queue.totals.value.done > 0">
                  <p class="text-base font-semibold text-primary">
                    <template v-if="queue.totals.value.converted > 0">
                      Saved {{ formatBytes(Math.max(queue.totals.value.savedBytes, 0)) }} ({{
                        formatPercent(queue.totals.value.savedPercent)
                      }})
                    </template>
                    <template v-else>No files written</template>
                  </p>
                  <p class="text-xs text-muted-foreground">
                    <template v-if="queue.totals.value.converted > 0">
                      {{ formatBytes(queue.totals.value.original) }} →
                      {{ formatBytes(queue.totals.value.output) }} (smallest per image) ·
                    </template>
                    {{ queue.totals.value.filesWritten }} file{{
                      queue.totals.value.filesWritten === 1 ? "" : "s"
                    }}
                    written
                    <template v-if="queue.totals.value.skipped">
                      · {{ queue.totals.value.skipped }} skipped (larger)
                    </template>
                  </p>
                </template>
                <template v-else>
                  <p class="text-sm font-medium">
                    {{ queue.items.value.length }} image{{
                      queue.items.value.length > 1 ? "s" : ""
                    }}
                    ready
                  </p>
                  <p class="text-xs text-muted-foreground">
                    {{ formatBytes(totalBytes) }} total
                    <template v-if="settings.formats.length > 1">
                      · {{ queue.items.value.length }} × {{ settings.formats.length }} formats → up
                      to {{ queue.items.value.length * settings.formats.length }} files
                    </template>
                    <template v-else> · adjust settings, then squish </template>
                  </p>
                </template>
              </div>

              <div class="flex items-center gap-2">
                <template v-if="!queue.running.value">
                  <UiButton variant="ghost" size="sm" @click="queue.clear">Clear</UiButton>
                  <UiButton variant="outline" size="sm" @click="queue.pickFiles">
                    <Icon name="lucide:plus" />
                    Add
                  </UiButton>
                  <UiButton size="sm" @click="queue.process(settings)">
                    <Icon name="lucide:zap" />
                    {{ queue.totals.value.done ? "Squish again" : "Squish" }}
                  </UiButton>
                </template>
                <UiButton v-else variant="outline" size="sm" @click="queue.cancel">
                  <Icon name="lucide:square" />
                  Cancel
                </UiButton>
              </div>
            </div>

            <UiScrollArea class="min-h-0 flex-1">
              <div class="space-y-0.5 p-3">
                <FileRow
                  v-for="item in queue.items.value"
                  :key="item.path"
                  :item="item"
                  :running="queue.running.value"
                  @reveal="queue.reveal"
                  @compare="openCompare"
                  @remove="queue.remove"
                />
              </div>
            </UiScrollArea>

            <div
              v-if="isOver"
              class="pointer-events-none absolute inset-0 z-10 m-3 flex items-center justify-center rounded-2xl border-2 border-dashed border-primary bg-primary/10 text-lg font-semibold text-primary"
            >
              Drop to add
            </div>
          </template>
        </main>
      </UiSplitterPanel>

      <UiSplitterHandle with-handle />

      <UiSplitterPanel :default-size="34" :min-size="30" :max-size="50" class="bg-sidebar">
        <UiScrollArea type="auto" class="h-full">
          <SettingsPanel
            v-model="settings"
            @preset="applyPreset"
            @reset="reset"
            @choose-output-dir="chooseOutputDir"
          />
        </UiScrollArea>
      </UiSplitterPanel>
    </UiSplitter>

    <HowToSheet v-model:open="helpOpen" />
    <CompareDialog v-model:open="compareOpen" :item="compareItem" />
    <ShortcutsDialog v-model:open="shortcutsOpen" />
  </div>
</template>

<script lang="ts" setup>
  import { open } from "@tauri-apps/plugin-dialog";

  import type { QueueItem } from "~/types/squish";

  const { settings, applyPreset, reset } = useSettings();
  const queue = useImageQueue();
  const { isOver } = useTauriDrop((paths) => queue.addPaths(paths));
  const { open: helpOpen } = useHelp();

  const shortcutsOpen = ref(false);
  const compareOpen = ref(false);
  const compareItem = ref<QueueItem | null>(null);
  function openCompare(path: string) {
    compareItem.value = queue.items.value.find((i) => i.path === path) ?? null;
    compareOpen.value = compareItem.value !== null;
  }

  // File menu (⌘O, ⇧⌘O, ⌘↩): same actions as the buttons in the window.
  useMenuActions({
    "add-images": () => queue.pickFiles(),
    "add-folder": () => queue.pickFolder(),
    process: () => queue.process(settings.value),
    clear: () => {
      if (!queue.running.value) queue.clear();
    },
    shortcuts: () => {
      shortcutsOpen.value = true;
    },
  });

  const totalBytes = computed(() => queue.items.value.reduce((n, i) => n + i.bytes, 0));

  async function chooseOutputDir() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") settings.value.outputDir = dir;
  }

  useHead({ title: "Squish" });
</script>
