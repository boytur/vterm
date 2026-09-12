use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use theme::Theme;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TerminalData {
    pub name: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub session_name: Option<String>,
}

/// Split direction for a pane container.
/// - `Vertical` lays panes out left-to-right (a vertical divider).
/// - `Horizontal` stacks panes top-to-bottom (a horizontal divider).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SplitDir {
    Vertical,
    Horizontal,
}

/// Binary split tree over a tab's panes. Leaves hold indices into
/// [`TabData::panes`]; internal nodes split the available rect by `ratio`.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum SplitNode {
    Leaf(usize),
    Split {
        dir: SplitDir,
        #[serde(default = "default_split_ratio")]
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

fn default_split_ratio() -> f32 {
    0.5
}

/// One tab: a set of panes plus the split tree that lays them out.
/// Stored in the `terminals` field of [`WorkspaceData`] (kept field name so
/// pre-split `state.json` files keep loading — see custom `Deserialize`).
#[derive(Serialize, Clone, Debug)]
pub struct TabData {
    pub name: String,
    #[serde(default)]
    pub panes: Vec<TerminalData>,
    #[serde(default)]
    pub active_pane: usize,
    #[serde(default)]
    pub root: SplitNode,
}

impl Default for SplitNode {
    fn default() -> Self {
        SplitNode::Leaf(0)
    }
}

impl<'de> Deserialize<'de> for TabData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // New format has a `panes` key; the pre-split format is a bare
        // `TerminalData` map ({name, cwd?, session_name?}) — one tab, one pane.
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.get("panes").is_some() {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Terminal")
                .to_string();
            let panes: Vec<TerminalData> = value
                .get("panes")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?
                .unwrap_or_default();
            let active_pane = value
                .get("active_pane")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let root: SplitNode = value
                .get("root")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()
                .map_err(serde::de::Error::custom)?
                .unwrap_or(SplitNode::Leaf(0));
            let mut tab = TabData {
                name,
                panes,
                active_pane,
                root,
            };
            tab.normalize();
            Ok(tab)
        } else {
            let pane: TerminalData =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            Ok(TabData::single(pane))
        }
    }
}

impl TabData {
    pub fn single(pane: TerminalData) -> Self {
        let name = pane.name.clone();
        Self {
            name,
            panes: vec![pane],
            active_pane: 0,
            root: SplitNode::Leaf(0),
        }
    }

    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            self.panes
                .first()
                .map(|p| p.name.as_str())
                .unwrap_or("Terminal")
        } else {
            &self.name
        }
    }

    /// Leaves in left-to-right, top-to-bottom order.
    pub fn leaves(&self) -> Vec<usize> {
        let mut out = Vec::new();
        fn walk(node: &SplitNode, out: &mut Vec<usize>) {
            match node {
                SplitNode::Leaf(i) => out.push(*i),
                SplitNode::Split { first, second, .. } => {
                    walk(first, out);
                    walk(second, out);
                }
            }
        }
        walk(&self.root, &mut out);
        out
    }

    /// Clamps indices, drops dangling leaves, rebuilds a sane tree.
    pub fn normalize(&mut self) {
        if self.panes.is_empty() {
            self.panes.push(TerminalData {
                name: if self.name.is_empty() {
                    "Terminal".to_string()
                } else {
                    self.name.clone()
                },
                cwd: None,
                session_name: None,
            });
        }
        // Keep only leaves pointing at live panes, preserving order.
        let mut seen = vec![false; self.panes.len()];
        let mut ordered = Vec::new();
        for leaf in self.leaves() {
            if leaf < self.panes.len() && !seen[leaf] {
                seen[leaf] = true;
                ordered.push(leaf);
            }
        }
        for (i, used) in seen.iter().enumerate() {
            if !used {
                ordered.push(i);
            }
        }
        self.root = build_balanced(&ordered, 0);
        self.active_pane = self.active_pane.min(self.panes.len().saturating_sub(1));
        if !self.leaves().contains(&self.active_pane) {
            self.active_pane = self.leaves()[0];
        }
    }

    /// Splits `pane_idx`, appending the new pane and returning its index.
    pub fn split_pane(&mut self, pane_idx: usize, dir: SplitDir) -> usize {
        let pane_idx = pane_idx.min(self.panes.len().saturating_sub(1));
        let base = self.panes[pane_idx].clone();
        let new_idx = self.panes.len();
        self.panes.push(TerminalData {
            name: format!("{} {}", base.name, new_idx + 1),
            cwd: base.cwd.clone(),
            session_name: None,
        });
        self.root = replace_leaf(
            std::mem::replace(&mut self.root, SplitNode::Leaf(0)),
            pane_idx,
            SplitNode::Split {
                dir,
                ratio: 0.5,
                first: Box::new(SplitNode::Leaf(pane_idx)),
                second: Box::new(SplitNode::Leaf(new_idx)),
            },
        );
        self.active_pane = new_idx;
        new_idx
    }

    /// Closes `pane_idx`. Returns false when it was the last pane.
    pub fn close_pane(&mut self, pane_idx: usize) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        let pane_idx = pane_idx.min(self.panes.len() - 1);
        self.panes.remove(pane_idx);
        let renumber = |i: usize| if i > pane_idx { i - 1 } else { i };
        self.root = remove_leaf(
            std::mem::replace(&mut self.root, SplitNode::Leaf(0)),
            pane_idx,
        )
        .map(|root| renumber_tree(root, &renumber))
        .unwrap_or(SplitNode::Leaf(0));
        self.normalize();
        if self.active_pane >= self.panes.len() {
            self.active_pane = self.panes.len() - 1;
        }
        true
    }

    /// Lays the split tree out over a `avail_w × avail_h` box, returning one
    /// `(pane_idx, x, y, w, h)` rect per leaf in render order. Pure geometry
    /// shared by the renderer and mouse→cell mapping so they never disagree.
    pub fn allocations(&self, avail_w: f32, avail_h: f32) -> Vec<(usize, f32, f32, f32, f32)> {
        let mut out = Vec::new();
        alloc_node(&self.root, 0.0, 0.0, avail_w, avail_h, &mut out);
        out
    }
}

