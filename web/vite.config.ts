import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      // Perf: direct per-icon imports avoid Vite dev transforming the 1500-module
      // lucide barrel (9.5x first-load win per pagespeedfix 2026-02-12). Prod already
      // tree-shakes (sideEffects:false) — this is a dev-server win.
      "lucide-react/icons": fileURLToPath(
        new URL("./node_modules/lucide-react/dist/esm/icons", import.meta.url),
      ),
    },
  },
  server: {
    port: 3007,
    host: "127.0.0.1",
    warmup: { clientFiles: ["./src/main.tsx", "./src/App.tsx"] },
  },
  preview: { port: 3007, host: "127.0.0.1" },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
    cssMinify: true,
    chunkSizeWarningLimit: 220,
    assetsInlineLimit: 4096,
    rollupOptions: {
      output: {
        manualChunks: {
          react: ["react", "react-dom"],
          vendor: ["lucide-react"],
        },
      },
    },
  },
  optimizeDeps: { include: ["react", "react-dom"] },
});
