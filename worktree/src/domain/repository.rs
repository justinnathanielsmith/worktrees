use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Custom attributes for a worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorktreeMetadata {
    pub created_at: Option<String>,
    pub purpose: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

/// Represents a Git worktree and its current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// The size of the worktree directory on disk in bytes.
    pub size_bytes: u64,
    /// Custom metadata for this worktree.
    pub metadata: Option<WorktreeMetadata>,
}

/// Detailed git status of a specific worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitStatus {
    pub staged: Vec<(String, String)>,   // (path, code)
    pub unstaged: Vec<(String, String)>, // (path, code)
    pub untracked: Vec<String>,
}

/// Information about a single Git commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub graph: String,
}

/// The architectural context of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProjectContext {
    /// A standard project with no specialized sync logic.
    Standard,
    /// A Kotlin Multiplatform or Android project requiring specific properties sync.
    KmpAndroid,
}

/// Represents a Git stash entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub branch: String,
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
    /// Rebases the worktree onto the specified upstream branch.
    fn rebase(&self, path: &str, upstream: &str) -> Result<()>;
    /// Retrieves the diff of files with conflicts.
    fn get_conflict_diff(&self, path: &str) -> Result<String>;
    /// Explains a git conflict using AI.
    fn explain_rebase_conflict(&self, diff: &str) -> Result<String>;

    // --- Stash Operations ---
    /// Lists all stashes.
    fn list_stashes(&self, path: &str) -> Result<Vec<StashEntry>>;
    /// Applies the stash at the given index.
    fn apply_stash(&self, path: &str, index: usize) -> Result<()>;
    /// Pops the stash at the given index.
    fn pop_stash(&self, path: &str, index: usize) -> Result<()>;
    /// Drops the stash at the given index.
    fn drop_stash(&self, path: &str, index: usize) -> Result<()>;
    /// Pushes current changes to a new stash.
    fn stash_save(&self, path: &str, message: Option<&str>) -> Result<()>;

    // --- AI Configuration ---

    /// Retrieves the Gemini API key from environment or local storage.
    fn get_api_key(&self) -> Result<Option<String>>;
    /// Persists the Gemini API key.
    fn set_api_key(&self, key: &str) -> Result<()>;
    /// Cleans up stale worktrees (missing metadata or deleted branches).
    /// Returns a list of paths that were (or would be) removed.
    fn clean_worktrees(&self, dry_run: bool, artifacts: bool) -> Result<Vec<String>>;

    /// Resolves the absolute path to the root of the project (the "Bare Hub").
    fn get_project_root(&self) -> Result<std::path::PathBuf>;

    /// Converts a standard repository to a bare hub structure.
    /// Returns the path to the newly created hub directory.
    fn convert_to_bare(
        &self,
        name: Option<&str>,
        branch: Option<&str>,
    ) -> Result<std::path::PathBuf>;

    /// Checks the status of the current directory.
    fn check_status(&self, path: &std::path::Path) -> RepoStatus;

    /// Watches the repository for changes.
    /// Returns a channel receiver that emits repository events.
    fn watch(&self) -> Result<crossbeam_channel::Receiver<RepositoryEvent>>;

    /// Migrates the current standard repository to a Bare Hub structure in-place.
    /// Returns the path to the main worktree.
    fn migrate_to_bare(&self, force: bool, dry_run: bool) -> Result<std::path::PathBuf>;
}

/// Events emitted by the repository watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RepositoryEvent {
    /// The list of worktrees has changed (added, removed, pruned).
    #[allow(dead_code)]
    WorktreeListChanged,
    /// The git status of a specific worktree has changed.
    #[allow(dead_code)]
    StatusChanged(String),
    /// The HEAD of a specific worktree has changed (commit/checkout).
    #[allow(dead_code)]
    HeadChanged(String),
    /// A generic change that might require a full refresh.
    RescanRequired,
}

/// The status of the current repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoStatus {
    /// A valid Bare Hub (root or worktree).
    BareHub,
    /// A standard Git repository.
    StandardGit,
    /// Not a known Git repository format.
    NoRepo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_metadata_serialization() {
        let meta = WorktreeMetadata {
            purpose: Some("Feature: Login UI".to_string()),
            created_at: Some("2023-10-27".to_string()),
            ..WorktreeMetadata::default()
        };

        let json = serde_json::to_string(&meta).unwrap();
        let decoded: WorktreeMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.purpose, Some("Feature: Login UI".to_string()));
        assert_eq!(decoded.created_at, Some("2023-10-27".to_string()));
    }
}
