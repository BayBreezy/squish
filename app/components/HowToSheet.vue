<template>
  <UiSheet v-model:open="open">
    <UiSheetContent side="right" class="w-full overflow-y-auto sm:max-w-md">
      <UiSheetX />
      <UiSheetHeader>
        <UiSheetTitle>How to use Squish</UiSheetTitle>
        <UiSheetDescription>
          Shrink and convert images in four steps. Everything happens on your computer - nothing is
          uploaded.
        </UiSheetDescription>
        <UiGradientDivider class="mt-4" />
      </UiSheetHeader>

      <ol class="space-y-8 px-4">
        <li v-for="(step, i) in steps" :key="step.title" class="flex gap-6">
          <span
            class="flex size-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary"
          >
            {{ i + 1 }}
          </span>
          <div class="space-y-1">
            <h3 class="text-sm font-semibold">
              {{ step.title }}
            </h3>
            <p class="text-sm text-muted-foreground">{{ step.body }}</p>
          </div>
        </li>
      </ol>

      <UiSeparator />

      <div class="space-y-3 px-4 pb-6">
        <h3 class="text-sm font-semibold">Good to know</h3>
        <ul class="space-y-2 text-sm text-muted-foreground">
          <li v-for="tip in tips" :key="tip" class="flex gap-2">
            <Icon name="lucide:check" class="mt-0.5 size-4 text-primary" />
            <span>{{ tip }}</span>
          </li>
        </ul>
        <p class="pt-1 text-xs text-muted-foreground">
          Reopen this guide any time from the Help menu or with ⌘/ (Ctrl+/ on Windows and Linux).
        </p>
      </div>
    </UiSheetContent>
  </UiSheet>
</template>

<script lang="ts" setup>
  const open = defineModel<boolean>("open", { default: false });

  const steps = [
    {
      icon: "lucide:images",
      title: "Add your images",
      body: "Drag files or whole folders onto the window, or use Choose images / Choose folder. PNG, JPEG, WebP, HEIC (iPhone photos), GIF, BMP and TIFF are supported.",
    },
    {
      icon: "lucide:sliders-horizontal",
      title: "Pick your settings",
      body: "Start with a preset (Balanced is a good default). Tick one or more output formats — every image is converted to each — then adjust quality and optionally set a maximum width or height.",
    },
    {
      icon: "lucide:zap",
      title: "Press Squish",
      body: "Images are processed in parallel with live progress. You can cancel at any time.",
    },
    {
      icon: "lucide:folder-search",
      title: "Find your results",
      body: "Each file shows how much it shrank. Use the folder button to reveal it in Finder or Explorer. Results are saved next to the original with a “-squished” suffix.",
    },
  ];

  const tips = [
    "Your originals are never changed or overwritten.",
    "Wide-gamut photos (Display P3, as taken by iPhones) are converted to sRGB so colours look right everywhere.",
    "Photo metadata, including GPS location, is removed by default. Turn on Keep metadata under Output to copy it.",
    "Animated GIFs and WebPs keep only their first frame.",
    "WebP and AVIF give the smallest photos; AVIF is the slowest to encode.",
    "With several formats selected you get one file per format; the smallest is highlighted. Conversions that would be bigger than the original are skipped unless you turn that on under Output.",
    "PNG “Reduce colours” works like TinyPNG. Turn on Lossless to keep every pixel exactly.",
    "“Already optimal” means re-compressing wouldn’t help, so the original bytes were kept.",
    "Change “Save to” under Output to collect results in one folder.",
    "Use the compare button on a finished row to drag a divider between the original and the result.",
    "Shortcuts: ⌘O add images, ⇧⌘O add a folder, ⌘↩ start squishing. Press ⌘K to see them all.",
  ];
</script>
