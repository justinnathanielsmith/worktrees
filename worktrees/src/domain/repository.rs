use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Worktree {
    pub path: String,
    pub commit: String,
    pub branch: String,
    pub is_bare: bool,
    pub is_detached: bool,
    pub status_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GitStatus {
    pub staged: Vec<String>,
    pub unstaged: Vec<String>,
    pub untracked: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ProjectContext {
    Standard,
    KmpAndroid,
}

pub trait ProjectRepository {
    fn init_bare_repo(&self, url: &str, project_name: &str) -> Result<()>;
    fn add_worktree(&self, path: &str, branch: &str) -> Result<()>;
    fn add_new_worktree(&self, path: &str, branch: &str, base: &str) -> Result<()>;
    fn remove_worktree(&self, path: &str, force: bool) -> Result<()>;
    fn list_worktrees(&self) -> Result<Vec<Worktree>>;
    fn sync_configs(&self, path: &str) -> Result<()>;
    fn detect_context(&self) -> ProjectContext;
    fn get_preferred_editor(&self) -> Result<Option<String>>;
    fn set_preferred_editor(&self, editor: &str) -> Result<()>;

    // Git Operations
    fn fetch(&self, path: &str) -> Result<()>;
    fn get_status(&self, path: &str) -> Result<GitStatus>;
    fn stage_file(&self, path: &str, file: &str) -> Result<()>;
    fn unstage_file(&self, path: &str, file: &str) -> Result<()>;
    fn commit(&self, path: &str, message: &str) -> Result<()>;
    fn get_history(&self, path: &str, limit: usize) -> Result<Vec<GitCommit>>;
    fn list_branches(&self) -> Result<Vec<String>>;
    fn switch_branch(&self, path: &str, branch: &str) -> Result<()>;
}
