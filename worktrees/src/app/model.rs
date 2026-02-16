use crate::app::intent::Intent;
use crate::domain::repository::Worktree;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum PromptType {
    AddIntent,
    CommitMessage,
    ApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DashboardTab {
    Info,
    Status,
    Log,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct EditorConfig {
    pub name: String,
    pub command: String,
}

#[derive(Clone, Debug)]
pub struct StatusViewState {
    pub staged: Vec<String>,
    pub unstaged: Vec<String>,
    pub untracked: Vec<String>,
    pub selected_index: usize,
}

impl StatusViewState {
    pub fn total(&self) -> usize {
        self.staged.len() + self.unstaged.len() + self.untracked.len()
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
        prev_state: Box<AppState>,
    },
    /// Synchronization completed.
    SyncComplete {
        branch: String,
        prev_state: Box<AppState>,
    },
    /// Help modal showing shortcuts.
    Help { prev_state: Box<AppState> },
    /// Fetching from remote.
    Fetching {
        branch: String,
        prev_state: Box<AppState>,
    },
    /// Pushing changes to remote.
    Pushing {
        branch: String,
        prev_state: Box<AppState>,
    },
    /// Push completed.
    PushComplete {
        branch: String,
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
        prev_state: Box<AppState>,
    },
    /// The primary state showing all active worktrees.
    ListingWorktrees {
        worktrees: Vec<Worktree>,
        table_state: TableState,
        refresh_needed: bool,
        selection_mode: bool,
        dashboard: DashboardState,
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
    Error(String, Box<AppState>),
    /// Signal to exit the application.
    Exiting(Option<String>),
}

impl AppState {
    /// Signals that the worktree list needs to be re-fetched from the repository.
    pub fn request_refresh(&mut self) {
        if let AppState::ListingWorktrees { refresh_needed, .. } = self {
            *refresh_needed = true;
        }
    }

    /// Helper to extract the previous state from states that track it.
    pub fn prev_state_boxed(&self) -> &AppState {
        match self {
            AppState::Confirming { prev_state, .. } => prev_state,
            AppState::Syncing { prev_state, .. } => prev_state,
            AppState::SyncComplete { prev_state, .. } => prev_state,
            AppState::Help { prev_state } => prev_state,
            AppState::Fetching { prev_state, .. } => prev_state,
            AppState::Pushing { prev_state, .. } => prev_state,
            AppState::PushComplete { prev_state, .. } => prev_state,
            AppState::SelectingEditor { prev_state, .. } => prev_state,
            AppState::OpeningEditor { prev_state, .. } => prev_state,
            AppState::ViewingStatus { prev_state, .. } => prev_state,
            AppState::ViewingHistory { prev_state, .. } => prev_state,
            AppState::SwitchingBranch { prev_state, .. } => prev_state,
            AppState::Committing { prev_state, .. } => prev_state,
            AppState::Prompting { prev_state, .. } => prev_state,
            AppState::Timed { target_state, .. } => target_state,
            AppState::Error(_, prev_state) => prev_state,
            _ => panic!("State does not have a previous state"),
        }
    }
}
