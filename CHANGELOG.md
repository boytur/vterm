# Changelog

All notable changes to vterm are documented here.

## [Unreleased]

- Add copyable curl install commands to the landing page hero and install section.

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
