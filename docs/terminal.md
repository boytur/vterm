# Terminal Architecture

How vterm's embedded terminal works: the emulator core, how colors flow from
the app theme to running CLIs, and why each layer lives where it does.

The design deliberately mirrors [Zed](https://github.com/zed-industries/zed)'s
terminal (`crates/terminal`), which embeds the same emulator core.

## Layers

```
┌─ OWNED BY VTERM ─────────────────────────────────────────────┐
│  PTY lifecycle     portable_pty spawn / resize / kill        │
│  Env & identity    TERM=xterm-256color COLORTERM=truecolor   │
│                    TERM_PROGRAM=vterm COLORFGBG=15;0         │
│  Byte filtering    AltScreenFilter (scrollback hygiene)      │
│  Query replies     Event::ColorRequest → live palette        │
│  Rendering         gpui grid painting from TerminalSnapshot  │
│  Theme injection   Workspace → TerminalColors handle         │
├─ EMBEDDED CORE ──────────────────────────────────────────────┤
│  alacritty_terminal  parser, grid, scrollback, alternate     │
│  (Zed's fork pin)    screen, mouse/keyboard protocols        │
└───────────────────────────────────────────────────────────────┘
```

Source of truth: `crates/terminal/src/lib.rs`. The workspace crate renders
from snapshots and never touches the emulator directly.

## Emulator core

We embed `alacritty_terminal` (Zed's fork, pinned revision) instead of
maintaining our own VT parser. Bytes from the PTY are fed through
`vte::ansi::Processor::advance` under a `FairMutex`, batched per read burst
so heavy output costs one lock + repaint.

Key consequences of owning the layers *around* the core:

- **OSC color queries are answered by us**, not lost. vt100 (the previous
  core) silently swallowed `OSC 10/11/4` probes; apps like opencode key their
  dark/light palette off an OSC 11 reply and would guess wrong.
- **Scrollback purge** (`clear`) uses native CSI 3J semantics via
  `Term::clear_screen(ClearMode::Saved)`. The `AltScreenFilter` still detects
  macOS's home+2J clear pair and full-screen app entry, flushing history the
  way tmux does.
- **Rendering** works on a `TerminalSnapshot`: an immutable copy of exactly
  what the user sees (viewport including scrolled-back history), taken under
  one lock. Wide-char spacer cells are preserved so the grid stays aligned.

## Color pipeline

One shared handle guarantees that what an app *probes* equals what the user
*sees*:

```
Theme { bg_main, text_primary, ansi[0..16] }
        │  mapped once in workspace.rs (theme_terminal_colors)
        ▼
TerminalColors  ─── single Arc, cloned into every PtyTerminal ──┐
        │                                                       │
        │ set() on theme switch — applies live to               │
        │ already-running terminals                             │
        ▼                                                       ▼
OSC responder                                    gpui renderer
query_color(index):                              Palette::from_channels()
  1. app-set override (runtime OSC 4) wins       paints default fg/bg,
  2. else TerminalColors.resolve()               ANSI 0–15, cube/grayscale
     (fg/bg/ansi + standard ramps)               from the same values
```

Rules encoded here:

- **Cube (16–231) and grayscale (232–255) ramps stay xterm-standard** — they
  are computed constants everywhere. Grayscale steps by 10 (8..=238); the old
  renderer stepped by 11, miscoloring near-white grays.
- **Dim variants** (query indices 259–266) resolve to half-strength base
  colors; there is no separate dim palette.
- **Replies never bake in at spawn time.** Colors resolve at query/paint
  time, so switching themes updates terminals that are already running.

## Theme injection

`Workspace.terminal_colors` holds the active mapping. It is seeded at startup,
passed into every new `PtyTerminal`, and re-set in `set_theme`. Because both
the responder and renderer read the same handle, a CLI probing OSC 11 after a
theme switch gets the new background immediately — which is what lets apps
like opencode pick a readable dark/light palette against any vterm theme.

## Session management (macOS)

New terminals spawn the shell **directly on the PTY** — no multiplexer in
between (same as Zed). GNU screen interprets the byte stream and swallows
OSC queries, which broke color detection for every child process.

Screen remains only for **reattaching** to persisted sessions: when restoring
a workspace whose saved session socket still exists, vterm attaches with
`screen -xRR`. Trade-off: brand-new sessions do not survive app restarts;
restored ones do as long as their screen session is alive.

## Environment contract

Set on every spawned shell (`crates/terminal/src/lib.rs`):

| Variable | Value | Why |
|---|---|---|
| `TERM` | `xterm-256color` | Full capability advertisement |
| `COLORTERM` | `truecolor` | Truecolor SGR support |
| `TERM_PROGRAM` | `vterm` | Own identity; foreign vars stripped |
| `COLORFGBG` | `15;0` | Dark-surface hint for CLIs that skip probes |

Foreign terminal identity vars (`ZED_TERM`, `ITERM_*`, `WEZTERM_*`,
`KITTY_*`) are removed so CLIs do not enable integrations meant for another
host.

## Testing

`cargo test -p terminal` covers the byte filter, snapshot semantics
(attributes, cursor, scrollback offset views), history clearing, event
raising for color queries, and palette resolution across the full query
index space. `cargo bench -p terminal` keeps parse throughput honest.
