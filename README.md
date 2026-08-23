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

## Best Practices & Architecture

This project is structured around the `gpui` model architecture. State is stored centrally in the `Workspace` model and UI is derived via `Render`. File parsing (PTY) runs in background async executors communicating via MPSC channels.

