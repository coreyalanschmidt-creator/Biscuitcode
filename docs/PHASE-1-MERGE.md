# Phase 1 — Merging brand-locking files into the Tauri scaffold

> Read this BEFORE running `pnpm create tauri-app` for the first time. Source-of-truth for which pre-staged files land where in the scaffold output, and what to NOT clobber.

## Why this guide exists

The Phase 1 plan lists ~15 deliverables. Of those, this Windows-side session (no WSL2 access, can't compile) authored the **brand-locking subset** that doesn't depend on the exact Tauri 2.10.x scaffold layout:

| Pre-staged file | Why pre-staged | Goes where in scaffold |
|---|---|---|
| `src/theme/tokens.ts` | Brand palette TS constants | Drop in alongside scaffold's `src/` |
| `src/theme/fonts.css` | Self-hosted `@font-face` rules | Drop in alongside scaffold's `src/` |
| `src/errors/types.ts` | Catalogued error TS union | Drop in alongside scaffold's `src/` |
| `src/errors/ErrorToast.tsx` | Error toast component | Drop in alongside scaffold's `src/` |
| `src/locales/en.json` | i18n bundle (Phase 1 placeholders) | Drop in alongside scaffold's `src/` |
| `tailwind.config.ts` | Tailwind theme with brand tokens | **Replaces** scaffold's blank Tailwind config |
| `src-tauri/biscuitcode-core/Cargo.toml` | Workspace member crate | Add to scaffold's `src-tauri/` workspace |
| `src-tauri/biscuitcode-core/src/lib.rs` | Crate entry point | Add to scaffold's `src-tauri/` workspace |
| `src-tauri/biscuitcode-core/src/palette.rs` | Brand palette Rust constants | Add to scaffold's `src-tauri/` workspace |
| `src-tauri/biscuitcode-core/src/errors.rs` | Catalogued error Rust enum | Add to scaffold's `src-tauri/` workspace |
| `src-tauri/capabilities/{core,fs,shell,http}.json` | Deny-by-default capability ACLs | **Replaces** scaffold's auto-generated capability file (typically `default.json`) |

What this session **deliberately did NOT pre-stage** (the Phase 1 coder writes these against the scaffold output):

- `package.json` — version pinning depends on what `pnpm create tauri-app` selects
- `pnpm-workspace.yaml` — only needed if we go multi-package (we likely won't in v1)
- `tsconfig.json` — scaffold writes a sensible default; we just need to verify it
- `vite.config.ts` — scaffold writes one; we add `vite-plugin-monaco-editor` per Phase 3, not Phase 1
- `postcss.config.js` — scaffold writes one when Tailwind is selected
- `index.html` — scaffold writes one; we tweak it for the BiscuitCode title
- `src/main.tsx` / `src/App.tsx` — scaffold writes; we replace App contents
- `src-tauri/Cargo.toml` (top-level workspace) — scaffold writes; we add `members = ["biscuitcode-core"]` to it
- `src-tauri/tauri.conf.json` — scaffold writes; we patch in:
  - `"identifier": "io.github.Coreyalanschmidt-creator.biscuitcode"`
  - `"productName": "BiscuitCode"`
  - `"bundle.targets": ["deb", "appimage"]`
  - `"bundle.linux.deb.depends": ["libwebkit2gtk-4.1-0", "libgtk-3-0"]`
  - `"bundle.linux.deb.recommends": ["gnome-keyring", "ollama"]`
  - `"app.security.capabilities": ["core", "fs", "shell", "http"]`
- `src-tauri/src/main.rs` — scaffold writes; we wire `biscuitcode-core` and the IPC commands
- `src-tauri/build.rs` — scaffold writes; usually unchanged
- `src-tauri/icons/*` — scaffold creates a placeholder set; we replace from `packaging/icons/`
- Self-hosted font `.woff2` files — Phase 1 coder downloads from official sources (Inter + JetBrains Mono) and drops in `public/fonts/` (or wherever the scaffold expects static assets)
- `.github/workflows/ci.yml` — already pre-staged at repo root
- `LICENSE` — already pre-staged at repo root
- `README.md` — already pre-staged at repo root
- `.gitignore` — already pre-staged at repo root

## Step-by-step merge procedure

Run this **inside WSL2** in the BiscuitCode repo (after `bootstrap-wsl.sh` and `bootstrap-toolchain.sh` succeed):

### 1. Take a snapshot before scaffolding

```bash
git status   # confirm clean working tree
git tag pre-scaffold-snapshot   # so you can `git diff` after to see what scaffold added
```

### 2. Run the Tauri scaffold

```bash
# From the repo root. The scaffold creates files; existing files are NOT
# overwritten unless you confirm — but be careful with the prompts.
pnpm create tauri-app .
```

When prompted:

- **Project name:** `biscuitcode` (already in CLAUDE.md as the executable name)
- **Identifier:** `io.github.Coreyalanschmidt-creator.biscuitcode`
- **Frontend language:** TypeScript / JavaScript
- **Package manager:** pnpm
- **UI template:** React
- **UI flavor:** TypeScript

The scaffold drops files into `./` (and `src-tauri/`).

### 3. Reconcile the duplicates

The scaffold will likely create:
- `src/App.tsx`, `src/main.tsx`, `src/App.css`, `src/index.css` — **let it create them**; you'll edit `App.tsx` to use brand tokens later in this phase.
- `src/assets/` — **let it create**; we don't use it but it's harmless.
- `tailwind.config.ts` (if you select Tailwind in the prompts) — **OVERWRITE with our pre-staged version** that has the full brand palette.
- `src-tauri/capabilities/default.json` — **delete this**; our four `core.json` / `fs.json` / `shell.json` / `http.json` are the explicit replacements.

### 4. Add `biscuitcode-core` to the workspace

Open the **top-level** `src-tauri/Cargo.toml` (the one the scaffold wrote, NOT the one inside `biscuitcode-core/`). Add:

```toml
[workspace]
members = [".", "biscuitcode-core"]
```

(If `[workspace]` already exists, add `"biscuitcode-core"` to its `members` array.)

Then in `src-tauri/Cargo.toml`'s `[dependencies]`:
```toml
biscuitcode-core = { path = "biscuitcode-core" }
```

Verify with `cargo build --workspace` from `src-tauri/` — both crates should compile.

### 5. Patch `src-tauri/tauri.conf.json`

Open the file the scaffold wrote. Apply these changes (paths are JSON-pointer style):

| Path | Set to |
|---|---|
| `productName` | `"BiscuitCode"` |
| `identifier` | `"io.github.Coreyalanschmidt-creator.biscuitcode"` |
| `version` | `"0.1.0"` |
| `app.security.capabilities` | `["core", "fs", "shell", "http"]` |
| `bundle.active` | `true` |
| `bundle.targets` | `["deb", "appimage"]` |
| `bundle.icon` | `["packaging/icons/biscuitcode-32.png", "packaging/icons/biscuitcode-128.png", "packaging/icons/biscuitcode-256.png", "packaging/icons/biscuitcode.ico"]` (paths after rasterizing) |
| `bundle.linux.deb.depends` | `["libwebkit2gtk-4.1-0", "libgtk-3-0"]` |
| `bundle.linux.deb.recommends` | `["gnome-keyring", "ollama"]` |
| `bundle.linux.deb.suggests` | `["rust-analyzer", "typescript-language-server", "pyright", "gopls", "clangd"]` |

### 6. Wire i18n in the entry point

Edit `src/main.tsx` (the scaffold wrote it). Add at the top:
```tsx
import './theme/fonts.css';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './locales/en.json';

i18n.use(initReactI18next).init({
  resources: { en: { translation: en } },
  lng: 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});
```

(Add `i18next` and `react-i18next` to `package.json`: `pnpm add i18next react-i18next`.)

### 7. Edit `src/App.tsx` to render the cocoa-700 background + biscuit accent

Replace scaffold's `App.tsx` body with a minimal shell that demonstrates brand tokens:

```tsx
import { useTranslation } from 'react-i18next';

export default function App() {
  const { t } = useTranslation();
  return (
    <div className="min-h-screen bg-cocoa-700 text-cocoa-50 flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-lg font-semibold text-biscuit-500">
          {t('common.appName')}
        </h1>
        <p className="text-xs text-cocoa-200 mt-2">
          {t('common.tagline')}
        </p>
      </div>
    </div>
  );
}
```

### 8. Rasterize the icon set

```bash
sudo apt install -y librsvg2-bin imagemagick
mkdir -p packaging/icons
cd packaging/icons
for s in 16 32 48 64 128 256 512; do
  rsvg-convert -w $s -h $s biscuitcode.svg -o biscuitcode-$s.png
done
convert biscuitcode-16.png biscuitcode-32.png biscuitcode-48.png biscuitcode-256.png biscuitcode.ico
```

For 16x16 specifically, prefer the hand-tuned variant inline in `biscuitcode-icon-concepts.html` (stroke-width 72, corner radius 96) over a downscale of the master — see `docs/plan.md` Phase 8 deliverables.

### 9. Download self-hosted fonts

```bash
mkdir -p public/fonts

# Inter (SIL OFL — compatible with our MIT app)
# From https://github.com/rsms/inter/releases — grab the latest .zip,
# extract the woff2 files for Regular, Medium, SemiBold weights.

# JetBrains Mono (SIL OFL)
# From https://www.jetbrains.com/lp/mono/ — Download the woff2 set.
```

After downloading, the files at `public/fonts/Inter-Regular.woff2` etc. will be served by Vite at `/fonts/Inter-Regular.woff2` — matching the URLs in `src/theme/fonts.css`.

### 10. Verify Phase 1 ACs

From `docs/plan.md` Phase 1:

```bash
pnpm install
pnpm tauri dev   # opens a WSLg window in under 2s
```

If the window opens with a `#1C1610` background and an `Inter`-rendered `BiscuitCode` heading in `#E8B04C`, Phase 1's first AC is satisfied. Run the rest of the AC checklist from `docs/plan.md`.

### 11. Build the .deb (proves Phase 1's runnable checkpoint)

```bash
pnpm tauri build
ls -la src-tauri/target/release/bundle/deb/
# should show biscuitcode_0.1.0_amd64.deb
```

Verify the AC: `dpkg -s biscuitcode | grep -F 'Version: 0.1.0'` after `sudo dpkg -i`.

## When this guide is wrong

If `pnpm create tauri-app` produces something materially different from what's described here (e.g., the scaffold layout changes between Tauri 2.10 and a future 2.11), update this guide BEFORE proceeding. The plan ACs are the source of truth; this merge guide is just the procedure.
