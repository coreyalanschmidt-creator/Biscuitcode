// src/components/PreviewPanel.tsx
//
// Phase 7 deliverable: Preview panel.
// Renders the currently active file based on its extension:
//   - Markdown (.md):     react-markdown + remark-gfm + rehype-highlight + rehype-katex
//   - HTML (.html):       sandboxed iframe (sandbox="allow-scripts" only)
//   - Images:             PNG/JPG/WebP/SVG/GIF — CSS zoom/pan; animated GIFs honor <img>
//   - PDF (.pdf):         react-pdf single-page with prev/next
//   - Notebook (.ipynb):  read-only cell render — no execution controls
//
// Auto-open: listens for biscuitcode:preview-file event from Phase 6b / EditorArea.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import rehypeHighlight from 'rehype-highlight';
import rehypeKatex from 'rehype-katex';
import { Document, Page, pdfjs } from 'react-pdf';
import { invoke } from '@tauri-apps/api/core';
import { useEditorStore } from '../state/editorStore';

// Configure pdf.js worker.
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url,
).toString();

// ---------- Types ----------

type PreviewMode = 'markdown' | 'html' | 'image' | 'pdf' | 'notebook' | 'none';

interface NotebookCell {
  cell_type: 'markdown' | 'code' | 'raw';
  source: string[];
  outputs?: NotebookOutput[];
}

interface NotebookOutput {
  output_type: string;
  text?: string[];
  data?: Record<string, unknown>;
}

// ---------- Helpers ----------

function detectMode(path: string): PreviewMode {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  if (ext === 'md' || ext === 'markdown') return 'markdown';
  if (ext === 'html' || ext === 'htm') return 'html';
  if (['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif'].includes(ext)) return 'image';
  if (ext === 'pdf') return 'pdf';
  if (ext === 'ipynb') return 'notebook';
  return 'none';
}

// ---------- Markdown Preview ----------

function MarkdownPreview({ content }: { content: string }) {
  return (
    <div className="prose prose-invert prose-sm max-w-none px-4 py-4 text-cocoa-100">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        rehypePlugins={[rehypeHighlight, rehypeKatex]}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}

// ---------- HTML Preview ----------

function HtmlPreview({ content }: { content: string }) {
  const { t } = useTranslation();
  const blobUrl = useMemo(() => {
    const blob = new Blob([content], { type: 'text/html' });
    return URL.createObjectURL(blob);
  }, [content]);

  useEffect(() => {
    return () => URL.revokeObjectURL(blobUrl);
  }, [blobUrl]);

  return (
    <iframe
      src={blobUrl}
      sandbox="allow-scripts"
      title={t('preview.title')}
      className="w-full h-full border-0"
      aria-label={t('preview.title')}
    />
  );
}

// ---------- Image Preview ----------

function ImagePreview({ path }: { path: string }) {
  const { t } = useTranslation();
  const [zoom, setZoom] = useState(1.0);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const dragStart = useRef<{ x: number; y: number; ox: number; oy: number } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  // For SVG: use <img> so it renders in the webview context.
  // For all others: use asset:// protocol via Tauri fs (already in scope).
  // We read the file via fs_read and create a data URL.
  const [dataUrl, setDataUrl] = useState<string | null>(null);

  useEffect(() => {
    if (ext === 'svg') {
      // SVG can be loaded as text and shown as a data URL.
      invoke<string>('fs_read', { path })
        .then((svg) => {
          const blob = new Blob([svg], { type: 'image/svg+xml' });
          setDataUrl(URL.createObjectURL(blob));
        })
        .catch(() => setDataUrl(null));
    } else {
      // For binary images, use Tauri convertFileSrc (asset protocol).
      import('@tauri-apps/api/core').then(({ convertFileSrc }) => {
        setDataUrl(convertFileSrc(path));
      });
    }
  }, [path, ext]);

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    setZoom((z) => Math.max(0.1, Math.min(8, z - e.deltaY * 0.001)));
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    setDragging(true);
    dragStart.current = { x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y };
  }, [offset]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragging || !dragStart.current) return;
    setOffset({
      x: dragStart.current.ox + (e.clientX - dragStart.current.x),
      y: dragStart.current.oy + (e.clientY - dragStart.current.y),
    });
  }, [dragging]);

  const handleMouseUp = useCallback(() => {
    setDragging(false);
    dragStart.current = null;
  }, []);

  if (!dataUrl) return <div className="px-3 py-4 text-xs text-cocoa-400">Loading…</div>;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Zoom controls */}
      <div className="flex items-center gap-1 px-2 py-1 border-b border-cocoa-600 shrink-0">
        <button
          aria-label={t('preview.zoomOut')}
          className="px-1.5 py-0.5 text-xs text-cocoa-300 hover:text-cocoa-50 bg-cocoa-600 rounded"
          onClick={() => setZoom((z) => Math.max(0.1, z - 0.25))}
        >−</button>
        <button
          aria-label={t('preview.zoomReset')}
          className="px-1.5 py-0.5 text-xs text-cocoa-300 hover:text-cocoa-50 bg-cocoa-600 rounded"
          onClick={() => { setZoom(1); setOffset({ x: 0, y: 0 }); }}
        >
          {Math.round(zoom * 100)}%
        </button>
        <button
          aria-label={t('preview.zoomIn')}
          className="px-1.5 py-0.5 text-xs text-cocoa-300 hover:text-cocoa-50 bg-cocoa-600 rounded"
          onClick={() => setZoom((z) => Math.min(8, z + 0.25))}
        >+</button>
      </div>
      {/* Canvas */}
      <div
        ref={containerRef}
        className="flex-1 overflow-hidden flex items-center justify-center bg-cocoa-800 cursor-grab"
        style={{ cursor: dragging ? 'grabbing' : 'grab' }}
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        <img
          src={dataUrl}
          alt={path.split('/').pop()}
          style={{
            transform: `translate(${offset.x}px, ${offset.y}px) scale(${zoom})`,
            transformOrigin: 'center',
            maxWidth: 'none',
            userSelect: 'none',
            pointerEvents: 'none',
          }}
        />
      </div>
    </div>
  );
}

