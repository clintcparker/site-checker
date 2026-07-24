// Import from "vitest/config", not "vite" — only vitest's defineConfig knows
// about the `test` key.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "happy-dom",
  },
});
