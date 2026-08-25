# Changelog

All notable changes to vterm are documented here.

## [Unreleased]

- Enable GitHub Actions to create and approve pull requests so the release changelog automation works again.
- Add native Unicode and IME input handling for Thai, Arabic, Latin, and other composed text.
- Preserve combining-character deletion in macOS terminal sessions.

## [0.3.1] - 2026-08-24

- Keep the terminal surface dark regardless of the selected app theme, so CLI tools (opencode, codex, vim, …) render with their normal dark palettes and stay readable.
- Spawned shells now identify as `TERM_PROGRAM=vterm`; the launching terminal's identity variables (`ZED_TERM`, `ITERM_*`, `WEZTERM_*`, `KITTY_*`) are no longer inherited.
- Advertise a dark background via `COLORFGBG` so CLIs that skip color queries pick their dark palette.

## [0.3.0] - 2026-08-24

- Reorder terminal tabs by dragging them, with an accent-colored insertion line showing exactly where the tab will be dropped.
- Fix inverted scroll direction when using the trackpad or mouse wheel.
- Hide the terminal cursor while viewing scrollback history.
- Make `clear` actually wipe history instead of leaving it reachable by scrolling up.
- Fix Cmd+A and other editing shortcuts being stolen by terminal shortcuts while a rename dialog is open.
- Add `script/dev.sh` for auto rebuild-and-relaunch on save during development.
- Add a real text field to rename dialogs with caret, arrow keys, selection, and Cmd+A/C/X/V editing support.
- Make Cmd+A select the entire terminal screen so it can be copied with Cmd+C.
- Smooth out typing and scrolling while commands produce heavy output by batching PTY data before repainting.
- Fix scrollback not working when a command outputs more lines than fit on screen (e.g. `ls -la` in a large directory).
- Snap the terminal view back to the bottom when typing while scrolled up.
- Add copyable curl install commands to the landing page hero and install section.
- Render release notes and release history on the landing page from the project changelog.
- Redesign the landing page with responsive product-focused content, SEO metadata, and structured FAQ data.
- Let the curl installer replace the app bundle while vterm is still running.
- Restore terminal placement and PTY sizing for new and resumed terminals.
- Restore scrollback visibility and git branch detection from the active shell.
- Improve ANSI black contrast in the dark theme.
- Restructure the app into a Zed-style cargo workspace (vterm, terminal, workspace, ui, theme, auto_update crates).
- Stop recoloring CLI output with the app theme: ANSI 0–15 use a fixed xterm-standard palette, and bold text no longer forces a white foreground.
- Re-apply `TERM=xterm-256color` / `COLORTERM=truecolor` after the macOS screen wrapper replaces the spawn command, restoring truecolor for CLIs.

## [0.2.4] - 2026-08-23

- Add 27 built-in themes, including light, high-contrast, and VS Code-inspired palettes.
- Add a settings dialog with Appearance, Terminal, and About sections.
- Detect updates in the background and show a persistent Settings badge.
- Download updates in-app and relaunch while preserving terminal sessions, workspaces, and tabs.
- Make installation safer with version-correct app metadata, rollback, and old-process handling.
- Document the changelog workflow and validate release notes in pull requests.

## [0.2.3] - 2026-08-23

- Reopen the application when it is activated after all windows were closed.

## [0.2.2] - 2026-08-23

- Add a curl-based macOS installer that avoids the browser quarantine warning.
- Add an Open Graph preview image and stronger landing-page metadata.

## [0.2.1] - 2026-08-23

- Clean up clippy and test warnings so CI passes cleanly.

## [0.2.0] - 2026-08-23

- Add terminal quality-of-life features, including scrollback, URL links, and keyboard shortcuts.

## [0.1.4] - 2026-08-23

- Fix update version comparison so the app does not repeatedly offer the same release.

## [0.1.3] - 2026-08-23

- Download and install updates in place without redirecting to a browser.
- Add the landing-page logo and example recording.

## [0.1.2] - 2026-08-23

- Revert the portable-pty change and fix a terminal loading freeze.
- Add the application icon and a Gatekeeper workaround to the landing page.
- Add a custom POSIX PTY/parser benchmark.

## [0.1.1] - 2026-08-23

- Package the macOS app as a drag-to-Applications DMG.
- Add the first working CI/CD and Cloudflare Pages deployment path.

## [0.1.0] - 2026-08-23

- Publish the first vterm release with a GPUI terminal, workspaces, tabs, git integration, and a landing page.