fn build_balanced(order: &[usize], depth: usize) -> SplitNode {
    match order {
        [] => SplitNode::Leaf(0),
        [single] => SplitNode::Leaf(*single),
        _ => {
            let mid = order.len() / 2;
            let dir = if depth.is_multiple_of(2) {
                SplitDir::Vertical
            } else {
                SplitDir::Horizontal
            };
            SplitNode::Split {
                dir,
                ratio: 0.5,
                first: Box::new(build_balanced(&order[..mid], depth + 1)),
                second: Box::new(build_balanced(&order[mid..], depth + 1)),
            }
        }
    }
}

fn replace_leaf(node: SplitNode, target: usize, with: SplitNode) -> SplitNode {
    match node {
        SplitNode::Leaf(i) if i == target => with,
        SplitNode::Leaf(i) => SplitNode::Leaf(i),
        SplitNode::Split {
            dir,
            ratio,
            first,
            second,
        } => SplitNode::Split {
            dir,
            ratio: ratio.clamp(0.15, 0.85),
            first: Box::new(replace_leaf(*first, target, with.clone())),
            second: Box::new(replace_leaf(*second, target, with)),
        },
    }
}

/// Removes the leaf holding `target`, promoting its sibling. Returns `None`
/// when the tree was a single leaf.
fn remove_leaf(node: SplitNode, target: usize) -> Option<SplitNode> {
    match node {
        SplitNode::Leaf(i) if i == target => None,
        SplitNode::Leaf(i) => Some(SplitNode::Leaf(i)),
        SplitNode::Split {
            dir,
            ratio,
            first,
            second,
        } => {
            let first_has = contains_leaf(&first, target);
            let second_has = contains_leaf(&second, target);
            match (first_has, second_has) {
                (true, true) => {
                    // Shouldn't happen (leaf twice), drop from the left side.
                    match remove_leaf(*first, target) {
                        Some(rest) => Some(SplitNode::Split {
                            dir,
                            ratio,
                            first: Box::new(rest),
                            second,
                        }),
                        None => Some(*second),
                    }
                }
                (true, false) => match remove_leaf(*first, target) {
                    Some(rest) => Some(SplitNode::Split {
                        dir,
                        ratio,
                        first: Box::new(rest),
                        second,
                    }),
                    None => Some(*second),
                },
                (false, true) => match remove_leaf(*second, target) {
                    Some(rest) => Some(SplitNode::Split {
                        dir,
                        ratio,
                        first,
                        second: Box::new(rest),
                    }),
                    None => Some(*first),
                },
                (false, false) => Some(SplitNode::Split {
                    dir,
                    ratio,
                    first,
                    second,
                }),
            }
        }
    }
}

fn contains_leaf(node: &SplitNode, target: usize) -> bool {
    match node {
        SplitNode::Leaf(i) => *i == target,
        SplitNode::Split { first, second, .. } => {
            contains_leaf(first, target) || contains_leaf(second, target)
        }
    }
}

fn renumber_tree(node: SplitNode, renumber: &dyn Fn(usize) -> usize) -> SplitNode {
    match node {
        SplitNode::Leaf(i) => SplitNode::Leaf(renumber(i)),
        SplitNode::Split {
            dir,
            ratio,
            first,
            second,
        } => SplitNode::Split {
            dir,
            ratio,
            first: Box::new(renumber_tree(*first, renumber)),
            second: Box::new(renumber_tree(*second, renumber)),
        },
    }
}

