use crate::domain::repository::{
    GitCommit, GitStatus, ProjectContext, ProjectRepository, Worktree,
};
use anyhow::{Context, Result};
use keyring::Entry;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, instrument};

#[derive(Clone)]
pub struct GitProjectRepository;

impl GitProjectRepository {
    #[instrument]
    fn run_git(args: &[&str]) -> Result<String> {
        let git_cmd = std::env::var("WORKTREES_GIT_PATH").unwrap_or_else(|_| "git".to_string());
        debug!(command = %git_cmd, ?args, "Executing git command");

        let output = Command::new(&git_cmd)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute git {:?}. HELP: Ensure 'git' is installed and you have the necessary permissions.", args))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Git error: {}. HELP: Check your network connection or repository permissions.",
                err
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_global_config_path(&self, filename: &str) -> Option<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;
        Some(Path::new(&home).join(filename))
    }

    fn handle_context_files(&self, path: &str) {
        // 1. Generic synchronization from manifest
        if let Err(e) = self.sync_configs(path) {
            debug!(error = %e, "Generic configuration synchronization failed.");
        }

        // 2. Specialized KMP/Android synchronization
        if self.detect_context() == ProjectContext::KmpAndroid {
            // 1. Sync local.properties (Android SDK paths, etc.)
            let local_props = Path::new("local.properties");
            if local_props.exists() {
                let dest = Path::new(path).join("local.properties");
                let _ =
                    std::fs::copy(local_props, dest).context("Failed to copy local.properties.");
            }

            // 2. Sync and Optimize gradle.properties
            let gradle_props = Path::new("gradle.properties");
            let dest_gradle = Path::new(path).join("gradle.properties");

            if gradle_props.exists() {
                let _ = std::fs::copy(gradle_props, &dest_gradle)
                    .context("Failed to copy gradle.properties.");
            }

            // 3. Ensure Build Caching is enabled for Worktree performance
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dest_gradle)
            {
                // Check if already contains caching flag (simple check)
                if std::fs::read_to_string(&dest_gradle)
                    .map(|c| !c.contains("org.gradle.caching"))
                    .unwrap_or(false)
                {
                    let _ = writeln!(file, "\n# Optimized for Worktrees\norg.gradle.caching=true");
                }
            }
        }
    }

    fn get_status_summary(&self, path: &str) -> Result<String> {
        let output = Self::run_git(&["-C", path, "status", "--porcelain"])?;
        if output.trim().is_empty() {
            Ok("clean".to_string())
        } else {
            let mut staged = 0;
            let mut unstaged = 0;
            let mut untracked = 0;

            for line in output.lines() {
                if line.len() < 2 {
                    continue;
                }
                let s = &line[..2];
                match s {
                    "M " | "A " | "D " | "R " | "C " => staged += 1,
                    " M" | " D" | " T" => unstaged += 1,
                    "??" => untracked += 1,
                    "MM" | "MD" => {
                        staged += 1;
                        unstaged += 1;
                    }
                    _ => unstaged += 1,
                }
            }

            let mut summary = Vec::new();
            if staged > 0 {
                summary.push(format!("+{}", staged));
            }
            if unstaged > 0 {
                summary.push(format!("~{}", unstaged));
            }
            if untracked > 0 {
                summary.push(format!("?{}", untracked));
            }

            if summary.is_empty() {
                Ok("clean".to_string())
            } else {
                Ok(summary.join(" "))
            }
        }
    }
}

impl ProjectRepository for GitProjectRepository {
    fn init_bare_repo(&self, url: Option<&str>, project_name: &str) -> Result<()> {
        if Path::new(project_name).exists() {
            return Err(anyhow::anyhow!(
                "Directory '{}' already exists. HELP: Choose a different name or remove the existing directory.",
                project_name
            ));
        }

        std::fs::create_dir(project_name).context("Failed to create project directory. HELP: Ensure you have write permissions in the current folder.")?;
        std::env::set_current_dir(project_name).context("Failed to enter project directory.")?;

        if let Some(url_str) = url {
            Self::run_git(&["clone", "--bare", "--", url_str, ".bare"]).context("Failed to clone bare repository. HELP: Verify the repository URL and your SSH/HTTP credentials.")?;
        } else {
            Self::run_git(&["init", "--bare", ".bare"])
                .context("Failed to initialize bare repository.")?;
            // Ensure default branch is main
            Self::run_git(&["-C", ".bare", "symbolic-ref", "HEAD", "refs/heads/main"])?;
        }

        std::fs::write(".git", "gitdir: ./.bare\n")
            .context("Failed to write .git redirection file.")?;

        if url.is_some() {
            Self::run_git(&[
                "config",
                "remote.origin.fetch",
                "+refs/heads/*:refs/remotes/origin/*",
            ])?;
            Self::run_git(&["fetch", "origin"])?;
        }

        Ok(())
    }

