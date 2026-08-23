use crate::theme::Theme;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct TerminalData {
    pub name: String,
    #[serde(default)]
    pub cwd: Option<String>,
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
                    terminals: vec![TerminalData { name: name.clone(), cwd: None }],
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
        fs::write(Self::save_path(), json)
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(Self::save_path())?;
        let mut state: AppState = serde_json::from_str(&json)?;
        
        state.theme = match state.theme_name.as_deref() {
            Some("ubuntu") => Theme::ubuntu(),
            Some("dracula") => Theme::dracula(),
            Some("nord") => Theme::nord(),
            Some("gruvbox_dark") => Theme::gruvbox_dark(),
            Some("one_dark") => Theme::one_dark(),
            Some("solarized_dark") => Theme::solarized_dark(),
            Some("catppuccin_mocha") => Theme::catppuccin_mocha(),
            Some("tokyo_night") => Theme::tokyo_night(),
            Some("monokai") => Theme::monokai(),
            Some("ayu_dark") => Theme::ayu_dark(),
            Some("github_dark") => Theme::github_dark(),
            _ => Theme::zed_dark(),
        };

        if state.workspaces.is_empty() {
            let name = Self::default_dir_name();
            state.workspaces.push(WorkspaceData {
                name: name.clone(),
                terminals: vec![TerminalData { name: name.clone(), cwd: None }],
                active_term: 0,
            });
        }
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
                }],
                active_term: 0,
            }],
            active_workspace: 0,
            theme: crate::theme::Theme::ubuntu(),
            theme_name: Some("ubuntu".to_string()),
            font_size: crate::state::DEFAULT_FONT_SIZE,
        };
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].name, "Workspace");
        assert_eq!(state.workspaces[0].terminals.len(), 1);
        assert_eq!(state.workspaces[0].terminals[0].name, "Terminal");
    }
}
