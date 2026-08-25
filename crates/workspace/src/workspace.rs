use crate::components::settings::{SettingsSection, render_settings};
use crate::components::sidebar::render_sidebar;
use crate::components::tab_bar::render_tab_bar;
use crate::components::terminal::render_terminal_view;
use crate::components::title_bar::render_title_bar;
use crate::state::AppState;
use gpui::prelude::*;
use gpui::*;
use std::ops::Range;

use terminal::PtyTerminal;
use ui::{button::button, modal::modal_overlay, text_input::TextField};

fn process_cwd(pid: u32) -> Option<String> {
    let child_pids = std::process::Command::new("pgrep")
        .args(["-P", &pid.to_string()])
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.trim().parse::<u32>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for child_pid in child_pids {
        if let Some(cwd) = process_cwd(child_pid) {
            return Some(cwd);
        }
    }

    let output = std::process::Command::new("lsof")
        .args(["-p", &pid.to_string(), "-a", "-d", "cwd", "-F", "n"])
        .output()
        .ok()?;

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(str::to_string))
}

pub struct Workspace {
    pub state: AppState,
    pub focus_handle: gpui::FocusHandle,
    pub terminals: Vec<Vec<Entity<PtyTerminal>>>,
    pub tab_context_menu: Option<(usize, gpui::Point<gpui::Pixels>)>,
    pub tab_drop_target: Option<usize>,
    pub dir_drop_target: Option<usize>,
    pub dir_context_menu: Option<(usize, gpui::Point<gpui::Pixels>)>,
    pub renaming_tab_modal: Option<(usize, TextField)>,
    pub renaming_dir_modal: Option<(usize, TextField)>,
    pub git_branch: String,
    pub git_branches: Vec<String>,
    pub theme_menu_open: bool,
    pub settings_open: bool,
    pub settings_section: SettingsSection,
    pub branch_menu_open: bool,
    pub alert_modal: Option<(String, String)>,
    pub selection: Option<((u16, u16), (u16, u16))>,
    pub selecting: bool,
    pub toast: Option<String>,
    pub update_info: Option<auto_update::UpdateInfo>,
    pub update_checking: bool,
    pub update_downloading: bool,
    pub update_download_progress: Option<f32>,
    pub update_staged: Option<std::path::PathBuf>,
    pub update_installing: bool,
    pub update_error: Option<String>,
    pub ime_composition: String,
    /// Theme-derived palette injected into every terminal: OSC query
    /// replies and cell rendering both read from it, so they stay in sync.
    pub terminal_colors: terminal::TerminalColors,
}

/// Maps the app theme onto the terminal surface.
fn theme_terminal_colors(t: &theme::Theme) -> terminal::TerminalColors {
    let channel = |v: f32| (v * 255.0).round() as u8;
    let channels = |c: gpui::Rgba| [channel(c.r), channel(c.g), channel(c.b)];
    let fg = channels(t.text_primary);
    let bg = channels(t.bg_main);
    let ansi = t.ansi.map(channels);
    terminal::TerminalColors::new(fg, bg, ansi)
}

impl Workspace {
    pub fn get_active_terminal_cwd(&self, cx: &mut Context<Self>) -> Option<String> {
        let ws_idx = self.state.active_workspace;
        let ws = self.state.workspaces.get(ws_idx)?;
        let term_idx = ws.active_term;
        let term_entity = self.terminals.get(ws_idx)?.get(term_idx)?;
        let pid = term_entity.read(cx).child_pid?;
        process_cwd(pid)
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut state = AppState::new();
        let (rows, cols) = Self::terminal_size(window.viewport_size(), state.font_size);
        let mut state_changed = false;
        let terminal_colors = theme_terminal_colors(&state.theme);
        let mut terminals = Vec::new();
        for ws in &mut state.workspaces {
            let mut ws_terms = Vec::new();
            for term_data in &mut ws.terminals {
                let cwd = term_data.cwd.clone();
                let session_name = term_data.session_name.clone();
                let colors = terminal_colors.clone();
                let term = cx
                    .new(|cx| PtyTerminal::new_with_cwd(cwd, session_name, rows, cols, colors, cx));
                let actual_session = term.read(cx).session_name.clone();
                if term_data.session_name != actual_session {
                    term_data.session_name = actual_session;
                    state_changed = true;
                }
                cx.observe(&term, |_, _, cx| cx.notify()).detach();
                ws_terms.push(term);
            }
            terminals.push(ws_terms);
        }
        if state_changed {
            state.save().ok();
        }

        let mut this = Self {
            state,
            focus_handle: cx.focus_handle(),
            terminals,
            tab_context_menu: None,
            tab_drop_target: None,
            dir_drop_target: None,
            dir_context_menu: None,
            renaming_tab_modal: None,
            renaming_dir_modal: None,
            git_branch: String::new(),
            git_branches: Vec::new(),
            theme_menu_open: false,
            settings_open: false,
            settings_section: SettingsSection::Appearance,
            branch_menu_open: false,
            alert_modal: None,
            selection: None,
            selecting: false,
            toast: None,
            update_info: None,
            update_checking: false,
            update_downloading: false,
            update_download_progress: None,
            update_staged: None,
            update_installing: false,
            update_error: None,
            ime_composition: String::new(),
            terminal_colors,
        };

        this.poll_git_branch(cx);
        this.check_for_updates(cx);
        this.schedule_update_check(cx);
        this
    }

