use crate::app::intent::Intent;
use crate::domain::repository::Worktree;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum PromptType {
    NameNewWorktree { base_ref: String },
    CommitMessage,
    ApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DashboardTab {
    Info,
    Status,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefreshType {
    None,
    Dashboard,
    Full,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct EditorConfig {
    pub name: String,
    pub command: String,
}

impl EditorConfig {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                name: "VS Code".into(),
                command: "code".into(),
            },
            Self {
                name: "Cursor".into(),
                command: "cursor".into(),
            },
            Self {
                name: "Zed".into(),
                command: "zed".into(),
            },
            Self {
                name: "Android Studio".into(),
                command: "studio".into(),
            },
            Self {
                name: "IntelliJ IDEA".into(),
                command: "idea".into(),
            },
            Self {
                name: "Vim".into(),
                command: "vim".into(),
            },
            Self {
                name: "Neovim".into(),
                command: "nvim".into(),
            },
            Self {
                name: "Antigravity".into(),
                command: "antigravity".into(),
            },
        ]
    }
}

#[derive(Clone, Debug)]
pub struct StatusViewState {
    pub staged: Vec<String>,
    pub unstaged: Vec<String>,
    pub untracked: Vec<String>,
    pub selected_index: usize,
    pub diff_preview: Option<String>,
    pub show_diff: bool,
}

impl StatusViewState {
    pub fn total(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.untracked.len()
    }