// ---------- PDF Preview ----------

function PdfPreview({ content }: { content: string }) {
  const { t } = useTranslation();
  const [numPages, setNumPages] = useState<number>(0);
  const [page, setPage] = useState(1);

  // content is base64 — react-pdf handles this via `data` prop.
  const pdfData = useMemo(() => `data:application/pdf;base64,${btoa(content)}`, [content]);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="flex items-center gap-2 px-2 py-1 border-b border-cocoa-600 shrink-0">
        <button
          aria-label={t('preview.prevPage')}
          className="px-1.5 py-0.5 text-xs text-cocoa-300 hover:text-cocoa-50 bg-cocoa-600 rounded disabled:opacity-40"
          onClick={() => setPage((p) => Math.max(1, p - 1))}
          disabled={page <= 1}
        >‹</button>
        <span className="text-xs text-cocoa-300">
          {t('preview.pageOf', { current: page, total: numPages })}
        </span>
        <button
          aria-label={t('preview.nextPage')}
          className="px-1.5 py-0.5 text-xs text-cocoa-300 hover:text-cocoa-50 bg-cocoa-600 rounded disabled:opacity-40"
          onClick={() => setPage((p) => Math.min(numPages, p + 1))}
          disabled={page >= numPages}
        >›</button>
      </div>
      <div className="flex-1 overflow-auto flex justify-center py-2">
        <Document
          file={pdfData}
          onLoadSuccess={({ numPages: n }) => setNumPages(n)}
          loading={<p className="text-xs text-cocoa-400 p-4">Loading PDF…</p>}
          error={<p className="text-xs text-red-400 p-4">Failed to load PDF.</p>}
        >
          <Page pageNumber={page} width={500} />
        </Document>
      </div>
    </div>
  );
}

// ---------- Notebook Preview ----------

