import tailwindcss from "@tailwindcss/vite";
// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "latest",
  devtools: { enabled: true },
  ssr: false,
  telemetry: false,

  // Rust build output is huge; watching it causes EMFILE errors.
  ignore: ["**/src-tauri/**"],

  modules: [
    "@nuxtjs/color-mode",
    "motion-v/nuxt",
    "@vueuse/nuxt",
    "@nuxt/icon",
    "@nuxt/fonts",
    "vue-sonner/nuxt",
  ],

  imports: {
    imports: [
      { from: "tailwind-variants", name: "tv" },
      { from: "tailwind-variants", name: "VariantProps", type: true },
      { from: "vue-sonner", name: "toast", as: "useSonner" },
    ],
  },

  colorMode: {
    storageKey: "squish-color-mode",
    classSuffix: "",
  },

  icon: {
    clientBundle: { scan: true, sizeLimitKb: 0 },
    mode: "svg",
    class: "shrink-0",
    fetchTimeout: 2000,
    serverBundle: "local",
  },

  css: ["~/assets/css/tailwind.css"],

  vite: {
    // Keep Tauri CLI output readable and the port stable (devUrl is fixed).
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
    server: { strictPort: true },
    plugins: [tailwindcss()],
    optimizeDeps: {
      include: [
        "@tauri-apps/api/core",
        "@tauri-apps/api/webview",
        "@tauri-apps/plugin-dialog",
        "@tauri-apps/plugin-opener",
        "@vue/devtools-core",
        "reka-ui",
        "tailwind-variants",
      ],
    },
  },
});