    pub fn selected_file(&self) -> Option<&str> {
        let idx = self.selected_index;
        if idx < self.staged.len() {
            Some(&self.staged[idx])
        } else if idx < self.staged.len() + self.unstaged.len() {
            Some(&self.unstaged[idx - self.staged.len()])
        } else if idx < self.total() {
            Some(&self.untracked[idx - self.staged.len() - self.unstaged.len()])
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct DashboardState {
    pub active_tab: DashboardTab,
    pub cached_status: Option<crate::domain::repository::GitStatus>,
    pub cached_history: Option<Vec<crate::domain::repository::GitCommit>>,
}

/// The possible states of the TUI application.
#[derive(Clone)]
pub enum AppState {
    /// The starting state when no project is detected.
    Welcome,
    /// Actively initializing a new bare repository.
    Initializing { project_name: String },
    /// Successfully initialized a new repository.
    Initialized { project_name: String },
    /// Actively adding a new worktree.
    AddingWorktree { intent: String, branch: String },
    /// Successfully added a new worktree.
    WorktreeAdded { intent: String },
    /// Actively removing a worktree.
    RemovingWorktree { intent: String },
    /// Successfully removed a worktree.
    WorktreeRemoved,
    /// Confirming a destructive operation.
    Confirming {
        title: String,
        message: String,
        action: Box<Intent>,
        prev_state: Box<AppState>,
    },
    /// Synchronizing configuration files.
    Syncing {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Synchronization completed.
    SyncComplete {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Help modal showing shortcuts.
    Help {
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Fetching from remote.
    Fetching {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Pulling from remote.
    Pulling {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Pull completed.
    PullComplete {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Pushing changes to remote.
    Pushing {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Push completed.
    PushComplete {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// Selecting an editor to open a worktree.
    SelectingEditor {
        branch: String,
        options: Vec<EditorConfig>,
        selected: usize,
        prev_state: Box<AppState>,
    },
    /// Opening a worktree in the selected editor.
    OpeningEditor {
        branch: String,
        editor: String,
        #[allow(dead_code)]
        prev_state: Box<AppState>,
    },
    /// The primary state showing all active worktrees.
    ListingWorktrees {
        worktrees: Vec<Worktree>,
        table_state: TableState,
        refresh_needed: RefreshType,
        selection_mode: bool,
        dashboard: DashboardState,
        filter_query: String,
        is_filtering: bool,
    },
    /// Detailed Git status view for a specific worktree.
    ViewingStatus {
        path: String,
        branch: String,
        status: StatusViewState,
        prev_state: Box<AppState>,
    },
    /// Git commit history log view.
    ViewingHistory {
        branch: String,
        commits: Vec<crate::domain::repository::GitCommit>,
        selected_index: usize,
        prev_state: Box<AppState>,
    },
    /// Branch selection menu for switching worktree branches.
    SwitchingBranch {
        path: String,
        branches: Vec<String>,
        selected_index: usize,
        prev_state: Box<AppState>,
    },
    /// Commit menu selection.
    Committing {
        path: String,
        branch: String,
        selected_index: usize,
        prev_state: Box<AppState>,
    },
    /// Branch selection menu for creating a new worktree.
    PickingBaseRef {
        branches: Vec<String>,
        selected_index: usize,
        prev_state: Box<AppState>,
    },
    /// General purpose text input prompt.
    Prompting {
        prompt_type: PromptType,
        input: String,
        prev_state: Box<AppState>,
    },
    /// Initial setup of canonical worktrees.
    SettingUpDefaults,
    /// Canonical setup completed.
    SetupComplete,
    /// Temporary state that transitions after a duration.
    Timed {
        inner_state: Box<AppState>,
        target_state: Box<AppState>,
        start_time: Instant,
        duration: Duration,
    },
    /// An error state with a message.
    Error(String, #[allow(dead_code)] Box<AppState>),
    /// Signal to exit the application.
    Exiting(Option<String>),
}

impl AppState {
    /// Signals that the worktree list needs to be re-fetched from the repository.
    pub fn request_refresh(&mut self) {
        if let AppState::ListingWorktrees { refresh_needed, .. } = self {
            *refresh_needed = RefreshType::Full;
        }
    }

    /// Helper to extract the previous state from states that track it.
    #[allow(dead_code)]
    pub fn prev_state_boxed(&self) -> &AppState {
        match self {
            AppState::Confirming { prev_state, .. } => prev_state,
            AppState::Syncing { prev_state, .. } => prev_state,
            AppState::SyncComplete { prev_state, .. } => prev_state,
            AppState::Help { prev_state } => prev_state,
            AppState::Fetching { prev_state, .. } => prev_state,
            AppState::Pulling { prev_state, .. } => prev_state,
            AppState::PullComplete { prev_state, .. } => prev_state,
            AppState::Pushing { prev_state, .. } => prev_state,
            AppState::PushComplete { prev_state, .. } => prev_state,
            AppState::SelectingEditor { prev_state, .. } => prev_state,
            AppState::OpeningEditor { prev_state, .. } => prev_state,
            AppState::ViewingStatus { prev_state, .. } => prev_state,
            AppState::ViewingHistory { prev_state, .. } => prev_state,
            AppState::SwitchingBranch { prev_state, .. } => prev_state,
            AppState::Committing { prev_state, .. } => prev_state,
            AppState::PickingBaseRef { prev_state, .. } => prev_state,
            AppState::Prompting { prev_state, .. } => prev_state,
            AppState::Timed { target_state, .. } => target_state,
            AppState::Error(_, prev_state) => prev_state,
            _ => panic!("State does not have a previous state"),
        }
    }
}

pub fn filter_worktrees(worktrees: &[Worktree], query: &str) -> Vec<Worktree> {
    if query.is_empty() {
        return worktrees.to_vec();
    }
    let query = query.to_lowercase();
    worktrees
        .iter()
        .filter(|wt| {
            wt.branch.to_lowercase().contains(&query) || wt.path.to_lowercase().contains(&query)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_worktrees() {
        let worktrees = vec![
            Worktree {
                path: "/path/to/main".to_string(),
                branch: "main".to_string(),
                commit: "123".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: None,
                metadata: None,
            },
            Worktree {
                path: "/path/to/dev".to_string(),
                branch: "dev".to_string(),
                commit: "456".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: None,
                metadata: None,
            },
            Worktree {
                path: "/path/to/feature-login".to_string(),
                branch: "feature/login".to_string(),
                commit: "789".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: None,
                metadata: None,
            },
        ];

        // Empty query returns all
        let filtered = filter_worktrees(&worktrees, "");
        assert_eq!(filtered.len(), 3);

        // Exact match branch
        let filtered = filter_worktrees(&worktrees, "main");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "main");

        // Partial match branch
        let filtered = filter_worktrees(&worktrees, "login");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "feature/login");

        // Match path
        let filtered = filter_worktrees(&worktrees, "feature-login");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "feature/login");

        // Case insensitive
        let filtered = filter_worktrees(&worktrees, "MAIN");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "main");

        // No match
        let filtered = filter_worktrees(&worktrees, "xyz");
        assert_eq!(filtered.len(), 0);
    }
}
