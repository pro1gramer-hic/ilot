import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

// Recommended Tauri config (https://v2.tauri.app/start/frontend/vite/), Windows only.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/target/**", "**/.claude/**"],
    },
  },
  build: {
    target: "chrome105",
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // Four pages: the island, the mini-player window, Mochi's window and the bar of the
    // small video player.
    rolldownOptions: {
      input: {
        main: fileURLToPath(new URL("index.html", import.meta.url)),
        mini: fileURLToPath(new URL("mini.html", import.meta.url)),
        buddy: fileURLToPath(new URL("buddy.html", import.meta.url)),
        "video-bar": fileURLToPath(new URL("video-bar.html", import.meta.url)),
      },
    },
  },
});
