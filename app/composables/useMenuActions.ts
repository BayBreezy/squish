import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Keep in sync with ACTION_EVENT and the ACTION_* ids in src-tauri/src/menu.rs. */
const ACTION_EVENT = "squish://menu-action";

export type MenuAction = "add-images" | "add-folder" | "process" | "clear" | "shortcuts";

/** Run a handler when the matching File-menu item is chosen. */
export function useMenuActions(handlers: Record<MenuAction, () => void | Promise<void>>) {
  let unlisten: (() => void) | undefined;

  onMounted(async () => {
    if (!isTauri()) return;
    unlisten = await listen<MenuAction>(ACTION_EVENT, (event) => {
      void handlers[event.payload]?.();
    });
  });

  onBeforeUnmount(() => unlisten?.());
}
