use crate::domain::repository::{GitCommit, GitStatus};

#[derive(Debug)]
pub enum AsyncAction {
    Fetch { path: String, branch: String },
    Pull { path: String, branch: String },
    Push { path: String, branch: String },
    GetStatus { path: String },
    GetHistory { path: String, limit: usize },
    SyncConfigs { path: String, branch: String },
}

#[derive(Debug)]
pub enum AsyncResult {
    FetchCompleted {
        branch: String,
        result: anyhow::Result<()>,
    },
    PullCompleted {
        branch: String,
        result: anyhow::Result<()>,
    },
    PushCompleted {
        branch: String,
        result: anyhow::Result<()>,
    },
    StatusFetched {
        path: String,
        result: anyhow::Result<GitStatus>,
    },
    HistoryFetched {
        path: String,
        result: anyhow::Result<Vec<GitCommit>>,
    },
    SyncCompleted {
        branch: String,
        result: anyhow::Result<()>,
    },
    BranchesFetched {
        result: anyhow::Result<Vec<String>>,
    },
    CleanCompleted {
        result: anyhow::Result<Vec<String>>,
    },
}
