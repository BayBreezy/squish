import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "vue-sonner";

import type { InspectResult, ProcessEvent, QueueItem, Settings } from "~/types/squish";

const IMAGE_EXTENSIONS = [
  "png",
  "jpg",
  "jpeg",
  "webp",
  "heic",
  "heif",
  "gif",
  "bmp",
  "tif",
  "tiff",
];

function revokeThumb(item: QueueItem) {
  if (item.thumb) URL.revokeObjectURL(item.thumb);
}

async function reveal(path: string) {
  try {
    await revealItemInDir(path);
  } catch (e) {
    toast.error("Couldn't open Finder", { description: String(e) });
  }
}

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/**
 * Tell the user what was added and what was left out, so it's clear that only
 * images are processed. Stays quiet when everything added was a clean image.
 */
function announceAdded(added: number, duplicates: number, animated: number, result: InspectResult) {
  const { ignored, skipped } = result;
  if (!ignored && !skipped.length && !duplicates && !animated && added > 0) return;

  const details: string[] = [];
  if (ignored) details.push(`${plural(ignored, "other file")} ignored — only images are processed`);
  if (skipped.length) {
    const names = skipped
      .slice(0, 3)
      .map((s) => s.path.split(/[\\/]/).pop())
      .join(", ");
    details.push(`${plural(skipped.length, "file")} couldn't be read (${names})`);
  }
  if (animated) {
    details.push(`${plural(animated, "animated image")}: only the first frame is kept`);
  }
  if (duplicates) details.push(`${plural(duplicates, "image")} already in the list`);

  const title = added > 0 ? `Added ${plural(added, "image")}` : "No new images added";
  const description = details.join(" · ");
  if (added > 0) toast.info(title, { description });
  else toast.warning(title, { description });
}

