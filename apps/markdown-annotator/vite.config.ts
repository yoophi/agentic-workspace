import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

const devServerPort = Number.parseInt(process.env.VITE_DEV_SERVER_PORT ?? "1421", 10);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@examples": path.resolve(__dirname, "../../examples/markdown-annotator"),
    },
  },
  clearScreen: false,
  server: {
    port: Number.isInteger(devServerPort) ? devServerPort : 1421,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2020",
    minify: !process.env.TAURI_DEBUG,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
