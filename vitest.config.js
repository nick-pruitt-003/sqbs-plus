import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

// Separate from vite.config.js on purpose: tests compile Svelte components
// directly, without the SvelteKit plugin's routing/SSR machinery.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
    },
    // Component tests need Svelte's browser build, not its SSR build.
    conditions: ["browser"],
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.{test,spec}.{ts,js}"],
    restoreMocks: true,
  },
});
