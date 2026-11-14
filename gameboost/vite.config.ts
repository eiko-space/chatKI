import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// Tauri erwartet einen festen Port und soll Rust-Dateien nicht beobachten.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  // Eigene (leere) PostCSS-Konfiguration: verhindert, dass Vite eine fremde
  // postcss.config aus einem übergeordneten Ordner lädt. Tailwind läuft über das Vite-Plugin.
  css: { postcss: {} },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
