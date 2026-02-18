use crate::app::intent::Intent;
use crate::domain::repository::Worktree;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum PromptType {
    NameNewWorktree { base_ref: String },
    CommitMessage,
    StashMessage,
    ApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    Manage,
    Git,
    Filter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardTab {
    Info,
    Status,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub staged: Vec<(String, String)>,
    pub unstaged: Vec<(String, String)>,
    pub untracked: Vec<String>,
    pub selected_index: usize,
    pub diff_preview: Option<String>,
    pub show_diff: bool,
}

impl StatusViewState {
    pub const fn total(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.untracked.len()
    }

    pub fn selected_file(&self) -> Option<&str> {
        let idx = self.selected_index;
        if idx < self.staged.len() {
            Some(&self.staged[idx].0)
        } else if idx < self.staged.len() + self.unstaged.len() {
            Some(&self.unstaged[idx - self.staged.len()].0)
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
    pub loading: bool,
}

/// The possible states of the TUI application.
#[derive(Clone, Debug)]
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
        prev_state: Box<Self>,
    },
    /// Synchronizing configuration files.
    Syncing {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Synchronization completed.
    SyncComplete {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Help modal showing shortcuts.
    Help {
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Fetching from remote.
    Fetching {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Pulling from remote.
    Pulling {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Pull completed.
    PullComplete {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Pushing changes to remote.
    Pushing {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Push completed.
    PushComplete {
        branch: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// Selecting an editor to open a worktree.
    SelectingEditor {
        branch: String,
        options: Vec<EditorConfig>,
        selected: usize,
        prev_state: Box<Self>,
    },
    /// Opening a worktree in the selected editor.
    OpeningEditor {
        branch: String,
        editor: String,
        #[allow(dead_code)]
        prev_state: Box<Self>,
    },
    /// The primary state showing all active worktrees.
    ListingWorktrees {
        worktrees: Vec<Worktree>,
        filtered_worktrees: Vec<Worktree>,
        table_state: TableState,
        refresh_needed: RefreshType,
        selection_mode: bool,
        dashboard: DashboardState,
        filter_query: String,
        is_filtering: bool,
        mode: AppMode,
        last_selection_change: Instant,
    },
    /// Detailed Git status view for a specific worktree.
    ViewingStatus {
        path: String,
        branch: String,
        status: StatusViewState,
        prev_state: Box<Self>,
    },
    /// Git stash view.
    ViewingStashes {
        path: String,
        branch: String,
        stashes: Vec<crate::domain::repository::StashEntry>,
        selected_index: usize,
        prev_state: Box<Self>,
    },
    /// Loading stashes for a worktree.
    LoadingStashes {
        path: String,
        branch: String,
        prev_state: Box<Self>,
    },
    /// Actively applying/popping/dropping/saving stash.
    StashAction {
        message: String,
        prev_state: Box<Self>,
    },
    /// Git commit history log view.
    ViewingHistory {
        branch: String,
        commits: Vec<crate::domain::repository::GitCommit>,
        selected_index: usize,
        prev_state: Box<Self>,
    },
    /// Loading status for a worktree.
    LoadingStatus {
        path: String,
        branch: String,
        prev_state: Box<Self>,
    },
    /// Loading history for a worktree.
    LoadingHistory {
        branch: String,
        prev_state: Box<Self>,
    },
    /// Loading branches for selection.
    LoadingBranches { prev_state: Box<Self> },
    /// Cleaning stale worktrees/artifacts.
    Cleaning { prev_state: Box<Self> },
    /// Actively staging a file.
    Staging { path: String, prev_state: Box<Self> },
    /// Actively unstaging a file.
    Unstaging { path: String, prev_state: Box<Self> },
    /// Actively switching branch.
    SwitchingBranchTask { path: String, prev_state: Box<Self> },
    /// Actively generating a commit message.
    GeneratingCommitMessage { prev_state: Box<Self> },
    /// Loading diff for preview.
    LoadingDiff { prev_state: Box<Self> },
    /// Branch selection menu for switching worktree branches.
    SwitchingBranch {
        path: String,
        branches: Vec<String>,
        selected_index: usize,
        prev_state: Box<Self>,
    },
    /// Commit menu selection.
    Committing {
        path: String,
        branch: String,
        selected_index: usize,
        prev_state: Box<Self>,
    },
    /// Branch selection menu for creating a new worktree.
    PickingBaseRef {
        branches: Vec<String>,
        selected_index: usize,
        prev_state: Box<Self>,
    },
    /// General purpose text input prompt.
    Prompting {
        prompt_type: PromptType,
        input: String,
        prev_state: Box<Self>,
    },
    /// Initial setup of canonical worktrees.
    SettingUpDefaults,
    /// Canonical setup completed.
    SetupComplete,
    /// Temporary state that transitions after a duration.
    Timed {
        inner_state: Box<Self>,
        target_state: Box<Self>,
        start_time: Instant,
        duration: Duration,
    },
    /// An error state with a message.
    Error(String, #[allow(dead_code)] Box<Self>),
    /// Signal to exit the application.
    Exiting(Option<String>),
}

impl AppState {
    /// Signals that the worktree list needs to be re-fetched from the repository.
    pub const fn request_refresh(&mut self) {
        if let Self::ListingWorktrees { refresh_needed, .. } = self {
            *refresh_needed = RefreshType::Full;
        }
    }

    /// Helper to extract the previous state from states that track it.
    #[allow(dead_code)]
    pub fn prev_state_boxed(&self) -> &Self {
        match self {
            Self::Confirming { prev_state, .. }
            | Self::Syncing { prev_state, .. }
            | Self::SyncComplete { prev_state, .. }
            | Self::Help { prev_state }
            | Self::Fetching { prev_state, .. }
            | Self::Pulling { prev_state, .. }
            | Self::PullComplete { prev_state, .. }
            | Self::Pushing { prev_state, .. }
            | Self::PushComplete { prev_state, .. }
            | Self::SelectingEditor { prev_state, .. }
            | Self::OpeningEditor { prev_state, .. }
            | Self::ViewingStatus { prev_state, .. }
            | Self::ViewingStashes { prev_state, .. }
            | Self::ViewingHistory { prev_state, .. }
            | Self::LoadingStatus { prev_state, .. }
            | Self::LoadingHistory { prev_state, .. }
            | Self::LoadingBranches { prev_state, .. }
            | Self::Cleaning { prev_state, .. }
            | Self::Staging { prev_state, .. }
            | Self::Unstaging { prev_state, .. }
            | Self::SwitchingBranchTask { prev_state, .. }
            | Self::GeneratingCommitMessage { prev_state, .. }
            | Self::LoadingDiff { prev_state, .. }
            | Self::LoadingStashes { prev_state, .. }
            | Self::StashAction { prev_state, .. }
            | Self::Error(_, prev_state) => prev_state,
            Self::Timed { target_state, .. } => target_state,
            _ => panic!("State does not have a previous state"),
        }
    }
}

pub fn filter_worktrees(worktrees: &[Worktree], query: &str) -> Vec<Worktree> {
    if query.is_empty() {
        return worktrees.to_vec();
    }

    let matcher = SkimMatcherV2::default();
    let query = query.to_lowercase();
    let mut scored_worktrees: Vec<(i64, Worktree)> = worktrees
        .iter()
        .filter_map(|wt| {
            // Match against both branch and path, take the best score
            let branch_score = matcher.fuzzy_match(&wt.branch, &query);
            let path_score = matcher.fuzzy_match(&wt.path, &query);

            match (branch_score, path_score) {
                (Some(s1), Some(s2)) => Some((s1.max(s2), wt.clone())),
                (Some(s), None) | (None, Some(s)) => Some((s, wt.clone())),
                (None, None) => None,
            }
        })
        .collect();

    // Sort by score descending
    scored_worktrees.sort_by(|a, b| b.0.cmp(&a.0));

    scored_worktrees.into_iter().map(|(_, wt)| wt).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_worktrees_fuzzy() {
        let worktrees = vec![
            Worktree {
                path: "/path/to/main".to_string(),
                branch: "main".to_string(),
                commit: "abc1234".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: Some("clean".to_string()),
                size_bytes: 0,
                metadata: None,
            },
            Worktree {
                path: "/path/to/dev".to_string(),
                branch: "dev".to_string(),
                commit: "def5678".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: Some("+1 ~2".to_string()),
                size_bytes: 0,
                metadata: None,
            },
            Worktree {
                path: "/path/to/feature-login".to_string(),
                branch: "feature/login".to_string(),
                commit: "789".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: Some("clean".to_string()),
                size_bytes: 0,
                metadata: None,
            },
        ];

        // Empty query returns all
        let filtered = filter_worktrees(&worktrees, "");
        assert_eq!(filtered.len(), 3);

        // Fuzzy match branch
        let filtered = filter_worktrees(&worktrees, "mn");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "main");

        // Partial match
        let filtered = filter_worktrees(&worktrees, "log");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "feature/login");

        // Fuzzy match across path/branch
        let filtered = filter_worktrees(&worktrees, "featlog");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "feature/login");

        // Case insensitive (handled by skim matcher)
        let filtered = filter_worktrees(&worktrees, "MAIN");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].branch, "main");

        // No match
        let filtered = filter_worktrees(&worktrees, "xyz");
        assert_eq!(filtered.len(), 0);
    }
}
