import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 30000,
  retries: 0,
  use: {
    baseURL: "http://localhost:3000",
    screenshot: "on",
    trace: "on-first-retry",
  },
  webServer: {
    command: "cargo leptos serve",
    url: "http://localhost:3000",
    timeout: 120000,
    reuseExistingServer: true,
    cwd: "..",
  },
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
});
