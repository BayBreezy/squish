import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";

/**
 * Native file drops. HTML5 drag events don't expose real file paths inside the
 * webview, so we listen to Tauri's drag-drop event instead.
 */
export function useTauriDrop(onDrop: (paths: string[]) => void) {
  const isOver = ref(false);
  let unlisten: (() => void) | undefined;

  onMounted(async () => {
    if (!isTauri()) return;
    unlisten = await getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === "enter" || p.type === "over") isOver.value = true;
      else if (p.type === "leave") isOver.value = false;
      else if (p.type === "drop") {
        isOver.value = false;
        if (p.paths.length) onDrop(p.paths);
      }
    });
  });

  onBeforeUnmount(() => unlisten?.());

  return { isOver };
}