    pub(crate) fn terminal_size(viewport: Size<Pixels>, font_size: f32) -> (u16, u16) {
        let cell_w = font_size * (8.4 / 14.0);
        let cell_h = font_size * (20.0 / 14.0);
        let cols = ((f32::from(viewport.width) - 192.0 - 32.0) / cell_w).max(10.0) as u16;
        let rows = ((f32::from(viewport.height) - 64.0 - 32.0) / cell_h).max(10.0) as u16;
        (rows, cols)
    }

    fn poll_git_branch(&mut self, cx: &mut Context<Self>) {
        let mut active_pid = None;
        let ws_idx = self.state.active_workspace;
        if let Some(ws) = self.state.workspaces.get(ws_idx) {
            let term_idx = ws.active_term;
            if let Some(term_entity) = self
                .terminals
                .get(ws_idx)
                .and_then(|terms| terms.get(term_idx))
            {
                active_pid = term_entity.read(cx).child_pid;
            }
        }

        let executor = cx.background_executor().clone();
        cx.spawn(
            move |workspace: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    let (branch, _cwd) = executor
                        .spawn(async move {
                            let cwd = active_pid.and_then(process_cwd);
                            let branch = if let Some(ref c) = cwd
                                && let Ok(output) = std::process::Command::new("git")
                                    .args(["branch", "--show-current"])
                                    .current_dir(c)
                                    .output()
                            {
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                stdout.trim().to_string()
                            } else {
                                String::new()
                            };
                            (branch, cwd)
                        })
                        .await;

                    workspace
                        .update(&mut async_cx, |this, cx| {
                            if this.git_branch != branch {
                                this.git_branch = branch;
                                cx.notify();
                            }
                        })
                        .ok();
                }
            },
        )
        .detach();

        cx.spawn(
            |this: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    async_cx
                        .background_executor()
                        .timer(std::time::Duration::from_secs(2))
                        .await;
                    this.update(&mut async_cx, |this, cx| {
                        this.poll_git_branch(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    pub fn toggle_theme_menu(&mut self, cx: &mut Context<Self>) {
        self.theme_menu_open = !self.theme_menu_open;
        cx.notify();
    }

    pub fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        self.theme_menu_open = false;
        self.branch_menu_open = false;
        cx.notify();
    }

    pub fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.update_checking || self.update_downloading || self.update_installing {
            return;
        }

        self.update_checking = true;
        self.update_error = None;
        cx.notify();

        let executor = cx.background_executor().clone();
        cx.spawn(
            move |workspace: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    let result = executor
                        .spawn(async { auto_update::check_for_update_detailed() })
                        .await;

                    workspace
                        .update(&mut async_cx, |this, cx| {
                            this.update_checking = false;
                            match result {
                                Ok(Some(info)) => {
                                    if this
                                        .update_info
                                        .as_ref()
                                        .is_some_and(|current| current.version != info.version)
                                    {
                                        if let Some(staged) = this.update_staged.take() {
                                            let _ = std::fs::remove_dir_all(staged);
                                        }
                                    }
                                    this.update_info = Some(info.clone());
                                    this.update_error = None;
                                }
                                Ok(None) => {
                                    this.update_error = None;
                                    if this.update_staged.is_none() {
                                        this.update_info = None;
                                    }
                                }
                                Err(error) => {
                                    this.update_error = Some(error);
                                }
                            }
                            cx.notify();
                        })
                        .ok();
                }
            },
        )
        .detach();
    }

    fn schedule_update_check(&mut self, cx: &mut Context<Self>) {
        cx.spawn(
            |this: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    async_cx
                        .background_executor()
                        .timer(std::time::Duration::from_secs(60 * 60))
                        .await;
                    this.update(&mut async_cx, |this, cx| {
                        this.check_for_updates(cx);
                        this.schedule_update_check(cx);
                    })
                    .ok();
                }
            },
        )
        .detach();
    }

    fn snapshot_session(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        for (ws_idx, ws) in self.state.workspaces.iter_mut().enumerate() {
            for (term_idx, term_data) in ws.terminals.iter_mut().enumerate() {
                let Some(term) = self
                    .terminals
                    .get(ws_idx)
                    .and_then(|terms| terms.get(term_idx))
                else {
                    continue;
                };
                let terminal = term.read(cx);
                if let Some(pid) = terminal.child_pid {
                    term_data.cwd = process_cwd(pid).or_else(|| term_data.cwd.clone());
                }
                if term_data.session_name != terminal.session_name {
                    term_data.session_name = terminal.session_name.clone();
                }
            }
        }
        self.state.save().map_err(|error| error.to_string())
    }

    pub fn download_update(&mut self, info: auto_update::UpdateInfo, cx: &mut Context<Self>) {
        if self.update_downloading || self.update_staged.is_some() || !info.can_auto_install {
            return;
        }

        let (progress_tx, progress_rx) = async_channel::unbounded();
        self.update_downloading = true;
        self.update_download_progress = Some(0.0);
        self.update_error = None;
        cx.notify();

        let executor = cx.background_executor().clone();
        cx.spawn(
            move |workspace: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    while let Ok(progress) = progress_rx.recv().await {
                        if workspace
                            .update(&mut async_cx, |this, cx| {
                                this.update_download_progress = Some(progress);
                                cx.notify();
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            },
        )
        .detach();

        cx.spawn(
            move |workspace: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    let result = executor
                        .spawn(async move { auto_update::download_update(&info, progress_tx) })
                        .await;

                    workspace
                        .update(&mut async_cx, |this, cx| {
                            this.update_downloading = false;
                            match result {
                                Ok(path) => {
                                    this.update_staged = Some(path);
                                    this.update_download_progress = Some(1.0);
                                }
                                Err(error) => {
                                    this.update_download_progress = None;
                                    this.update_error = Some(error);
                                }
                            }
                            cx.notify();
                        })
                        .ok();
                }
            },
        )
        .detach();
    }

    pub fn install_update(&mut self, cx: &mut Context<Self>) {
        if self.update_installing {
            return;
        }

        let Some(staged_app) = self.update_staged.clone() else {
            return;
        };

        if let Err(error) = self.snapshot_session(cx) {
            self.update_error = Some(format!("Could not save session before update: {error}"));
            cx.notify();
            return;
        }
        self.update_installing = true;
        cx.notify();

        let executor = cx.background_executor().clone();
        cx.spawn(
            move |workspace: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                let mut async_cx = cx.clone();
                async move {
                    let result = executor
                        .spawn(async move { auto_update::install_update(&staged_app) })
                        .await;

                    workspace
                        .update(&mut async_cx, |this, cx| {
                            this.update_installing = false;
                            if let Err(error) = result {
                                this.update_error = Some(error);
                            }
                            cx.notify();
                        })
                        .ok();
                }
            },
        )
        .detach();
    }

    pub fn select_settings_section(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.settings_section = section;
        cx.notify();
    }

    pub fn toggle_branch_menu(&mut self, cx: &mut Context<Self>) {
        self.branch_menu_open = !self.branch_menu_open;
        if self.branch_menu_open
            && let Some(cwd) = self.get_active_terminal_cwd(cx)
            && let Ok(output) = std::process::Command::new("git")
                .args(["branch", "--format=%(refname:short)"])
                .current_dir(cwd)
                .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.git_branches = stdout.lines().map(|s| s.to_string()).collect();
        }
        cx.notify();
    }

    pub fn checkout_branch(&mut self, branch: &str, cx: &mut Context<Self>) {
        if let Some(cwd) = self.get_active_terminal_cwd(cx) {
            if let Ok(output) = std::process::Command::new("git")
                .args(["checkout", branch])
                .current_dir(&cwd)
                .output()
                && !output.status.success()
            {
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.alert_modal = Some(("Git Checkout Failed".to_string(), stderr.to_string()));
            }

            if let Ok(output) = std::process::Command::new("git")
                .args(["branch", "--show-current"])
                .current_dir(&cwd)
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.git_branch = stdout.trim().to_string();
            }
        }
        self.branch_menu_open = false;
        cx.notify();
    }

    pub fn set_theme(
        &mut self,
        theme_fn: fn() -> theme::Theme,
        theme_name: String,
        cx: &mut Context<Self>,
    ) {
        self.state.theme = theme_fn();
        // Re-inject the palette: running terminals pick it up on their next
        // paint, and the next OSC probe from any CLI gets the new values.
        let (fg, bg, ansi) = theme_terminal_colors(&self.state.theme).get();
        self.terminal_colors.set(fg, bg, ansi);
        self.state.theme_name = Some(theme_name);
        self.theme_menu_open = false;
        self.state.save().ok();
        cx.notify();
    }

    pub fn add_dir(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = "Workspace".to_string();

        let cwd = self.get_active_terminal_cwd(cx);
        let (rows, cols) = Self::terminal_size(window.viewport_size(), self.state.font_size);
        let colors = self.terminal_colors.clone();
        let term =
            cx.new(|cx| PtyTerminal::new_with_cwd(cwd.clone(), None, rows, cols, colors, cx));
        let session_name = term.read(cx).session_name.clone();
        let new_ws = crate::state::WorkspaceData {
            name: format!("{} {}", name, self.state.workspaces.len() + 1),
            terminals: vec![crate::state::TerminalData {
                name,
                cwd: cwd.clone(),
                session_name,
            }],
            active_term: 0,
        };
        self.state.workspaces.push(new_ws);
        self.state.active_workspace = self.state.workspaces.len() - 1;

        cx.observe(&term, |_, _, cx| cx.notify()).detach();
        self.terminals.push(vec![term]);
        self.state.save().ok();
        cx.notify();
    }

    pub fn close_tab(&mut self, ws_idx: usize, tab_idx: usize, cx: &mut Context<Self>) {
        if let Some(term) = self
            .terminals
            .get(ws_idx)
            .and_then(|terms| terms.get(tab_idx))
        {
            term.update(cx, |term, _| term.shutdown());
        }
        if let Some(ws) = self.state.workspaces.get_mut(ws_idx) {
            if ws.terminals.len() > 1 {
                ws.terminals.remove(tab_idx);
                self.terminals[ws_idx].remove(tab_idx);
                if ws.active_term >= ws.terminals.len() {
                    ws.active_term = ws.terminals.len() - 1;
                }
            } else {
                if self.state.workspaces.len() > 1 {
                    self.state.workspaces.remove(ws_idx);
                    self.terminals.remove(ws_idx);
                    if self.state.active_workspace >= self.state.workspaces.len() {
                        self.state.active_workspace = self.state.workspaces.len() - 1;
                    }
                } else {
                    std::process::exit(0);
                }
            }
        }
        self.state.save().ok();
        cx.notify();
    }

    pub fn adjust_font_size(&mut self, delta: f32, cx: &mut Context<Self>) {
        let new_size = (self.state.font_size + delta).clamp(8.0, 40.0);
        if (new_size - self.state.font_size).abs() > f32::EPSILON {
            self.state.font_size = new_size;
            self.state.save().ok();
            cx.notify();
        }
    }

    fn active_screen_size(&self, cx: &App) -> (u16, u16) {
        let ws_idx = self.state.active_workspace;
        if let Some(ws) = self.state.workspaces.get(ws_idx)
            && let Some(term) = self
                .terminals
                .get(ws_idx)
                .and_then(|t| t.get(ws.active_term))
        {
            return term.read(cx).size();
        }
        (24, 80)
    }

    fn active_cursor(&self, cx: &App) -> Option<(u16, u16)> {
        let ws_idx = self.state.active_workspace;
        let ws = self.state.workspaces.get(ws_idx)?;
        let term = self.terminals.get(ws_idx)?.get(ws.active_term)?.read(cx);
        term.cursor_position()
    }

    pub fn cell_at(&self, pos: gpui::Point<gpui::Pixels>, cx: &App) -> (u16, u16) {
        let font_size = self.state.font_size;
        let cell_w = font_size * (8.4 / 14.0);
        let cell_h = font_size * (20.0 / 14.0);
        let (rows, cols) = self.active_screen_size(cx);
        let origin_x = 192.0 + 16.0;
        let origin_y = 64.0 + 16.0;
        let col = (((f32::from(pos.x) - origin_x) / cell_w).floor()).clamp(0.0, (cols as f32) - 1.0)
            as u16;
        let row = (((f32::from(pos.y) - origin_y) / cell_h).floor()).clamp(0.0, (rows as f32) - 1.0)
            as u16;
        (col, row)
    }

    pub fn selected_text(&self, cx: &App) -> Option<String> {
        let (start, end) = self.selection?;
        let ws_idx = self.state.active_workspace;
        let ws = self.state.workspaces.get(ws_idx)?;
        let term = self.terminals.get(ws_idx)?.get(ws.active_term)?.read(cx);
        Some(term.text_in_range(start, end))
    }

    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.selected_text(cx)
            && !text.trim().is_empty()
        {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            self.toast = Some("Copied!".to_string());
            cx.spawn(
                |this: gpui::WeakEntity<Workspace>, cx: &mut gpui::AsyncApp| {
                    let mut async_cx = cx.clone();
                    async move {
                        async_cx
                            .background_executor()
                            .timer(std::time::Duration::from_secs(2))
                            .await;
                        this.update(&mut async_cx, |this, cx| {
                            this.toast = None;
                            cx.notify();
                        })
                        .ok();
                    }
                },
            )
            .detach();
            cx.notify();
        }
    }

    pub fn write_active(&mut self, bytes: &[u8], cx: &mut App) {
        let ws_idx = self.state.active_workspace;
        if let Some(ws) = self.state.workspaces.get(ws_idx)
            && let Some(term) = self
                .terminals
                .get(ws_idx)
                .and_then(|t| t.get(ws.active_term))
        {
            term.update(cx, |term, _| term.write(bytes));
        }
    }

    pub fn paste_clipboard(&mut self, cx: &mut App) {
        if let Some(item) = cx.read_from_clipboard()
            && let Some(text) = item.text()
        {
            self.write_active(text.as_bytes(), cx);
        }
    }

    pub fn on_terminal_mouse_down(
        &mut self,
        event: &gpui::MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == gpui::MouseButton::Right {
            self.paste_clipboard(cx);
            return;
        }
        if event.button == gpui::MouseButton::Left {
            let cell = self.cell_at(event.position, cx);
            self.selecting = true;
            self.selection = Some((cell, cell));
            cx.notify();
        }
    }

    pub fn on_terminal_mouse_move(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selecting
            && let Some((start, _)) = self.selection
        {
            let cell = self.cell_at(event.position, cx);
            self.selection = Some((start, cell));
            cx.notify();
        }
    }

    pub fn on_terminal_mouse_up(
        &mut self,
        _event: &gpui::MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selecting = false;
        if let Some((start, end)) = self.selection {
            if start == end {
                self.selection = None;
            } else {
                self.copy_selection(cx);
            }
        }
        cx.notify();
    }

    pub fn on_terminal_scroll_wheel(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.modifiers.platform {
            let delta = f32::from(event.delta.pixel_delta(px(16.0)).y);
            self.adjust_font_size(delta * 0.1, cx);
            return;
        }

        let ws_idx = self.state.active_workspace;
        let font_size = self.state.font_size;
        let cell_h = font_size * (20.0 / 14.0);
        let delta_pixels = event.delta.pixel_delta(px(font_size)).y;
        // Multiply by 3 so a typical trackpad swipe scrolls several lines at a
        // time rather than accumulating many tiny sub-line deltas.
        let delta_lines = f32::from(delta_pixels) / cell_h * 3.0;

        if let Some(ws) = self.state.workspaces.get(ws_idx)
            && let Some(term_entity) = self
                .terminals
                .get(ws_idx)
                .and_then(|terms| terms.get(ws.active_term))
        {
            term_entity.update(cx, |term, _| {
                term.scroll(delta_lines);
            });
            cx.notify();
        }
    }

    pub fn handle_key_down(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings_open {
            if event.keystroke.key == "escape" {
                self.toggle_settings(cx);
            }
            return;
        }

        let ws_idx = self.state.active_workspace;

        // Open modals own every keystroke — including Cmd+A/C/X/V — before
        // any global terminal shortcut gets a chance to fire.
        if self.alert_modal.is_some() {
            let key = event.keystroke.key.as_str();
            if key == "enter" || key == "escape" {
                self.alert_modal = None;
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        if self.renaming_tab_modal.is_some() || self.renaming_dir_modal.is_some() {
            let key = event.keystroke.key.as_str().to_string();
            let modifiers = event.keystroke.modifiers;

            match key.as_str() {
                "enter" => {
                    if let Some((idx, field)) = self.renaming_tab_modal.take() {
                        if let Some(ws) = self.state.workspaces.get_mut(ws_idx) {
                            ws.terminals[idx].name = field.value().to_string();
                            self.state.save().ok();
                        }
                    } else if let Some((idx, field)) = self.renaming_dir_modal.take() {
                        if let Some(ws) = self.state.workspaces.get_mut(idx) {
                            ws.name = field.value().to_string();
                            self.state.save().ok();
                        }
                    }
                }
                "escape" => {
                    self.renaming_tab_modal = None;
                    self.renaming_dir_modal = None;
                }
                _ => {
                    if let Some((_, field)) = self
                        .renaming_tab_modal
                        .as_mut()
                        .or(self.renaming_dir_modal.as_mut())
                    {
                        field.key(&key, &modifiers, cx);
                    }
                }
            }
            cx.notify();
            cx.stop_propagation();
            return;
        }

        if event.keystroke.modifiers.platform {
            match event.keystroke.key.as_str() {
                "=" | "+" => {
                    self.adjust_font_size(1.0, cx);
                    return;
                }
                "-" => {
                    self.adjust_font_size(-1.0, cx);
                    return;
                }
                "0" => {
                    self.state.font_size = crate::state::DEFAULT_FONT_SIZE;
                    self.state.save().ok();
                    cx.notify();
                    return;
                }
                "c" => {
                    self.copy_selection(cx);
                    cx.stop_propagation();
                    return;
                }
                "a" => {
                    // Select the entire visible screen, like Cmd+A in other
                    // terminals; Cmd+C then copies it via selected_text().
                    let (rows, cols) = self.active_screen_size(cx);
                    if rows > 0 && cols > 0 {
                        self.selection = Some(((0, 0), (cols - 1, rows - 1)));
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "v" => {
                    self.paste_clipboard(cx);
                    cx.stop_propagation();
                    return;
                }
                "k" => {
                    self.write_active(b"\x0C", cx);
                    cx.stop_propagation();
                    return;
                }
                "t" => {
                    self.add_term(window, cx);
                    cx.stop_propagation();
                    return;
                }
                "n" => {
                    self.add_dir(window, cx);
                    cx.stop_propagation();
                    return;
                }
                "w" => {
                    if let Some(ws) = self.state.workspaces.get(ws_idx) {
                        self.delete_term(ws.active_term, cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                _ => {}
            }
        }

        if let Some(ws) = self.state.workspaces.get(ws_idx) {
            let term_idx = ws.active_term;
            if let Some(term_entity) = self
                .terminals
                .get(ws_idx)
                .and_then(|terms| terms.get(term_idx))
            {
                let key = event.keystroke.key.as_str();
                let modifiers = &event.keystroke.modifiers;

                let bytes = match key {
                    "enter" => vec![b'\r'],
                    "backspace" => vec![0x7f],
                    "tab" => vec![b'\t'],
                    "escape" => vec![0x1b],
                    "up" => vec![0x1b, b'[', b'A'],
                    "down" => vec![0x1b, b'[', b'B'],
                    "right" => vec![0x1b, b'[', b'C'],
                    "left" => vec![0x1b, b'[', b'D'],
                    "space" => vec![b' '],
                    _ if modifiers.control && key.chars().count() == 1 => {
                        let c = key.chars().next().unwrap();
                        if c.is_ascii_lowercase() {
                            vec![(c as u8) - b'a' + 1]
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                if !bytes.is_empty() {
                    term_entity.update(cx, |term, _| {
                        term.write(&bytes);
                    });
                    // Prevent GPUI's tab focus-traversal (and other default
                    // key handling) from swallowing keys like Tab/arrows so
                    // they reach the shell.
                    cx.stop_propagation();
                }
            }
        }
    }

    pub fn move_dir(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from != to && from < self.state.workspaces.len() && to <= self.state.workspaces.len() {
            let ws = self.state.workspaces.remove(from);
            let term = self.terminals.remove(from);
            let new_to = if from < to { to - 1 } else { to };
            self.state.workspaces.insert(new_to, ws);
            self.terminals.insert(new_to, term);
            if self.state.active_workspace == from {
                self.state.active_workspace = new_to;
            }
            self.state.save().ok();
            cx.notify();
        }
    }

    pub fn open_dir_menu(
        &mut self,
        idx: usize,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.dir_context_menu = Some((idx, pos));
        cx.notify();
    }

    pub fn close_dir_menu(&mut self, cx: &mut Context<Self>) {
        self.dir_context_menu = None;
        cx.notify();
    }

    pub fn select_dir(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.state.workspaces.len() {
            self.state.active_workspace = idx;
            self.state.save().ok();
            cx.notify();
        }
    }

    pub fn delete_dir(&mut self, idx: usize, cx: &mut Context<Self>) {
        if self.state.workspaces.len() > 1 {
            if let Some(terms) = self.terminals.get(idx) {
                for term in terms {
                    term.update(cx, |term, _| term.shutdown());
                }
            }
            self.state.workspaces.remove(idx);
            self.terminals.remove(idx);
            if self.state.active_workspace >= self.state.workspaces.len() {
                self.state.active_workspace = self.state.workspaces.len() - 1;
            }
        } else {
            std::process::exit(0);
        }
        self.state.save().ok();
        cx.notify();
    }

    pub fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let ws_idx = self.state.active_workspace;
        if let Some(ws) = self.state.workspaces.get_mut(ws_idx)
            && from != to
            && from < ws.terminals.len()
            && to <= ws.terminals.len()
        {
            let term_data = ws.terminals.remove(from);
            let term = self.terminals[ws_idx].remove(from);
            let new_to = if from < to { to - 1 } else { to };
            ws.terminals.insert(new_to, term_data);
            self.terminals[ws_idx].insert(new_to, term);
            if ws.active_term == from {
                ws.active_term = new_to;
            }
            self.state.save().ok();
            cx.notify();
        }
    }

    pub fn open_tab_menu(
        &mut self,
        idx: usize,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.tab_context_menu = Some((idx, pos));
        cx.notify();
    }

    pub fn close_tab_menu(&mut self, cx: &mut Context<Self>) {
        self.tab_context_menu = None;
        cx.notify();
    }

    pub fn select_term(&mut self, idx: usize, cx: &mut Context<Self>) {
        let ws_idx = self.state.active_workspace;
        if let Some(ws) = self.state.workspaces.get_mut(ws_idx)
            && idx < ws.terminals.len()
        {
            ws.active_term = idx;
            self.state.save().ok();
            cx.notify();
        }
    }

    pub fn delete_term(&mut self, idx: usize, cx: &mut Context<Self>) {
        self.close_tab(self.state.active_workspace, idx, cx);
    }

    pub fn add_term(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ws_idx = self.state.active_workspace;
        let cwd = self.get_active_terminal_cwd(cx);
        let name = "Terminal".to_string();

        let (rows, cols) = Self::terminal_size(window.viewport_size(), self.state.font_size);
        let colors = self.terminal_colors.clone();
        let term =
            cx.new(|cx| PtyTerminal::new_with_cwd(cwd.clone(), None, rows, cols, colors, cx));
        cx.observe(&term, |_, _, cx| cx.notify()).detach();
        let session_name = term.read(cx).session_name.clone();

        if let Some(ws) = self.state.workspaces.get_mut(ws_idx) {
            ws.terminals.push(crate::state::TerminalData {
                name,
                cwd,
                session_name,
            });
            ws.active_term = ws.terminals.len() - 1;
            self.terminals[ws_idx].push(term);
            self.state.save().ok();
            cx.notify();
        }
    }

    pub fn start_renaming_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        let ws_idx = self.state.active_workspace;
        if let Some(ws) = self.state.workspaces.get(ws_idx)
            && let Some(term) = ws.terminals.get(idx)
        {
            self.renaming_tab_modal = Some((idx, TextField::new(term.name.clone())));
            cx.notify();
        }
    }

    pub fn start_renaming_dir(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(ws) = self.state.workspaces.get(idx) {
            self.renaming_dir_modal = Some((idx, TextField::new(ws.name.clone())));
            cx.notify();
        }
    }
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn utf16_to_byte_offset(text: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }

    let mut utf16_offset = 0;
    for (byte_offset, character) in text.char_indices() {
        utf16_offset += character.len_utf16();
        if offset <= utf16_offset {
            return byte_offset + character.len_utf8();
        }
    }
    text.len()
}

impl EntityInputHandler for Workspace {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.ime_composition.is_empty() {
            return None;
        }
        let length = utf16_len(&self.ime_composition);
        let start = range.start.min(length);
        let end = range.end.min(length);
        *adjusted_range = Some(start..end);
        Some(
            self.ime_composition[utf16_to_byte_offset(&self.ime_composition, start)
                ..utf16_to_byte_offset(&self.ime_composition, end)]
                .to_string(),
        )
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let caret = utf16_len(&self.ime_composition);
        Some(UTF16Selection {
            range: caret..caret,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        (!self.ime_composition.is_empty()).then(|| 0..utf16_len(&self.ime_composition))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.ime_composition.clear();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ime_composition.clear();
        if !text.is_empty() {
            self.write_active(text.as_bytes(), cx);
        }
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ime_composition.clear();
        self.ime_composition.push_str(new_text);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (row, col) = self.active_cursor(cx)?;
        let font_size = self.state.font_size;
        let cell_w = font_size * (8.4 / 14.0);
        let cell_h = font_size * (20.0 / 14.0);
        Some(Bounds::new(
            element_bounds.origin
                + point(
                    px(16.0 + col as f32 * cell_w),
                    px(16.0 + row as f32 * cell_h),
                ),
            size(px(cell_w), px(cell_h)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod input_tests {
    use super::utf16_to_byte_offset;

    #[test]
    fn utf16_offsets_handle_multibyte_and_surrogate_characters() {
        let text = "Aส🙂";
        assert_eq!(utf16_to_byte_offset(text, 0), 0);
        assert_eq!(utf16_to_byte_offset(text, 1), 1);
        assert_eq!(utf16_to_byte_offset(text, 2), 4);
        assert_eq!(utf16_to_byte_offset(text, 4), 8);
        assert_eq!(utf16_to_byte_offset(text, 9), 8);
    }
}
impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.state.theme;

        let mut root = div()
            .id("workspace")
            .track_focus(&self.focus_handle)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    let mut changed = false;
                    if this.tab_context_menu.is_some() {
                        this.tab_context_menu = None;
                        changed = true;
                    }
                    if this.dir_context_menu.is_some() {
                        this.dir_context_menu = None;
                        changed = true;
                    }
                    if this.theme_menu_open {
                        this.theme_menu_open = false;
                        changed = true;
                    }
                    if this.branch_menu_open {
                        this.branch_menu_open = false;
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _event, _window, cx| {
                    let mut changed = false;
                    if this.tab_context_menu.is_some() {
                        this.tab_context_menu = None;
                        changed = true;
                    }
                    if this.dir_context_menu.is_some() {
                        this.dir_context_menu = None;
                        changed = true;
                    }
                    if this.theme_menu_open {
                        this.theme_menu_open = false;
                        changed = true;
                    }
                    if this.branch_menu_open {
                        this.branch_menu_open = false;
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(Self::handle_key_down))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg_main)
            .text_color(theme.text_primary)
            .text_size(px(13.0))
            .child(render_title_bar(self, cx));

        root = root.child(
            div()
                .flex_1()
                .w_full()
                .flex()
                .flex_row()
                .min_h_0()
                .overflow_hidden()
                .child(render_sidebar(self, cx))
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .overflow_hidden()
                        .child(render_tab_bar(self, cx))
                        .child(render_terminal_view(
                            self,
                            window,
                            cx,
                            window.viewport_size(),
                        )),
                ),
        );

        if self.settings_open {
            root = root.child(render_settings(self, cx));
        }

        if let Some((idx, pos)) = self.tab_context_menu {
            root = root.child(
                div()
                    .absolute()
                    .left(pos.x)
                    .top(pos.y)
                    .w(px(150.0))
                    .bg(theme.bg_sidebar)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_1()
                    .flex()
                    .flex_col()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .hover(|s| s.bg(theme.bg_tab_inactive))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _e, _w, cx| {
                                    this.start_renaming_tab(idx, cx);
                                    this.close_tab_menu(cx);
                                }),
                            )
                            .child("Rename Tab"),
                    )
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .text_color(theme.ansi[1])
                            .hover(|s| s.bg(theme.bg_tab_inactive))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _e, _w, cx| {
                                    this.delete_term(idx, cx);
                                    this.close_tab_menu(cx);
                                }),
                            )
                            .child("Close Tab"),
                    ),
            );
        }

        if let Some((idx, field)) = self.renaming_tab_modal.clone() {
            root = root.child(modal_overlay(
                theme,
                "Rename Tab",
                "Enter a custom name for this tab.",
                field.render(theme),
                div()
                    .flex()
                    .w_full()
                    .justify_center()
                    .gap_3()
                    .child(button("Cancel", theme, false).w_full().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _e, _w, cx| {
                            this.renaming_tab_modal = None;
                            cx.notify();
                        }),
                    ))
                    .child(button("Rename", theme, true).w_full().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _e, _w, cx| {
                            let ws_idx = this.state.active_workspace;
                            if let Some(ws) = this.state.workspaces.get_mut(ws_idx) {
                                ws.terminals[idx].name = field.value().to_string();
                                this.state.save().ok();
                            }
                            this.renaming_tab_modal = None;
                            cx.notify();
                        }),
                    )),
            ));
        }

        if let Some((idx, pos)) = self.dir_context_menu {
            root = root.child(
                div()
                    .absolute()
                    .left(pos.x)
                    .top(pos.y)
                    .w(px(150.0))
                    .bg(theme.bg_sidebar)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .p_1()
                    .flex()
                    .flex_col()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .hover(|s| s.bg(theme.bg_tab_inactive))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _e, _w, cx| {
                                    this.start_renaming_dir(idx, cx);
                                    this.close_dir_menu(cx);
                                }),
                            )
                            .child("Rename Directory"),
                    )
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .hover(|s| s.bg(theme.bg_tab_inactive))
                            .cursor_pointer()
                            .text_color(theme.ansi[1])
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _e, _w, cx| {
                                    this.delete_dir(idx, cx);
                                    this.close_dir_menu(cx);
                                }),
                            )
                            .child("Delete Directory"),
                    ),
            );
        }

        if self.theme_menu_open {
            let mut list = div()
                .id("theme-dropdown")
                .absolute()
                .top(px(28.0))
                .right(px(16.0))
                .w(px(160.0))
                .max_h(px(300.0))
                .overflow_y_scroll()
                .bg(theme.bg_sidebar)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .shadow_lg()
                .p_1()
                .flex()
                .flex_col()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation());

            for (name, func) in theme::Theme::builtins() {
                let n = name.to_string();
                let is_active = self.state.theme_name.as_deref() == Some(name);
                let display_name = theme::Theme::display_name(name);

                list = list.child(
                    div()
                        .id(name)
                        .p_2()
                        .rounded_sm()
                        .hover(|s| s.bg(theme.bg_tab_inactive))
                        .cursor_pointer()
                        .text_color(if is_active {
                            theme.accent
                        } else {
                            theme.text_primary
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _e, _w, cx| {
                                this.set_theme(func, n.clone(), cx);
                            }),
                        )
                        .child(display_name),
                );
            }
            root = root.child(list);
        }

        if let Some((idx, field)) = self.renaming_dir_modal.clone() {
            root = root.child(modal_overlay(
                theme,
                "Rename Workspace",
                "Enter a custom name for this workspace.",
                field.render(theme),
                div()
                    .flex()
                    .w_full()
                    .justify_center()
                    .gap_3()
                    .child(button("Cancel", theme, false).w_full().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _e, _w, cx| {
                            this.renaming_dir_modal = None;
                            cx.notify();
                        }),
                    ))
                    .child(button("Rename", theme, true).w_full().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _e, _w, cx| {
                            if let Some(ws) = this.state.workspaces.get_mut(idx) {
                                ws.name = field.value().to_string();
                                this.state.save().ok();
                            }
                            this.renaming_dir_modal = None;
                            cx.notify();
                        }),
                    )),
            ));
        }

        if let Some((title, msg)) = self.alert_modal.clone() {
            root = root.child(modal_overlay(
                theme,
                title.clone(),
                "",
                div()
                    .w_full()
                    .p_2()
                    .bg(theme.bg_sidebar)
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .text_color(theme.text_primary)
                    .text_size(px(12.0))
                    .child(msg),
                div().flex().w_full().justify_end().child(
                    button("OK", theme, true).w(px(80.0)).on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _e, _w, cx| {
                            this.alert_modal = None;
                            cx.notify();
                        }),
                    ),
                ),
            ));
        }

        if self.branch_menu_open && !self.git_branches.is_empty() {
            let mut list = div()
                .id("branch-dropdown")
                .absolute()
                .top(px(28.0))
                .left(px(80.0)) // Roughly aligned with branch text
                .w(px(500.0))
                .max_h(px(300.0))
                .overflow_y_scroll()
                .bg(theme.bg_sidebar)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .shadow_lg()
                .p_1()
                .flex()
                .flex_col()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .p_2()
                        .text_color(theme.text_muted)
                        .text_size(px(11.0))
                        .child("Local Branches"),
                );

            for branch in &self.git_branches {
                let b = branch.clone();
                let is_active = self.git_branch == *branch;

                list = list.child(
                    div()
                        .id(gpui::SharedString::from(branch.clone()))
                        .p_2()
                        .rounded_sm()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .hover(|s| s.bg(theme.bg_tab_inactive))
                        .cursor_pointer()
                        .text_color(if is_active {
                            theme.accent
                        } else {
                            theme.text_primary
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _e, _w, cx| {
                                this.checkout_branch(&b, cx);
                            }),
                        )
                        .child(
                            div()
                                .w(px(14.0))
                                .h(px(14.0))
                                .child(
                                    gpui::svg()
                                        .path("icons/git_branch.svg")
                                        .text_color(if is_active {
                                            theme.accent
                                        } else {
                                            theme.text_muted
                                        })
                                        .size(px(14.0)),
                                ),
                        )
                        .child(div().flex_1().child(branch.clone()))
                        .child(if is_active {
                            div().w(px(12.0)).child("✓")
                        } else {
                            div().w(px(12.0))
                        }),
                );
            }
            let backdrop = div()
                .id("branch-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(gpui::rgba(0x00000022))
                .cursor_default()
                .on_mouse_move(cx.listener(|_, _, _, _| {}))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _e, _w, cx| {
                        this.branch_menu_open = false;
                        cx.notify();
                    }),
                )
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation());

            root = root.child(backdrop).child(list);
        }

        if let Some(msg) = &self.toast {
            root = root.child(
                div()
                    .absolute()
                    .top(px(48.0))
                    .right(px(16.0))
                    .bg(theme.bg_sidebar)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_md()
                    .shadow_lg()
                    .px_4()
                    .py_2()
                    .text_color(theme.text_primary)
                    .child(msg.clone()),
            );
        }

        root
    }
}
