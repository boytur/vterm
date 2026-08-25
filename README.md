# vterm

A high-performance, GPU-accelerated terminal emulator written in Rust using the GPUI framework (from Zed).

## Features

- **Blazing Fast**: Powered by GPUI for native, GPU-accelerated rendering.
- **TrueColor Support**: Full 256-color and truecolor support natively interpreted from ANSI escapes.
- **Workspaces & Tabs**: Support for multiple isolated workspaces, sidebar navigation, and multi-tabbed terminals with drag-and-drop reordering.
- **Git Integration**: Instantly see your current git branch in the title bar. Click the branch to switch branches effortlessly (with Zed-style dropdown).
- **Customizable Themes**: Comes with 27 built-in light, dark, high-contrast, and colorful themes.
- **Persisted State**: Your tabs, workspaces, theme selections, and terminal sessions are persisted across restarts.
- **In-App Updates**: Detect, download, and relaunch updates without losing terminal sessions.

## Install (macOS)

vterm isn't notarized (no paid Apple Developer ID), so a DMG downloaded via
browser will show "vterm is damaged" on first open — that's Gatekeeper
rejecting an unsigned app that came in quarantined, not a broken build.

Recommended — installs via curl, which doesn't quarantine the download:

```bash
curl -fsSL https://raw.githubusercontent.com/boytur/vterm/master/install.sh | bash
```

Manual alternative: download the [DMG](https://github.com/boytur/vterm/releases/latest/download/vterm-macos.dmg),
drag `vterm.app` to Applications, then run:

```bash
xattr -cr /Applications/vterm.app
```

## Prerequisites

- Rust (latest stable)
- macOS (Linux and Windows support depends on GPUI support)

## Building & Running

1. Clone the repository:
   ```bash
   git clone https://github.com/boytur/vterm.git
   cd vterm
   ```
2. Run the application:
   ```bash
   cargo run --release
   ```

## Development

vterm is a cargo workspace, laid out like Zed's — one crate per domain:

- `crates/vterm/`: Thin binary; window creation and asset loading.
- `crates/terminal/`: Headless terminal core: PTY allocation, shell spawning, VT100 parsing, scrollback.
- `crates/workspace/`: App shell: `Workspace` model, persisted state, sidebar/tab bar/title bar/settings, terminal view rendering.
- `crates/ui/`: Reusable widgets (buttons, modals, text field).
- `crates/theme/`: Built-in color palettes.
- `crates/auto_update/`: In-app update check, download, and relaunch.

Iterate with auto rebuild + relaunch on save:

```bash
./script/dev.sh            # falls back to a polling watcher
cargo install cargo-watch  # optional: instant debounced watching
```

True hot reload isn't possible for GPUI apps (typed render state can't survive a library swap), so the loop relaunches instead.
- `src/workspace.rs`: Core workspace logic, tab management, and git integrations.

### Changelog

Every pull request must update [`CHANGELOG.md`](CHANGELOG.md). Add a concise,
user-facing bullet under `## [Unreleased]` describing what changed. Keep the
entry unreleased until a version tag is created; do not add a version number or
date manually during normal development.

The Changelog Check workflow verifies that the PR changes the `Unreleased`
section and that every existing `v*` tag has a dated release section with at
least one bullet. When a `v*` tag is pushed, the release workflow uses those
entries as GitHub Release notes, opens a changelog PR, and enables auto-merge so
the versioned changelog lands on `master` after required checks pass.
For this to work automatically, enable **Allow GitHub Actions to create and
approve pull requests** in Settings → Actions → General → Workflow permissions.
The release changelog workflow uses its scoped `GITHUB_TOKEN` to create and
merge the generated PR. `RELEASE_TOKEN` is only needed when the version-bump
workflow must push a tag that starts CD.

## CI / CD

- **CI** (`.github/workflows/ci.yml`): builds and runs `cargo test` on macOS for every push/PR to `master`.
- **CD — Release** (`.github/workflows/cd.yml`): on a `v*` tag (or manual dispatch) it builds the optimized macOS binary, packages it as a `.app` in a DMG, and attaches it to a GitHub Release.
- **CD — Landing Page** (`.github/workflows/deploy.yml`): deploys the `landing/` site to Cloudflare Pages whenever it changes.

### Release a new version

First merge the pending feature PRs, then push a semantic-version tag:

```bash
git tag v0.2.5
git push origin v0.2.5
```

The tag starts the macOS build and release. A follow-up changelog PR is opened
and auto-merged after its required checks pass, so the versioned history lands
on `master` without a manual merge.

### Cloudflare Pages setup
Add these repository secrets (Settings → Secrets → Actions):
- `CLOUDFLARE_API_TOKEN` — a token with `Cloudflare Pages: Edit` permissions.
- `CLOUDFLARE_ACCOUNT_ID` — your Cloudflare account ID.

The landing page lives in `landing/` (static HTML + Tailwind + JS). To preview locally:

```bash
cd landing && npx serve .
```

The landing page's release notes are sourced from the root [`CHANGELOG.md`](CHANGELOG.md).
The deploy workflow copies it into the site artifact, so update the root changelog only.

## Landing Page

A static marketing site is available in `landing/`:
- `index.html` — markup (Tailwind via CDN)
- `css/styles.css` — custom styles
- `js/main.js` — animated terminal demo + interactive theme switcher

## Best Practices & Architecture

This project is structured around the `gpui` model architecture. State is stored centrally in the `Workspace` model and UI is derived via `Render`. File parsing (PTY) runs in background async executors communicating via MPSC channels.
