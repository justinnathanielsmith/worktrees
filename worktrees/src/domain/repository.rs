use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Represents a Git worktree and its current state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Worktree {
    /// The absolute path to the worktree on disk.
    pub path: String,
    /// The short commit hash currently checked out.
    pub commit: String,
    /// The name of the branch currently checked out.
    pub branch: String,
    /// Whether this is the primary bare repository (hub).
    pub is_bare: bool,
    /// Whether the HEAD is in a detached state.
    pub is_detached: bool,
    /// A summarized string of git status (e.g., "+2 ~1").
    pub status_summary: Option<String>,
}

/// Detailed git status of a specific worktree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GitStatus {
    pub staged: Vec<String>,
    pub unstaged: Vec<String>,
    pub untracked: Vec<String>,
}

/// Information about a single Git commit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// The architectural context of the project.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ProjectContext {
    /// A standard project with no specialized sync logic.
    Standard,
    /// A Kotlin Multiplatform or Android project requiring specific properties sync.
    KmpAndroid,
}

/// The core abstraction for interacting with Git and project-specific configurations.
pub trait ProjectRepository {
    /// Initializes a new bare repository at the specified project name directory.
    fn init_bare_repo(&self, url: Option<&str>, project_name: &str) -> Result<()>;
    /// Adds an existing branch as a new worktree at the given path.
    fn add_worktree(&self, path: &str, branch: &str) -> Result<()>;
    /// Creates a new branch from a base and adds it as a worktree.
    fn add_new_worktree(&self, path: &str, branch: &str, base: &str) -> Result<()>;
    /// Removes a worktree and its associated files.
    fn remove_worktree(&self, path: &str, force: bool) -> Result<()>;
    /// Lists all worktrees managed by the current bare repository.
    fn list_worktrees(&self) -> Result<Vec<Worktree>>;
    /// Synchronizes configuration files (symlinks/copies) to the target worktree.
    fn sync_configs(&self, path: &str) -> Result<()>;
    /// Detects the project context (e.g., Standard vs KMP).
    fn detect_context(&self, base_path: &Path) -> ProjectContext;
    /// Retrieves the user's preferred editor for opening worktrees.
    fn get_preferred_editor(&self) -> Result<Option<String>>;
    /// Persists the user's preferred editor command.
    fn set_preferred_editor(&self, editor: &str) -> Result<()>;

    // --- Git Operations ---

    /// Fetches all remotes and prunes stale branches.
    fn fetch(&self, path: &str) -> Result<()>;
    /// Pulls changes from the remote repository.
    fn pull(&self, path: &str) -> Result<()>;
    /// Pushes committed changes to the remote repository.
    fn push(&self, path: &str) -> Result<()>;
    /// Retrieves the porcelain status for the given worktree path.
    fn get_status(&self, path: &str) -> Result<GitStatus>;
    /// Stages all changes (modified and untracked).
    fn stage_all(&self, path: &str) -> Result<()>;
    /// Unstages all changes.
    fn unstage_all(&self, path: &str) -> Result<()>;
    /// Stages a file for commit.
    fn stage_file(&self, path: &str, file: &str) -> Result<()>;
    /// Unstages a file.
    fn unstage_file(&self, path: &str, file: &str) -> Result<()>;
    /// Creates a new commit with the given message.
    fn commit(&self, path: &str, message: &str) -> Result<()>;
    /// Generates a diff string for the current changes.
    fn get_diff(&self, path: &str) -> Result<String>;
    /// Generates a conventional commit message using AI (Gemini).
    fn generate_commit_message(&self, diff: &str, branch: &str) -> Result<String>;
    /// Retrieves the recent commit history.
    fn get_history(&self, path: &str, limit: usize) -> Result<Vec<GitCommit>>;
    /// Lists all available local branches.
    fn list_branches(&self) -> Result<Vec<String>>;
    /// Switches the worktree to a different branch.
    fn switch_branch(&self, path: &str, branch: &str) -> Result<()>;

    // --- AI Configuration ---

    /// Retrieves the Gemini API key from environment or local storage.
    fn get_api_key(&self) -> Result<Option<String>>;
    /// Persists the Gemini API key.
    fn set_api_key(&self, key: &str) -> Result<()>;
    /// Cleans up stale worktrees (missing metadata or deleted branches).
    /// Returns a list of paths that were (or would be) removed.
    fn clean_worktrees(&self, dry_run: bool) -> Result<Vec<String>>;
}