fn alloc_node(
    node: &SplitNode,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    out: &mut Vec<(usize, f32, f32, f32, f32)>,
) {
    match node {
        SplitNode::Leaf(i) => out.push((*i, x, y, w, h)),
        SplitNode::Split {
            dir,
            ratio,
            first,
            second,
        } => {
            let r = ratio.clamp(0.15, 0.85);
            match dir {
                SplitDir::Vertical => {
                    alloc_node(first, x, y, w * r, h, out);
                    alloc_node(second, x + w * r, y, w * (1.0 - r), h, out);
                }
                SplitDir::Horizontal => {
                    alloc_node(first, x, y, w, h * r, out);
                    alloc_node(second, x, y + h * r, w, h * (1.0 - r), out);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkspaceData {
    pub name: String,
    /// Tabs (kept serialized name for pre-split `state.json` compat: each
    /// element used to be a bare `TerminalData`, now a `TabData`).
    pub terminals: Vec<TabData>,
    pub active_term: usize,
}

pub const DEFAULT_FONT_SIZE: f32 = 16.0;

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

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    // Keep naming independent from the process CWD (dev.sh, Finder, and a
    // release app can all start in different directories).
    fn default_dir_name() -> String {
        "Workspace".into()
    }

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|_| {
            let name = Self::default_dir_name();
            Self {
                workspaces: vec![WorkspaceData {
                    name: name.clone(),
                    terminals: vec![TabData::single(TerminalData {
                        name: name.clone(),
                        cwd: None,
                        session_name: None,
                    })],
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
        path.push(std::env::var("VTERM_CONFIG_NAME").unwrap_or_else(|_| "vterm".to_string()));
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
                terminals: vec![TabData::single(TerminalData {
                    name: name.clone(),
                    cwd: None,
                    session_name: None,
                })],
                active_term: 0,
            });
        }
        for workspace in &mut state.workspaces {
            if workspace.terminals.is_empty() {
                workspace.terminals.push(TabData::single(TerminalData {
                    name: workspace.name.clone(),
                    cwd: None,
                    session_name: None,
                }));
            }
            for tab in &mut workspace.terminals {
                tab.normalize();
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
                terminals: vec![TabData::single(TerminalData {
                    name: "Terminal".into(),
                    cwd: None,
                    session_name: None,
                })],
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
        assert_eq!(state.workspaces[0].terminals[0].panes[0].name, "Terminal");
    }

    fn pane(name: &str) -> TerminalData {
        TerminalData {
            name: name.to_string(),
            cwd: None,
            session_name: None,
        }
    }

    #[test]
    fn old_single_pane_json_loads_as_one_tab() {
        // Pre-split state.json stored tabs as bare TerminalData maps.
        let json = r#"{"name":"Terminal","cwd":"/tmp","session_name":null}"#;
        let tab: TabData = serde_json::from_str(json).unwrap();
        assert_eq!(tab.panes.len(), 1);
        assert_eq!(tab.panes[0].cwd.as_deref(), Some("/tmp"));
        assert_eq!(tab.root, SplitNode::Leaf(0));
    }

    #[test]
    fn split_then_close_round_trips() {
        let mut tab = TabData::single(pane("Term"));
        let second = tab.split_pane(0, SplitDir::Vertical);
        assert_eq!(second, 1);
        assert_eq!(tab.panes.len(), 2);
        assert_eq!(tab.leaves(), vec![0, 1]);
        assert!(matches!(
            tab.root,
            SplitNode::Split {
                dir: SplitDir::Vertical,
                ..
            }
        ));

        let third = tab.split_pane(1, SplitDir::Horizontal);
        assert_eq!(tab.panes.len(), 3);
        assert_eq!(tab.leaves().len(), 3);

        assert!(tab.close_pane(third));
        assert_eq!(tab.panes.len(), 2);
        assert_eq!(tab.leaves(), vec![0, 1]);
        assert!(tab.close_pane(1));
        assert_eq!(tab.root, SplitNode::Leaf(0));
        assert!(!tab.close_pane(0), "last pane must not close");
    }

    #[test]
    fn normalize_repairs_dangling_trees() {
        let mut tab = TabData {
            name: "T".into(),
            panes: vec![pane("a")],
            active_pane: 7,
            root: SplitNode::Leaf(9),
        };
        tab.normalize();
        assert_eq!(tab.root, SplitNode::Leaf(0));
        assert_eq!(tab.active_pane, 0);
    }

    #[test]
    fn allocations_tile_without_gaps_or_overlap() {
        let mut tab = TabData::single(pane("a"));
        tab.split_pane(0, SplitDir::Vertical);
        tab.split_pane(0, SplitDir::Horizontal);
        let rects = tab.allocations(900.0, 600.0);
        assert_eq!(rects.len(), 3);
        // Area is conserved.
        let area: f32 = rects.iter().map(|(_, _, _, w, h)| w * h).sum();
        assert!((area - 900.0 * 600.0).abs() < 1.0);
        // First-level vertical split at 50%: left stack is 450 wide.
        let left: Vec<_> = rects.iter().filter(|(_, x, _, _, _)| *x < 450.0).collect();
        assert_eq!(left.len(), 2);
    }
}
