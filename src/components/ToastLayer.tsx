// src/components/ToastLayer.tsx
//
// Single mount-point for transient toasts. Consumes:
//   - `biscuitcode:toast`        — info/warning notices (e.g. shortcut placeholders)
//   - `biscuitcode:error-toast`  — catalogued errors (rendered via ErrorToast)
//
// Phase 2 deliverable. Toasts auto-dismiss after 5s; error toasts stay
// until manually dismissed (per the catalogue's design contract).
//
// Mount once, in App.tsx, alongside <WorkspaceGrid />.

import { useEffect, useState } from 'react';
import { ErrorToast } from '../errors/ErrorToast';
import { type AppErrorPayload, isAppError } from '../errors/types';

type InfoToast = { id: number; kind: 'info' | 'warn'; text: string };
type ErrorToastEntry = { id: number; payload: AppErrorPayload };

export function ToastLayer() {
  const [infoToasts, setInfoToasts] = useState<InfoToast[]>([]);
  const [errorToasts, setErrorToasts] = useState<ErrorToastEntry[]>([]);

  useEffect(() => {
    let nextId = 1;

    const onInfo = (e: Event) => {
      const detail = (e as CustomEvent).detail as { kind?: 'info' | 'warn'; text?: string };
      if (!detail?.text) return;
      const id = nextId++;
      setInfoToasts((ts) => [...ts, { id, kind: detail.kind ?? 'info', text: detail.text! }]);
      window.setTimeout(() => {
        setInfoToasts((ts) => ts.filter((t) => t.id !== id));
      }, 5000);
    };

    const onError = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (!isAppError(detail)) return;
      const id = nextId++;
      setErrorToasts((ts) => [...ts, { id, payload: detail }]);
    };

    window.addEventListener('biscuitcode:toast', onInfo);
    window.addEventListener('biscuitcode:error-toast', onError);
    return () => {
      window.removeEventListener('biscuitcode:toast', onInfo);
      window.removeEventListener('biscuitcode:error-toast', onError);
    };
  }, []);

  return (
    <div
      aria-live="polite"
      className="fixed bottom-8 right-4 z-50 flex flex-col gap-2 pointer-events-none"
    >
      {infoToasts.map((t) => (
        <div
          key={t.id}
          className={`
            pointer-events-auto px-3 py-2 rounded-md shadow-lg text-sm
            ${t.kind === 'warn'
              ? 'bg-cocoa-600 border border-accent-warn/40 text-cocoa-50'
              : 'bg-cocoa-600 border border-cocoa-400 text-cocoa-50'}
          `}
        >
          {t.text}
        </div>
      ))}
      {errorToasts.map((t) => (
        <div key={t.id} className="pointer-events-auto">
          <ErrorToast
            error={t.payload}
            onDismiss={() => setErrorToasts((cur) => cur.filter((c) => c.id !== t.id))}
          />
        </div>
      ))}
    </div>
  );
}
