# Installing BiscuitCode

> User-facing install guide. For developer setup (WSL2, toolchains, etc.) see `docs/DEV-SETUP.md` instead.

BiscuitCode v1 ships as a `.deb` (primary) and an `.AppImage` (portable) for Linux Mint 22 XFCE x86_64. Other distros that match the Ubuntu 24.04 noble base (Ubuntu 24.04, Debian Bookworm-derivatives with the right webkit2gtk version) should also work but aren't tested.

## Recommended: `.deb` install

The `.deb` is the cleanest install — it integrates with apt/dpkg, registers a Whisker-menu entry under Development, installs icons at every resolution, and uninstalls cleanly.

### From the GitHub release page

1. Visit `https://github.com/Coreyalanschmidt-creator/biscuitcode/releases/latest`
2. Download `BiscuitCode_<version>_amd64.deb` and `BiscuitCode_<version>_amd64.deb.asc`.
3. (Optional but recommended) Verify the GPG signature:
   ```bash
   gpg --verify BiscuitCode_<version>_amd64.deb.asc BiscuitCode_<version>_amd64.deb
   ```
   Should report "Good signature" against the BiscuitCode release key.
4. Install via GDebi (double-click in Files manager) OR via terminal:
   ```bash
   sudo dpkg -i BiscuitCode_<version>_amd64.deb
   sudo apt -f install   # picks up any missing dependencies
   ```
5. Launch from Whisker menu → Development → BiscuitCode.

### Dependencies the `.deb` declares

| Type | Package | What for |
|---|---|---|
| Depends | `libwebkit2gtk-4.1-0` | Tauri 2 webview runtime |
| Depends | `libgtk-3-0`          | UI framework |
| Recommends | `gnome-keyring`    | Stores API keys safely (BiscuitCode blocks API-key entry without it) |
| Recommends | `ollama`           | Local LLMs (Gemma 4 etc.) — not required if you only use Anthropic / OpenAI |
| Suggests | `rust-analyzer`, `typescript-language-server`, `pyright`, `gopls`, `clangd` | Per-language code intelligence; install only the ones for languages you use |

### Uninstall

```bash
sudo apt remove biscuit-code
```

This removes the binary, `.desktop` entry, icons, and the `/usr/bin/biscuitcode` symlink. **Your settings, conversations, and snapshots are preserved** at:
- `~/.config/biscuitcode/`  (settings.json)
- `~/.local/share/biscuitcode/` (conversations.db)
- `~/.cache/biscuitcode/`    (snapshots, model cache)

To remove those too: `rm -rf ~/.config/biscuitcode ~/.local/share/biscuitcode ~/.cache/biscuitcode`. (`apt purge` does NOT delete user data — by design.)

## Alternative: AppImage

For users who prefer not to use apt, or who want to try BiscuitCode without affecting their system:

1. Download `BiscuitCode-<version>-x86_64.AppImage` from the releases page.
2. Make it executable:
   ```bash
   chmod +x BiscuitCode-<version>-x86_64.AppImage
   ```
3. **Install `libfuse2t64` first** (Ubuntu 24.04 / Mint 22 require this):
   ```bash
   sudo apt install libfuse2t64
   ```
   Without this, the AppImage will refuse to launch with a confusing "FUSE not found" error.
4. Run it:
   ```bash
   ./BiscuitCode-<version>-x86_64.AppImage
   ```

The AppImage is portable — copy it anywhere, run it from a USB stick, etc. Auto-updates via the Tauri updater plugin (in-app: Settings → About → Check for updates).

## First-run onboarding

On first launch BiscuitCode walks you through a 3-screen setup:

1. **Welcome** — logo + tagline.
2. **Pick models** — add at least one provider:
   - **Anthropic (Claude)** — paste your API key from https://console.anthropic.com/settings/keys
   - **OpenAI (ChatGPT)** — paste your API key from https://platform.openai.com/api-keys
   - **Ollama (local models, including Gemma 4)** — click "Install Ollama" if you don't already have it; the app shows you the verbatim install command and runs it only after you confirm.
3. **Open a folder** — file picker. You can also click "Continue without a folder" if you're just exploring.

You should be coding within 2 minutes of double-clicking the `.deb`.

## Updating

- **AppImage**: in-app `Settings → About → Check for updates`. If a newer version exists, BiscuitCode prompts; on accept it downloads + replaces the AppImage and restarts.
- **`.deb`**: `Settings → About → Check for updates` notifies you of a newer release and opens the GitHub releases page so you can download the new `.deb`. **No auto-install of `.deb`** because `dpkg -i` requires sudo and BiscuitCode never asks for sudo.

## Troubleshooting

### "BiscuitCode needs a system keyring" toast on first launch

Your Linux session doesn't have a Secret Service daemon running. On Mint 22 XFCE this is unusual but can happen if you run `startxfce4` from `xinit` bypassing PAM. Fix:

```bash
sudo apt install gnome-keyring libsecret-1-0 libsecret-tools
# Then either log out + log in OR start the daemon manually:
dbus-launch --exit-with-session gnome-keyring-daemon --start --components=secrets
```

Click Retry in BiscuitCode after.

### "Gemma 4 isn't available on your Ollama version" toast

Your Ollama is older than 0.20.0 (the version that added Gemma 4 support, released 2026-04-03). The toast includes the upgrade command:

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

Until you upgrade, BiscuitCode falls back to Gemma 3 transparently — agent-mode tool calling will be less reliable on Gemma 3 base; switch to `qwen2.5-coder:7b` if you need stable tool use and have 12+ GB RAM.

### App icon shows a generic gear / question mark in the Whisker menu

XFCE icon cache wasn't refreshed after install. Run:
```bash
gtk-update-icon-cache -t /usr/share/icons/hicolor
```
Then log out + back in (or restart the panel: `xfce4-panel -r`).

### AppImage refuses to launch with "FUSE not found" or "couldn't extract appimage"

You're missing `libfuse2t64`:
```bash
sudo apt install libfuse2t64
```

## Reporting issues

File a bug at `https://github.com/Coreyalanschmidt-creator/biscuitcode/issues` with:
- BiscuitCode version (`Settings → About`)
- Mint version (`lsb_release -a`)
- XFCE version (`xfce4-about` or the Whisker about page)
- Steps to reproduce
- Any error code surfaced (e.g., `E007 GemmaVersionFallback`)

The error code lets us look up the exact failure path in `docs/ERROR-CATALOGUE.md` rather than guessing.
