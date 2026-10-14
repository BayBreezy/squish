import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Keep in sync with HELP_EVENT in src-tauri/src/menu.rs. */
const HELP_EVENT = "squish://show-help";

/**
 * State for the "How to use" sheet. Opens from the native Help menu (Rust emits
 * an event), from the header button, and once automatically on first launch.
 */
export function useHelp() {
  const open = ref(false);
  const seen = useLocalStorage("squish-help-seen", false);
  let unlisten: (() => void) | undefined;

  onMounted(async () => {
    if (!seen.value) open.value = true;
    if (isTauri()) unlisten = await listen(HELP_EVENT, () => (open.value = true));
  });

  // Once the user has closed it, don't pop it up again on later launches.
  watch(open, (isOpen) => {
    if (!isOpen) seen.value = true;
  });

  onBeforeUnmount(() => unlisten?.());

  return { open };
}
