<template>
  <UiDialog v-model:open="open">
    <UiDialogContent class="flex max-h-[88vh] flex-col gap-4 sm:max-w-lg">
      <UiDialogHeader>
        <UiDialogTitle>Keyboard shortcuts</UiDialogTitle>
        <UiDialogDescription>
          Everything you can do from the keyboard. Most are also in the menu bar.
        </UiDialogDescription>
      </UiDialogHeader>

      <div class="min-h-0 flex-1 space-y-5 overflow-y-auto pr-1">
        <section v-for="group in groups" :key="group.title" class="space-y-2">
          <h3 class="text-sm font-semibold">{{ group.title }}</h3>
          <ul class="divide-y rounded-lg border">
            <li
              v-for="item in group.items"
              :key="item.label"
              class="flex items-center justify-between gap-4 px-3 py-2"
            >
              <span class="text-[13px]">{{ item.label }}</span>
              <span class="flex shrink-0 items-center gap-1">
                <kbd
                  v-for="(key, i) in item.keys.map(display)"
                  :key="i"
                  class="min-w-6 rounded-md border bg-muted px-1.5 py-0.5 text-center font-sans text-xs text-muted-foreground"
                >
                  {{ key }}
                </kbd>
              </span>
            </li>
          </ul>
        </section>
      </div>
    </UiDialogContent>
  </UiDialog>
</template>

<script lang="ts" setup>
  const open = defineModel<boolean>("open", { default: false });

  /** `mod` is ⌘ on macOS and Ctrl elsewhere. */
  type Key = "mod" | "shift" | "alt" | "enter" | "left" | "right" | (string & {});
  interface Shortcut {
    label: string;
    keys: Key[];
  }
  interface Group {
    title: string;
    items: Shortcut[];
  }

  const isMac = import.meta.client && navigator.userAgent.includes("Mac");

  const SYMBOLS: Record<string, [mac: string, other: string]> = {
    mod: ["⌘", "Ctrl"],
    shift: ["⇧", "Shift"],
    alt: ["⌥", "Alt"],
    enter: ["↩", "Enter"],
    left: ["←", "←"],
    right: ["→", "→"],
  };
  function display(key: Key): string {
    const symbol = SYMBOLS[key];
    return symbol ? symbol[isMac ? 0 : 1] : key;
  }

  const groups = computed<Group[]>(() => {
    const list: Group[] = [
      {
        title: "Images",
        items: [
          { label: "Add images", keys: ["mod", "O"] },
          { label: "Add a folder", keys: ["mod", "shift", "O"] },
          { label: "Start squishing", keys: ["mod", "enter"] },
        ],
      },
      {
        title: "Help",
        items: [
          { label: "How to use Squish", keys: ["mod", "/"] },
          { label: "Keyboard shortcuts", keys: ["mod", "K"] },
        ],
      },
      {
        title: "Before / after viewer",
        items: [
          { label: "Move the divider", keys: ["left", "right"] },
          { label: "Close", keys: ["Esc"] },
        ],
      },
      {
        title: "Text fields",
        items: [
          { label: "Undo", keys: ["mod", "Z"] },
          { label: "Redo", keys: ["mod", "shift", "Z"] },
          { label: "Cut / Copy / Paste", keys: ["mod", "X · C · V"] },
          { label: "Select all", keys: ["mod", "A"] },
        ],
      },
    ];
    if (isMac) {
      list.push({
        title: "Window",
        items: [
          { label: "Minimise", keys: ["mod", "M"] },
          { label: "Close window", keys: ["mod", "W"] },
          { label: "Hide Squish", keys: ["mod", "H"] },
          { label: "Quit Squish", keys: ["mod", "Q"] },
        ],
      });
    }
    return list;
  });
</script>