function NotebookPreview({ content }: { content: string }) {
  const { t } = useTranslation();
  let cells: NotebookCell[] = [];
  try {
    const nb = JSON.parse(content);
    cells = (nb.cells ?? []) as NotebookCell[];
  } catch {
    return <p className="px-3 py-4 text-xs text-red-400">Invalid notebook JSON.</p>;
  }

  return (
    <div className="flex flex-col gap-2 px-2 py-3 overflow-auto">
      {cells.map((cell, idx) => (
        <div
          key={idx}
          className="border border-cocoa-600 rounded overflow-hidden"
          role="region"
          aria-label={t('preview.notebookCell', { n: idx + 1 })}
        >
          {/* Cell type badge */}
          <div className="px-2 py-0.5 bg-cocoa-800 border-b border-cocoa-600 flex items-center gap-1">
            <span className="text-xs text-cocoa-400">
              {cell.cell_type === 'code'
                ? t('preview.notebookCode')
                : t('preview.notebookMarkdown')}
            </span>
          </div>

          {/* Cell source */}
          {cell.cell_type === 'markdown' ? (
            <div className="px-3 py-2 prose prose-invert prose-sm max-w-none text-cocoa-100">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>
                {cell.source.join('')}
              </ReactMarkdown>
            </div>
          ) : (
            <pre
              className="px-3 py-2 text-xs overflow-x-auto"
              style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace", color: '#E0D3BE' }}
            >
              {cell.source.join('')}
            </pre>
          )}

          {/* Outputs (read-only text/plain/stream only) */}
          {cell.outputs && cell.outputs.length > 0 && (
            <div className="border-t border-cocoa-700 bg-cocoa-900">
              <div className="px-2 py-0.5 text-xs text-cocoa-500">{t('preview.notebookOutput')}</div>
              {cell.outputs.map((out, oi) => {
                const text =
                  out.text?.join('') ??
                  (typeof out.data?.['text/plain'] === 'string'
                    ? out.data['text/plain']
                    : Array.isArray(out.data?.['text/plain'])
                    ? (out.data['text/plain'] as string[]).join('')
                    : '');
                return (
                  <pre
                    key={oi}
                    className="px-3 py-1 text-xs text-cocoa-200 overflow-x-auto"
                    style={{ fontFamily: "'JetBrains Mono', 'Ubuntu Mono', monospace" }}
                  >
                    {text}
                  </pre>
                );
              })}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

// ---------- PreviewPanel ----------

export function PreviewPanel() {
  const { t } = useTranslation();
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const tabs = useEditorStore((s) => s.tabs);
  const [content, setContent] = useState<string>('');
  const [previewPath, setPreviewPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Determine active path: either from a preview-specific event or from active tab.
  const activePath = previewPath ?? (activeTabId ? tabs.find((t) => t.id === activeTabId)?.path ?? null : null);
  const mode = activePath ? detectMode(activePath) : 'none';

  // Load file content whenever activePath or mode changes.
  useEffect(() => {
    if (!activePath || mode === 'none') {
      setContent('');
      return;
    }
    setLoading(true);
    invoke<string>('fs_read', { path: activePath })
      .then((c) => { setContent(c); setLoading(false); })
      .catch(() => { setContent(''); setLoading(false); });
  }, [activePath, mode]);

  // Listen for biscuitcode:preview-file event (from AI edits via Phase 6b).
  useEffect(() => {
    const handler = (e: Event) => {
      const { path } = (e as CustomEvent).detail ?? {};
      if (path) setPreviewPath(path);
    };
    window.addEventListener('biscuitcode:preview-file', handler);
    return () => window.removeEventListener('biscuitcode:preview-file', handler);
  }, []);

  const renderContent = () => {
    if (loading) {
      return <p className="px-3 py-4 text-xs text-cocoa-400">{t('editor.loading')}</p>;
    }
    if (!activePath || mode === 'none') {
      return (
        <p className="px-3 py-4 text-xs text-cocoa-400 italic">
          {activePath ? t('preview.unsupportedType') : t('preview.noFile')}
        </p>
      );
    }
    switch (mode) {
      case 'markdown': return <MarkdownPreview content={content} />;
      case 'html':     return <HtmlPreview content={content} />;
      case 'image':    return <ImagePreview path={activePath} />;
      case 'pdf':      return <PdfPreview content={content} />;
      case 'notebook': return <NotebookPreview content={content} />;
      default:         return null;
    }
  };

  return (
    <section
      aria-label={t('preview.title')}
      className="h-full flex flex-col bg-cocoa-700 border-l border-cocoa-500 overflow-hidden"
    >
      <header className="px-3 py-2 flex items-center gap-2 border-b border-cocoa-600 shrink-0">
        <span className="text-xs font-semibold uppercase tracking-wider text-cocoa-200 flex-1">
          {t('preview.title')}
        </span>
        {activePath && (
          <span
            className="text-xs text-cocoa-400 truncate max-w-[120px]"
            title={activePath}
          >
            {activePath.split('/').pop()}
          </span>
        )}
      </header>
      <div className="flex-1 overflow-auto">
        {renderContent()}
      </div>
    </section>
  );
}
