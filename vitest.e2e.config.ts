// vitest.e2e.config.ts
//
// Vitest config for the tests/e2e/ suite.
//
// The e2e specs (agent-mode-demo.spec.ts, agent-tool-card-render.spec.ts) are
// authored as Vitest + @testing-library/react tests that mock the Tauri IPC
// layer. They are excluded from the default `pnpm test` run (vitest.config.ts
// excludes tests/e2e/**) to keep the unit-test cycle fast.
//
// `pnpm test:e2e` runs this config, which includes only tests/e2e/**.
//
// Playwright is installed as a devDependency and playwright.config.ts exists
// for CI infrastructure (the `npx playwright install --with-deps chromium` step
// is wired in ci.yml). The e2e specs remain Vitest-style because they test the
// full React component integration layer with a mocked IPC — they do not require
// a running Tauri binary or browser automation. A future phase can migrate to
// real Playwright browser tests against `tauri dev`; the runner infrastructure
// is in place.

import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: false,
    include: ['tests/e2e/**/*.spec.ts', 'tests/e2e/**/*.spec.tsx'],
  },
});
