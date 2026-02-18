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
    StagedFile {
        path: String,
        result: anyhow::Result<()>,
    },
    UnstagedFile {
        path: String,
        result: anyhow::Result<()>,
    },
    StagedAll {
        path: String,
        result: anyhow::Result<()>,
    },
    UnstagedAll {
        path: String,
        result: anyhow::Result<()>,
    },
    BranchSwitched {
        path: String,
        result: anyhow::Result<()>,
    },
    DiffFetched {
        path: String,
        result: anyhow::Result<String>,
    },
    CommitMessageGenerated {
        result: anyhow::Result<String>,
    },
    StashesFetched {
        path: String,
        result: anyhow::Result<Vec<crate::domain::repository::StashEntry>>,
    },
    StashApplied {
        result: anyhow::Result<()>,
    },
    StashPopped {
        result: anyhow::Result<()>,
    },
    StashDropped {
        result: anyhow::Result<()>,
    },
    StashSaved {
        result: anyhow::Result<()>,
    },
    WorktreesListed {
        result: anyhow::Result<Vec<crate::domain::repository::Worktree>>,
    },
}