    fn add_worktree(&self, path: &str, branch: &str) -> Result<()> {
        Self::run_git(&["worktree", "add", "--", path, branch]).context(format!(
            "Failed to add worktree '{}'. HELP: Ensure the branch '{}' exists on origin.",
            path, branch
        ))?;
        self.handle_context_files(path);
        Ok(())
    }

    fn add_new_worktree(&self, path: &str, branch: &str, base: &str) -> Result<()> {
        let res = Self::run_git(&["worktree", "add", "-b", branch, "--", path, base]);

        if res.is_err() && base == "HEAD" {
            debug!("Normal worktree add failed on HEAD, trying --orphan for fresh repository...");
            Self::run_git(&["worktree", "add", "--orphan", "-b", branch, path])
                .context(format!("Failed to create orphan worktree '{}'. HELP: Ensure your Git version is 2.42+ or manually create the first commit.", path))?;
            self.handle_context_files(path);
            return Ok(());
        }

        res.context(format!("Failed to create new worktree '{}' from '{}'. HELP: Ensure the base branch '{}' is valid.", path, base, base))?;
        self.handle_context_files(path);
        Ok(())
    }

    fn remove_worktree(&self, path: &str, force: bool) -> Result<()> {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push("--");
        args.push(path);

        Self::run_git(&args).context(format!("Failed to remove worktree '{}'. HELP: Ensure the directory is not in use by another process.", path))?;
        Ok(())
    }

    fn sync_configs(&self, path: &str) -> Result<()> {
        let manifest_path = Path::new(".worktrees.sync");
        if !manifest_path.exists() {
            debug!("No .worktrees.sync manifest found, skipping generic synchronization.");
            return Ok(());
        }

        let content = std::fs::read_to_string(manifest_path)
            .context("Failed to read .worktrees.sync manifest.")?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let action = parts[0];
            let source_name = parts[1];
            let source = Path::new(source_name);
            let destination = Path::new(path).join(source_name);

            if !source.exists() {
                debug!(
                    ?source,
                    "Source file defined in manifest does not exist, skipping."
                );
                continue;
            }

            match action {
                "symlink" => {
                    #[cfg(unix)]
                    {
                        if destination.exists() {
                            let _ = std::fs::remove_file(&destination);
                            let _ = std::fs::remove_dir_all(&destination);
                        }
                        std::os::unix::fs::symlink(source, &destination).with_context(|| {
                            format!("Failed to symlink {:?} to {:?}", source, destination)
                        })?;
                    }
                    #[cfg(not(unix))]
                    {
                        // Fallback to copy on non-unix systems for simplicity in this implementation
                        if source.is_dir() {
                            // Minimal dir copy logic or just skip
                        } else {
                            std::fs::copy(source, &destination)?;
                        }
                    }
                }
                "copy" => {
                    if source.is_dir() {
                        // For simplicity, we only support file copies in this version
                        debug!(?source, "Directory copy not yet supported, skipping.");
                    } else {
                        std::fs::copy(source, &destination).with_context(|| {
                            format!("Failed to copy {:?} to {:?}", source, destination)
                        })?;
                    }
                }
                _ => debug!(?action, "Unknown action in manifest, skipping."),
            }
        }

