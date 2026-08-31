import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  test: {
    projects: [
      {
        plugins: [vue()],
        test: {
          name: "lib",
          environment: "node",
          include: ["src/lib/*.test.ts"],
        },
      },
      {
        plugins: [vue()],
        test: {
          name: "components",
          environment: "happy-dom",
          include: ["src/components/**/*.test.ts", "src/views/**/*.test.ts"],
        },
      },
    ],
  },
});
