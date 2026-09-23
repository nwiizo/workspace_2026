/// <reference types="vitest/config" />
import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  build: { rollupOptions: { input: ["index.html", "_model-audit.html"] } },
  test: {
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
});