        Ok(())
    }

    fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let output = Self::run_git(&["worktree", "list", "--porcelain"])?;

        output
            .split("\n\n")
            .filter(|block| !block.is_empty())
            .map(|block| {
                let mut wt = block.lines().fold(
                    Worktree {
                        path: String::new(),
                        commit: String::new(),
                        branch: String::new(),
                        is_bare: false,
                        is_detached: false,
                        status_summary: None,
                    },
                    |mut wt, line| {
                        if let Some(path) = line.strip_prefix("worktree ") {
                            wt.path = path.to_string();
                        } else if let Some(head) = line.strip_prefix("HEAD ") {
                            wt.commit = head.chars().take(7).collect();
                        } else if let Some(branch) = line.strip_prefix("branch ") {
                            wt.branch = branch.trim_start_matches("refs/heads/").to_string();
                        } else if line == "bare" {
                            wt.is_bare = true;
                        } else if line == "detached" {
                            wt.is_detached = true;
                        }
                        wt
                    },
                );

                if !wt.is_bare && !wt.path.is_empty() {
                    wt.status_summary = self.get_status_summary(&wt.path).ok();
                }

                Ok(wt)
            })
            .collect()
    }

    fn detect_context(&self) -> ProjectContext {
        const INDICATORS: &[&str] = &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "local.properties",
        ];

        if INDICATORS.iter().any(|i| Path::new(i).exists()) {
            ProjectContext::KmpAndroid
        } else {
            ProjectContext::Standard
        }
    }

    fn get_preferred_editor(&self) -> Result<Option<String>> {
        if let Some(path) = self.get_global_config_path(".worktrees.editor") {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                return Ok(Some(content.trim().to_string()));
            }
        }
        Ok(None)
    }

    fn set_preferred_editor(&self, editor: &str) -> Result<()> {
        let path = self.get_global_config_path(".worktrees.editor").ok_or_else(|| {
            anyhow::anyhow!("Could not determine home directory for configuration")
        })?;
        std::fs::write(path, editor)?;
        Ok(())
    }

    fn fetch(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "fetch", "--all", "--prune"])?;
        Ok(())
    }

    fn push(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "push"])?;
        Ok(())
    }

    fn get_status(&self, path: &str) -> Result<GitStatus> {
        let output = Self::run_git(&["-C", path, "status", "--porcelain"])?;
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();

        for line in output.lines() {
            if line.len() < 4 {
                continue;
            }
            let status = &line[..2];
            let file = line[3..].to_string();

            match status {
                "M " | "A " | "D " | "R " | "C " => staged.push(file),
                " M" | " D" => unstaged.push(file),
                "??" => untracked.push(file),
                "MM" => {
                    staged.push(file.clone());
                    unstaged.push(file);
                }
                _ => {}
            }
        }

        Ok(GitStatus {
            staged,
            unstaged,
            untracked,
        })
    }

    fn stage_all(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "add", "."])?;
        Ok(())
    }

    fn unstage_all(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "restore", "--staged", "."])?;
        Ok(())
    }

    fn stage_file(&self, path: &str, file: &str) -> Result<()> {
        Self::run_git(&["-C", path, "add", file])?;
        Ok(())
    }

    fn unstage_file(&self, path: &str, file: &str) -> Result<()> {
        Self::run_git(&["-C", path, "reset", "HEAD", "--", file])?;
        Ok(())
    }

    fn commit(&self, path: &str, message: &str) -> Result<()> {
        Self::run_git(&["-C", path, "commit", "-m", message])?;
        Ok(())
    }

    fn get_diff(&self, path: &str) -> Result<String> {
        // Get staged changes first, as that's what we usually commit
        let output = Self::run_git(&["-C", path, "diff", "--cached"])?;
        if output.trim().is_empty() {
            // If no staged changes, look at unstaged changes for context
            return Self::run_git(&["-C", path, "diff"]);
        }
        Ok(output)
    }

    fn generate_commit_message(&self, diff: &str, branch: &str) -> Result<String> {
        debug!("Retrieving API key for commit message generation...");
        let api_key = self.get_api_key()?.ok_or_else(|| {
            debug!("API key not found in environment, keyring, or fallback file.");
            anyhow::anyhow!("Gemini API key not found. Set it with 'wt config set-key <key>' or GEMINI_API_KEY environment variable.")
        })?;

        debug!(
            "API key retrieved (length: {}). Initializing Gemini client...",
            api_key.len()
        );
        let client = super::gemini_client::GeminiClient::new(api_key);

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async { client.generate_commit_message(diff, branch).await })
        })
    }

    fn get_history(&self, path: &str, limit: usize) -> Result<Vec<GitCommit>> {
        let limit_str = limit.to_string();
        let output = Self::run_git(&[
            "-C",
            path,
            "log",
            &format!("-{}", limit_str),
            "--pretty=format:%h|%an|%ad|%s",
            "--date=short",
        ])?;

        Ok(output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 4 {
                    Some(GitCommit {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        message: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    fn list_branches(&self) -> Result<Vec<String>> {
        let output = Self::run_git(&["branch", "--format=%(refname:short)"])?;
        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    fn switch_branch(&self, path: &str, branch: &str) -> Result<()> {
        Self::run_git(&["-C", path, "checkout", branch])?;
        Ok(())
    }

    fn get_api_key(&self) -> Result<Option<String>> {
        // 1. Check Environment
        if let Ok(key) = std::env::var("GEMINI_API_KEY")
            && !key.trim().is_empty()
        {
            return Ok(Some(key.trim().to_string()));
        }

        // 2. Check Keyring
        let entry = Entry::new("worktrees", "gemini_api_key")
            .context("Failed to initialize system keyring entry for 'worktrees'")?;

        match entry.get_password() {
            Ok(password) if !password.trim().is_empty() => {
                return Ok(Some(password.trim().to_string()));
            }
            Ok(_) => { /* empty, continue */ }
            Err(keyring::Error::NoEntry) => { /* missing, continue */ }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "System keyring error ({}). Please ensure your system keychain is unlocked.",
                    e
                ));
            }
        }

        // 3. Check Legacy File
        if let Some(path) = self.get_global_config_path(".worktrees.gemini_key") {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let key = content.trim().to_string();
                if !key.is_empty() {
                    return Ok(Some(key));
                }
            }
        }

        Ok(None)
    }

    fn set_api_key(&self, key: &str) -> Result<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }

        // 1. Try Keyring
        let entry_res = Entry::new("worktrees", "gemini_api_key");
        match entry_res {
            Ok(entry) => {
                if let Err(e) = entry.set_password(key) {
                    debug!(error = %e, "Failed to store key in keyring, falling back to file.");
                }
            }
            Err(e) => debug!(error = %e, "Failed to initialize keyring, falling back to file."),
        }

        // 2. Always store in file as fallback/sync
        if let Some(path) = self.get_global_config_path(".worktrees.gemini_key") {
            std::fs::write(path, key).context("Failed to store API key in fallback file")?;
        }

        Ok(())
    }

    fn clean_worktrees(&self, dry_run: bool) -> Result<Vec<String>> {
        use std::fs;

        let bare_path = Path::new(".bare");
        if !bare_path.exists() {
            return Err(anyhow::anyhow!(
                "Not in a bare repository project. HELP: Run this command from the project root containing .bare/"
            ));
        }

        let worktrees_admin_path = bare_path.join("worktrees");
        if !worktrees_admin_path.exists() {
            debug!("No worktrees directory found, nothing to clean.");
            return Ok(Vec::new());
        }

        let mut to_remove = Vec::new();

        // Get list of valid worktrees from git
        let valid_worktrees = self.list_worktrees().unwrap_or_default();
        let valid_paths: std::collections::HashSet<String> =
            valid_worktrees.iter().map(|wt| wt.path.clone()).collect();

        // Scan the .bare/worktrees/ directory for stale entries
        let entries = fs::read_dir(&worktrees_admin_path)
            .context("Failed to read .bare/worktrees/ directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let worktree_name = entry.file_name().to_string_lossy().to_string();
            let gitdir_file = entry.path().join("gitdir");

            // Check if gitdir file exists and is valid
            let is_stale = if !gitdir_file.exists() {
                debug!(
                    worktree = %worktree_name,
                    "Missing gitdir file, marking as stale"
                );
                true
            } else {
                // Read the gitdir file to get the worktree path
                match fs::read_to_string(&gitdir_file) {
                    Ok(gitdir_content) => {
                        let worktree_path = gitdir_content.trim().trim_end_matches("/.git");
                        let worktree_exists = Path::new(worktree_path).exists();
                        let is_in_valid_list = valid_paths.contains(worktree_path);

                        if !worktree_exists {
                            debug!(
                                worktree = %worktree_name,
                                path = %worktree_path,
                                "Worktree directory does not exist, marking as stale"
                            );
                            true
                        } else if !is_in_valid_list {
                            debug!(
                                worktree = %worktree_name,
                                path = %worktree_path,
                                "Worktree not in git's valid list, marking as stale"
                            );
                            true
                        } else {
                            false
                        }
                    }
                    Err(e) => {
                        debug!(
                            worktree = %worktree_name,
                            error = %e,
                            "Failed to read gitdir file, marking as stale"
                        );
                        true
                    }
                }
            };

            if is_stale {
                to_remove.push(worktree_name);
            }
        }

        // If not dry-run, actually prune the stale worktrees
        if !dry_run && !to_remove.is_empty() {
            debug!("Running git worktree prune to clean up stale entries");
            Self::run_git(&["worktree", "prune", "-v"])
                .context("Failed to prune stale worktrees")?;
        }

        Ok(to_remove)
    }
}
