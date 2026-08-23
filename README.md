# vterm

A high-performance, GPU-accelerated terminal emulator written in Rust using the GPUI framework (from Zed).

## Features

- **Blazing Fast**: Powered by GPUI for native, GPU-accelerated rendering.
- **TrueColor Support**: Full 256-color and truecolor support natively interpreted from ANSI escapes.
- **Workspaces & Tabs**: Support for multiple isolated workspaces, sidebar navigation, and multi-tabbed terminals.
- **Git Integration**: Instantly see your current git branch in the title bar. Click the branch to switch branches effortlessly (with Zed-style dropdown).
- **Customizable Themes**: Comes with multiple built-in themes (Ubuntu, Zed Dark, Dracula, Nord, Gruvbox, Tokyo Night, Catppuccin, etc.).
- **Persisted State**: Your tabs, workspaces, and theme selections are persisted across restarts.

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

- `src/components/`: Reusable UI components (Sidebar, Tab Bar, Terminal View).
- `src/pty.rs`: Handles pseudoterminal allocation, shell spawning, and VT100 parsing.
- `src/state.rs`: Manages application state serialization.
- `src/theme.rs`: Defines color palettes for different themes.
- `src/workspace.rs`: Core workspace logic, tab management, and git integrations.

## CI / CD

- **CI** (`.github/workflows/ci.yml`): builds and runs `cargo test` on macOS for every push/PR to `master`.
- **CD — Release** (`.github/workflows/cd.yml`): on a `v*` tag (or manual dispatch) it builds the optimized macOS binary, packages it as a `.app` in a zip, and attaches it to a GitHub Release.
- **CD — Landing Page** (`.github/workflows/deploy.yml`): deploys the `landing/` site to Cloudflare Pages whenever it changes.

### Release a new version
```bash
git tag v0.2.0
git push origin v0.2.0
```

### Cloudflare Pages setup
Add these repository secrets (Settings → Secrets → Actions):
- `CLOUDFLARE_API_TOKEN` — a token with `Cloudflare Pages: Edit` permissions.
- `CLOUDFLARE_ACCOUNT_ID` — your Cloudflare account ID.

The landing page lives in `landing/` (static HTML + Tailwind + JS). To preview locally:

```bash
cd landing && npx serve .
```

## Landing Page

A static marketing site is available in `landing/`:
- `index.html` — markup (Tailwind via CDN)
- `css/styles.css` — custom styles
- `js/main.js` — animated terminal demo + interactive theme switcher

## Best Practices & Architecture

This project is structured around the `gpui` model architecture. State is stored centrally in the `Workspace` model and UI is derived via `Render`. File parsing (PTY) runs in background async executors communicating via MPSC channels.

