import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  outputDir: "./test-results",
  use: {
    baseURL: "http://127.0.0.1:4173",
    locale: "zh-CN",
    viewport: { width: 900, height: 640 },
  },
  webServer: {
    command: "pnpm exec vite --port 4173 --strictPort --host 127.0.0.1",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: true,
    timeout: 60_000,
  },
  projects: [
    {
      name: "msedge",
      use: { browserName: "chromium", channel: "msedge" },
    },
  ],
});
