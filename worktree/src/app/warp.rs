use crate::domain::repository::Worktree;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WarpConfig {
    pub name: String,
    pub windows: Vec<Window>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Window {
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tab {
    pub title: Option<String>,
    pub layout: Option<Layout>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layout {
    pub grid: Grid,
    pub panes: Vec<Pane>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Grid {
    pub rows: usize,
    pub columns: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pane {
    pub cwd: String,
    // Using Option<Vec<String>> to allow omission if empty
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}

pub fn generate_config(project_name: &str, worktrees: &[Worktree]) -> String {
    let count = worktrees.len();
    if count == 0 {
        return String::new();
    }

    // Filter to only include active worktrees (not bare hub itself usually, but we'll include all paths for now)
    // Actually, usually users want main, dev, and features.
    let columns = if count > 1 { 2 } else { 1 };
    let rows = (count as f32 / columns as f32).ceil() as usize;

    let panes: Vec<Pane> = worktrees
        .iter()
        .map(|wt| Pane {
            cwd: wt.path.clone(),
            commands: vec![], // No default commands to keep it clean
        })
        .collect();

    let config = WarpConfig {
        name: project_name.to_string(),
        windows: vec![Window {
            tabs: vec![Tab {
                title: Some("Worktrees".to_string()),
                layout: Some(Layout {
                    grid: Grid { rows, columns },
                    panes,
                }),
            }],
        }],
    };

    serde_yaml::to_string(&config).unwrap_or_default()
}
