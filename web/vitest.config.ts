import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "lucide-react/icons": fileURLToPath(
        new URL("./node_modules/lucide-react/dist/esm/icons", import.meta.url),
      ),
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    reporters: ["default"],
  },
});
