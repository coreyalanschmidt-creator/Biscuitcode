# BiscuitCode — Development Setup

> Phase 0 deliverable. Read this before doing any code-phase work.

BiscuitCode targets **Linux Mint 22 XFCE** (Ubuntu 24.04 noble base). The maintainer's primary machine is **Windows 10**, so all code-phase work happens inside **WSL2 + Ubuntu 24.04** with GUI via WSLg. Releases are smoke-tested on a real Mint 22 XFCE machine before tagging.

## Hard requirements

| Tool | Minimum | Why |
|---|---|---|
| Windows 10/11 with WSL2 | latest | WSLg (GUI) requires Windows 10 21H2 or Windows 11 |
| Ubuntu 24.04 ("noble") | 24.04.x | Mint 22 ships the same noble package base; matching the dev env to the deploy env eliminates one class of "works on my machine" |
| Rust stable | 1.85 | Tauri 2.10.x MSRV |
| Node.js | 20 LTS | Tauri 2.x requires ≥ 18; 20 is the LTS we test against |
| pnpm | 9 | The vision picks pnpm explicitly |
| cargo-tauri-cli | 2.10.1 | Pinned to plan's Tauri version |

You do not need to know how to install these by hand — `scripts/bootstrap-wsl.sh` and `scripts/bootstrap-toolchain.sh` do it for you.

## One-time setup

### 1. Install WSL2 + Ubuntu 24.04 on Windows

From an **administrator PowerShell** on the Windows host:

```powershell
wsl --install -d Ubuntu-24.04
```

If WSL is already installed but you don't have Ubuntu 24.04:

```powershell
wsl --list --online                # confirm Ubuntu-24.04 appears
wsl --install -d Ubuntu-24.04
```

Reboot when prompted. Launch the Ubuntu app from the Start menu and create your Linux user.

### 2. Move the BiscuitCode repo into WSL's native filesystem

**Critical**: do NOT develop from `/mnt/c/Users/.../BiscuitCode`. Tauri builds in `/mnt/c/` are 5–10× slower and trigger inotify-watch exhaustion in `vite dev` and `cargo watch`.

From inside WSL2:

```bash
# Option A: Clone fresh (if the repo is on GitHub)
cd ~
git clone https://github.com/Coreyalanschmidt-creator/biscuitcode.git
cd biscuitcode

# Option B: Copy from the Windows-side checkout (current location)
cp -r /mnt/c/Users/super/Documents/GitHub/BiscuitCode ~/biscuitcode
cd ~/biscuitcode
```

After the move, **verify `realpath .` does NOT start with `/mnt/`**:

```bash
realpath .
# Expected: /home/<your-user>/biscuitcode
```

### 3. Run the bootstrap scripts

In order:

```bash
bash scripts/bootstrap-wsl.sh        # apt installs (system libs)
bash scripts/bootstrap-toolchain.sh  # rustup, nvm, node, pnpm, cargo-tauri
```

`bootstrap-wsl.sh` will sanity-check that you're inside WSL2, on Ubuntu 24.04 (or warn if not), under `$HOME` (not `/mnt/`), and that `busctl` is available. It refuses to proceed if any of those is wrong.

### 4. (Optional) Start gnome-keyring for API-key storage

BiscuitCode stores API keys in libsecret via the `keyring` Rust crate. On a fully-launched XFCE/GNOME session this Just Works. On a headless WSL session, the Secret Service daemon is not running by default and BiscuitCode's onboarding will block until it is. To start it:

```bash
dbus-launch --exit-with-session gnome-keyring-daemon --start --components=secrets
```

If you want this to start automatically on every WSL session, add the line to `~/.bashrc` or set up a systemd user unit. (BiscuitCode itself does not auto-start the daemon — it would need a known password to do so, which is exactly the security hole we're avoiding.)

### 5. Launch a dev window

From `~/biscuitcode`:

```bash
pnpm install
pnpm tauri dev
```

A WSLg window should open showing the BiscuitCode shell. If you see "GLib-GIO-WARNING: Couldn't find the Glib schemas" or similar, `bootstrap-wsl.sh` missed something — re-run it.

## Per-phase workflow (after Phase 0 is complete)

Each code phase follows the same loop:

```bash
# In your Claude Code session (running INSIDE WSL2, not from Windows):
/run-phase <N>
```

The coder subagent reads `docs/plan.md`, locates the named phase, writes a pre-mortem, implements, runs `pnpm test` and `cargo test --workspace`, and updates the plan with `Complete` / `Partial` / `Blocked`.

After each `Complete` phase, commit and push:

```bash
git add -A
git commit -m "feat(phase-N): <one-line description>"
git push
```

Smoke-test on the secondary Mint 22 XFCE machine when prompted by Phase 10 / release prep.

## Why these constraints exist

- **WSL2 + Ubuntu 24.04 specifically**: matches the Mint 22 deploy target's package base (noble). A different Ubuntu version means different `webkit2gtk` versions, different `libfuse2t64` situation, different system libraries. We minimize variables.
- **Project under `$HOME`, not `/mnt/c/`**: WSL filesystem performance and inotify behavior. Non-negotiable.
- **`gnome-keyring` for secrets**: the vision forbids plaintext key storage. There is no fallback. A future maintainer searching "Tauri secrets" may find `tauri-plugin-stronghold` — DO NOT USE IT (deprecated in v3; see `docs/adr/0001-no-stronghold.md`).
- **Mint 22 XFCE smoke testing on real hardware**: WSLg is good but not perfect — XFCE-specific tray rendering, GTK theme integration, `apt remove` cleanup, and Whisker-menu placement need to be verified on the actual environment users install into. The maintainer has a secondary Mint 22 XFCE machine for this.
