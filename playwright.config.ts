// playwright.config.ts
//
// Playwright configuration — Phase 10 infrastructure placeholder.
//
// The current e2e specs (tests/e2e/agent-mode-demo.spec.ts and
// tests/e2e/agent-tool-card-render.spec.ts) are Vitest + @testing-library/react
// tests that mock the Tauri IPC layer. They are run via `pnpm test:e2e` which
// invokes `vitest run --config vitest.e2e.config.ts`.
//
// This file exists so that:
//   - `npx playwright install --with-deps chromium` in CI has a valid config to
//     reference.
//   - Future real browser-automation specs authored against a `vite preview` or
//     `tauri dev` server can be added to tests/e2e/ and picked up here.
//
// When migrating to real Playwright specs, replace `command` and `url` below
// with the actual dev-server invocation and uncomment `webServer`.

import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'tests/e2e',
  timeout: 30_000,
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL: 'http://localhost:1420',
  },
  // webServer is commented out because the current e2e specs do not need a
  // running browser/server — they run via Vitest. Uncomment when real
  // Playwright browser tests are added.
  //
  // webServer: {
  //   command: 'pnpm tauri dev',
  //   url: 'http://localhost:1420',
  //   reuseExistingServer: !process.env.CI,
  // },
});
