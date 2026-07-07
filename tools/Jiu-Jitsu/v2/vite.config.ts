/// <reference types="vitest/config" />
import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  test: {
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
});
