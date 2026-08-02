import { defineConfig } from "vitest/config";

// Real Remotion renders spin up a bundler pass and a headless Chrome
// instance per test file; on a cold cache (first browser download) that can
// take minutes. Generous timeouts here trade test speed for never faking a
// pass on a render that's still legitimately in flight.
export default defineConfig({
  test: {
    environment: "node",
    testTimeout: 5 * 60 * 1000,
    hookTimeout: 5 * 60 * 1000,
  },
});
