<template>
  <UiTooltip>
    <UiTooltipTrigger as-child>
      <UiButton v-bind="$attrs" :variant="variant" :size="size" :aria-label="label">
        <Icon :name="icon" />
      </UiButton>
    </UiTooltipTrigger>
    <UiTooltipContent :side="side">{{ label }}</UiTooltipContent>
  </UiTooltip>
</template>

<script lang="ts" setup>
  import type { ButtonProps } from "~/components/Ui/Button.vue";

  /**
   * Icon-only button. Always renders a tooltip (and aria-label) from `label`,
   * so an icon button can never ship without one.
   */
  defineOptions({ inheritAttrs: false });

  withDefaults(
    defineProps<{
      /** Iconify name, e.g. `lucide:x`. */
      icon: string;
      /** Tooltip text and accessible name. */
      label: string;
      variant?: ButtonProps["variant"];
      size?: ButtonProps["size"];
      side?: "top" | "right" | "bottom" | "left";
    }>(),
    { variant: "ghost", size: "icon-sm", side: "top" },
  );
</script>
