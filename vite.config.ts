import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;
const port = Number.parseInt(process.env.DEV_PORT ?? "1420", 10);
const strictPort = process.env.DEV_STRICT_PORT === "true";

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  optimizeDeps: {
    exclude: ["@yoophi/ui"],
  },
  clearScreen: false,
  server: {
    port,
    strictPort,
    host: host || process.env.DEV_HOST || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
