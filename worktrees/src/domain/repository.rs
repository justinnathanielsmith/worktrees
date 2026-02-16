use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Worktree {
    pub path: String,
    pub commit: String,
    pub branch: String,
    pub is_bare: bool,
    pub is_detached: bool,
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
}
