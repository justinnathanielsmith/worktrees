use crate::domain::repository::Worktree;
use ratatui::widgets::TableState;

#[derive(Debug, Clone)]
pub enum PromptType {
    AddIntent,
    InitUrl,
    CommitMessage,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct EditorConfig {
    pub name: String,
    pub command: String,
}

#[derive(Clone)]
pub enum AppState {
    Welcome,
    Initializing { project_name: String },
    Initialized { project_name: String },
    AddingWorktree { intent: String, branch: String },
    WorktreeAdded { intent: String },
    RemovingWorktree { intent: String },
    WorktreeRemoved,
    Syncing { branch: String, prev_state: Box<AppState> },
    SyncComplete { branch: String, prev_state: Box<AppState> },
    SelectingEditor { 
        branch: String, 
        options: Vec<EditorConfig>,
        selected: usize,
        prev_state: Box<AppState> 
    },
    OpeningEditor { branch: String, editor: String, prev_state: Box<AppState> },
    ListingWorktrees { 
        worktrees: Vec<Worktree>, 
        table_state: TableState,
        refresh_needed: bool,
    },
    ViewingStatus {
        path: String,
        branch: String,
        staged: Vec<String>,
        unstaged: Vec<String>,
        untracked: Vec<String>,
        selected_index: usize, // Combined index for all three lists
        prev_state: Box<AppState>,
    },
    ViewingHistory {
        path: String,
        branch: String,
        commits: Vec<crate::domain::repository::GitCommit>,
        selected_index: usize,
        prev_state: Box<AppState>,
    },
    SwitchingBranch {
        path: String,
        branches: Vec<String>,
        selected_index: usize,
        prev_state: Box<AppState>,
    },
    Prompting {
        prompt_type: PromptType,
        input: String,
        prev_state: Box<AppState>,
    },
    SettingUpDefaults,
    SetupComplete,
    Error(String),
}

impl AppState {
    pub fn request_refresh(&mut self) {
        if let AppState::ListingWorktrees { refresh_needed, .. } = self {
            *refresh_needed = true;
        }
    }
}