export function useImageQueue() {
  const items = ref<QueueItem[]>([]);
  const running = ref(false);
  const inTauri = isTauri();

  const byPath = (path: string) => items.value.find((i) => i.path === path);

  const totals = computed(() => {
    const done = items.value.filter((i) => i.status === "done");
    let original = 0;
    let best = 0;
    let converted = 0;
    let filesWritten = 0;
    let bytesWritten = 0;
    let skipped = 0;
    for (const item of done) {
      const written = writtenOutputs(item);
      filesWritten += written.length;
      bytesWritten += written.reduce((n, o) => n + o.bytes, 0);
      skipped += Object.values(item.outputs).filter((o) => o?.kind === "skipped").length;
      const top = bestOutput(item);
      if (!top) continue;
      converted++;
      original += item.bytes;
      best += top.bytes;
    }
    return {
      done: done.length,
      converted,
      failed: items.value.filter((i) => i.status === "error").length,
      skipped,
      filesWritten,
      bytesWritten,
      original,
      /** Sum of the smallest output per image (not of every output). */
      output: best,
      savedBytes: original - best,
      savedPercent: original > 0 ? (1 - best / original) * 100 : 0,
    };
  });

  const completed = computed(
    () => items.value.filter((i) => i.status === "done" || i.status === "error").length,
  );

  async function loadThumbnails(paths: string[]) {
    // Small worker pool so adding 200 files doesn't flood the backend.
    const queue = [...paths];
    const worker = async () => {
      while (queue.length) {
        const path = queue.shift()!;
        try {
          const buf = await invoke<ArrayBuffer>("make_thumbnail", { path, maxPx: 112 });
          const item = byPath(path);
          if (item) item.thumb = URL.createObjectURL(new Blob([buf], { type: "image/jpeg" }));
        } catch {
          /* thumbnail is cosmetic */
        }
      }
    };
    await Promise.all([worker(), worker(), worker()]);
  }

  async function addPaths(paths: string[]) {
    if (!inTauri || !paths.length) return;
    try {
      const result = await invoke<InspectResult>("inspect_files", { paths });
      const known = new Set(items.value.map((i) => i.path));
      const fresh = result.files.filter((f) => !known.has(f.path));
      items.value.push(...fresh.map((f): QueueItem => ({ ...f, status: "idle", outputs: {} })));
      announceAdded(
        fresh.length,
        result.files.length - fresh.length,
        fresh.filter((f) => f.animated).length,
        result,
      );
      void loadThumbnails(fresh.map((f) => f.path));
    } catch (e) {
      toast.error("Couldn't read those files", { description: String(e) });
    }
  }

  async function pickFiles() {
    if (!inTauri) return;
    const picked = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }],
    });
    if (picked) await addPaths(Array.isArray(picked) ? picked : [picked]);
  }

  async function pickFolder() {
    if (!inTauri) return;
    const picked = await open({ directory: true, multiple: true });
    if (picked) await addPaths(Array.isArray(picked) ? picked : [picked]);
  }

  function remove(path: string) {
    const idx = items.value.findIndex((i) => i.path === path);
    if (idx === -1) return;
    revokeThumb(items.value[idx]!);
    items.value.splice(idx, 1);
  }

  function clear() {
    items.value.forEach(revokeThumb);
    items.value = [];
  }

  async function process(settings: Settings) {
    if (running.value || !items.value.length) return;
    // Re-run everything not already finished successfully? Simpler & predictable: all items.
    const targets = items.value.filter((i) => i.status !== "processing");
    targets.forEach((i) => {
      i.status = "queued";
      i.error = undefined;
      i.outputs = {};
    });
    running.value = true;

    const channel = new Channel<ProcessEvent>();
    // Tauri's Channel API (not a DOM EventTarget): `onmessage` is the only hook.
    // oxlint-disable-next-line unicorn/prefer-add-event-listener
    channel.onmessage = (msg) => {
      switch (msg.event) {
        case "started": {
          const item = byPath(msg.data.path);
          if (item) item.status = "processing";
          break;
        }
        case "output": {
          const item = byPath(msg.data.path);
          if (!item) break;
          item.outputs[msg.data.format] = {
            kind: "written",
            outputPath: msg.data.outputPath,
            bytes: msg.data.outputBytes,
            savedPercent: msg.data.savedPercent,
            keptOriginal: msg.data.keptOriginal,
          };
          break;
        }
        case "skipped": {
          const item = byPath(msg.data.path);
          if (item) {
            item.outputs[msg.data.format] = {
              kind: "skipped",
              wouldBeBytes: msg.data.wouldBeBytes,
            };
          }
          break;
        }
        case "outputFailed": {
          const item = byPath(msg.data.path);
          if (item) item.outputs[msg.data.format] = { kind: "failed", error: msg.data.error };
          break;
        }
        case "done": {
          const item = byPath(msg.data.path);
          if (item) item.status = "done";
          break;
        }
        case "failed": {
          const item = byPath(msg.data.path);
          if (!item) break;
          item.status = "error";
          item.error = msg.data.error;
          break;
        }
        case "finished": {
          items.value.forEach((i) => {
            if (i.status === "queued" || i.status === "processing") i.status = "idle";
          });
          break;
        }
      }
    };

    try {
      await invoke("process_images", {
        paths: targets.map((i) => i.path),
        settings,
        onEvent: channel,
      });
      const t = totals.value;
      const details = [
        `${t.filesWritten} file${t.filesWritten === 1 ? "" : "s"} from ${t.done} image${t.done === 1 ? "" : "s"}`,
        t.skipped ? `${t.skipped} skipped (larger)` : "",
        t.failed ? `${t.failed} failed` : "",
      ].filter(Boolean);
      if (t.converted > 0) {
        toast.success(
          `Saved ${formatBytes(Math.max(t.savedBytes, 0))} (${formatPercent(t.savedPercent)})`,
          { description: details.join(" · ") },
        );
      } else if (t.done > 0) {
        toast.info("Nothing was written", { description: details.join(" · ") });
      } else if (t.failed > 0) {
        toast.error("Nothing could be processed");
      }
    } catch (e) {
      toast.error("Processing failed", { description: String(e) });
      items.value.forEach((i) => {
        if (i.status === "queued" || i.status === "processing") i.status = "idle";
      });
    } finally {
      running.value = false;
    }
  }

  async function cancel() {
    if (running.value) await invoke("cancel_processing");
  }

  onBeforeUnmount(() => items.value.forEach(revokeThumb));

  return {
    items,
    running,
    inTauri,
    totals,
    completed,
    addPaths,
    pickFiles,
    pickFolder,
    remove,
    clear,
    process,
    cancel,
    reveal,
  };
}
