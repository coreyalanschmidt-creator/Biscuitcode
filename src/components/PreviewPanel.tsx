// src/components/PreviewPanel.tsx
//
// Phase 2 ships an empty shell. Phase 7 wires:
//   - Markdown (react-markdown + remark-gfm + mermaid + KaTeX)
//   - HTML (sandboxed iframe)
//   - Images (PNG/JPG/WebP/SVG/GIF with zoom/pan)
//   - PDF (pdf.js via react-pdf)
//   - Notebook (.ipynb) read-only render
//   - Auto-open trigger from Phase 6b AI edits to .md/.html/.svg/image

import { useTranslation } from 'react-i18next';

export function PreviewPanel() {
  const { t } = useTranslation();
  return (
    <section
      aria-label="Preview"
      className="h-full bg-cocoa-700 border-l border-cocoa-500 overflow-auto"
    >
      <header className="px-3 py-2 text-xs font-semibold uppercase tracking-wider text-cocoa-200">
        Preview
      </header>
      <div className="px-3 py-4 text-sm text-cocoa-300">
        <em>Preview pane wires in Phase 7 (markdown / HTML / images / PDF / notebooks).</em>
      </div>
    </section>
  );
}
