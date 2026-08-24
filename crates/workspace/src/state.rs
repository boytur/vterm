use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use theme::Theme;

#[derive(Serialize, Deserialize, Clone)]
pub struct TerminalData {
    pub name: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkspaceData {
    pub name: String,
    pub terminals: Vec<TerminalData>,
    pub active_term: usize,
}

pub const DEFAULT_FONT_SIZE: f32 = 14.0;

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub workspaces: Vec<WorkspaceData>,
    pub active_workspace: usize,
    #[serde(default)]
    pub theme_name: Option<String>,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(skip)]
    pub theme: Theme,
}

fn default_font_size() -> f32 {
    DEFAULT_FONT_SIZE
}

impl AppState {
    fn default_dir_name() -> String {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Workspace".into())
    }

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|_| {
            let name = Self::default_dir_name();
            Self {
                workspaces: vec![WorkspaceData {
                    name: name.clone(),
                    terminals: vec![TerminalData {
                        name: name.clone(),
                        cwd: None,
                        session_name: None,
                    }],
                    active_term: 0,
                }],
                active_workspace: 0,
                theme_name: Some("zed_dark".to_string()),
                font_size: DEFAULT_FONT_SIZE,
                theme: Theme::default(),
            }
        })
    }

    pub fn save_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("vterm");
        fs::create_dir_all(&path).ok();
        path.push("state.json");
        path
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        let path = Self::save_path();
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, json)?;
        fs::rename(temp, path)
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(Self::save_path())?;
        let mut state: AppState = serde_json::from_str(&json)?;

        state.theme = Theme::from_name(state.theme_name.as_deref());

        if state.workspaces.is_empty() {
            let name = Self::default_dir_name();
            state.workspaces.push(WorkspaceData {
                name: name.clone(),
                terminals: vec![TerminalData {
                    name: name.clone(),
                    cwd: None,
                    session_name: None,
                }],
                active_term: 0,
            });
        }
        for workspace in &mut state.workspaces {
            if workspace.terminals.is_empty() {
                workspace.terminals.push(TerminalData {
                    name: workspace.name.clone(),
                    cwd: None,
                    session_name: None,
                });
            }
            workspace.active_term = workspace
                .active_term
                .min(workspace.terminals.len().saturating_sub(1));
        }
        state.active_workspace = state
            .active_workspace
            .min(state.workspaces.len().saturating_sub(1));
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_app_state() {
        let state = AppState {
            workspaces: vec![WorkspaceData {
                name: "Workspace".into(),
                terminals: vec![TerminalData {
                    name: "Terminal".into(),
                    cwd: None,
                    session_name: None,
                }],
                active_term: 0,
            }],
            active_workspace: 0,
            theme: theme::Theme::ubuntu(),
            theme_name: Some("ubuntu".to_string()),
            font_size: crate::state::DEFAULT_FONT_SIZE,
        };
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].name, "Workspace");
        assert_eq!(state.workspaces[0].terminals.len(), 1);
        assert_eq!(state.workspaces[0].terminals[0].name, "Terminal");
    }
}
