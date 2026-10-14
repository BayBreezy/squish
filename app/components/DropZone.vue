<template>
  <div
    :class="[
      'flex h-full min-h-72 flex-col items-center justify-center gap-5 rounded-2xl border-2 border-dashed p-8 text-center transition-colors',
      active ? 'border-primary bg-primary/5' : 'border-border bg-muted/30',
    ]"
  >
    <div class="flex size-16 items-center justify-center rounded-2xl bg-primary/10 text-primary">
      <Icon :name="active ? 'lucide:download' : 'lucide:images'" class="size-8" />
    </div>
    <div class="space-y-1">
      <h2 class="text-xl font-semibold tracking-tight">
        {{ active ? "Drop to add" : "Drop images here" }}
      </h2>
      <p class="text-sm text-muted-foreground">
        PNG, JPEG, WebP, HEIC (iPhone photos), GIF, BMP or TIFF — or whole folders. Everything stays
        on your computer.
      </p>
    </div>
    <div class="flex flex-wrap justify-center gap-2">
      <UiButton @click="$emit('pick')">
        <Icon name="lucide:file-plus" />
        Choose images
      </UiButton>
      <UiButton variant="outline" @click="$emit('pickFolder')">
        <Icon name="lucide:folder-open" />
        Choose folder
      </UiButton>
    </div>
    <p v-if="!inTauri" class="text-xs text-destructive">
      Running in a browser — file access only works in the Squish desktop app.
    </p>
  </div>
</template>

<script lang="ts" setup>
  defineProps<{ active?: boolean; inTauri?: boolean }>();
  defineEmits<{ pick: []; pickFolder: [] }>();
</script>
