import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Static SPA — no Node runtime in prod. Dev proxies /v1 to local daemon/backend if present.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: false,
    proxy: {
      "/v1": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
  preview: {
    port: 4173,
  },
  build: {
    outDir: "dist",
    sourcemap: false,
    emptyOutDir: true,
  },
});
