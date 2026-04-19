import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import monacoEditorPlugin from 'vite-plugin-monaco-editor';

// Tauri's dev server must listen on a fixed port.
// Do NOT use process.env.TAURI_DEV_HOST — use 0.0.0.0 to bind all interfaces
// in WSLg so the Tauri webview can reach the dev server.
export default defineConfig({
  plugins: [
    react(),
    // Phase 3: Monaco workers. Only the base editor worker is bundled at
    // startup; language workers (ts, css, json, html) are added on-demand
    // when a file of that type is first opened.
    monacoEditorPlugin({ languageWorkers: ['editorWorkerService'] }),
  ],

  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: '0.0.0.0',
    watch: {
      // Avoid infinite rebuild loops on WSL2's inotify.
      ignored: ['**/src-tauri/**'],
    },
  },

  build: {
    // Tauri uses Chromium (WebKitGTK) so target ES2021.
    target: ['es2021', 'chrome105', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },

  // Env prefix for Tauri — only vars starting with TAURI_ are exposed to the
  // frontend bundle.
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
});
