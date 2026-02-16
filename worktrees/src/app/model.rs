use crate::domain::repository::Worktree;
use ratatui::widgets::TableState;

#[derive(Debug, Clone)]
pub enum PromptType {
    AddIntent,
    InitUrl,
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
    Syncing { branch: String },
    SyncComplete { branch: String },
    SelectingEditor { 
        branch: String, 
        options: Vec<EditorConfig>,
        selected: usize,
        prev_state: Box<AppState> 
    },
    OpeningEditor { branch: String, editor: String },
    ListingWorktrees { 
        worktrees: Vec<Worktree>, 
        table_state: TableState,
        refresh_needed: bool,
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
