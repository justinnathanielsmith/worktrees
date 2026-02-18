use crate::domain::repository::{
    GitCommit, GitStatus, ProjectContext, ProjectRepository, RepoStatus, Worktree, WorktreeMetadata,
};

use crate::domain::repository::RepositoryEvent;
use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use keyring::Entry;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, instrument};

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
            .with_context(|| format!("Failed to execute git {args:?}. HELP: Ensure 'git' is installed and you have the necessary permissions."))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Git error: {err}. HELP: Check your network connection or repository permissions."
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn resolve_config_path(legacy_filename: &str, new_filename: &str) -> Option<PathBuf> {
        // 1. Check legacy path first (for backward compatibility)
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let legacy_path = Path::new(&home).join(legacy_filename);
            if legacy_path.exists() {
                return Some(legacy_path);
            }
        }

        // 2. Check XDG_CONFIG_HOME explicitly (fixes tests and supports custom XDG locations)
        if let Ok(xdg_home) = std::env::var("XDG_CONFIG_HOME") {
            return Some(Path::new(&xdg_home).join("worktrees").join(new_filename));
        }

        // 3. Check Standard config path via dirs crate (Platform defaults)
        if let Some(config_dir) = dirs::config_dir() {
            return Some(config_dir.join("worktrees").join(new_filename));
        }

        // 4. Fallback to HOME if dirs fails (unlikely but safe)
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            return Some(Path::new(&home).join(legacy_filename));
        }

        None
    }

    fn handle_context_files(&self, path: &str) {
        // 1. Generic synchronization from manifest
        if let Err(e) = self.sync_configs(path) {
            debug!(error = %e, "Generic configuration synchronization failed.");
        }

        // 2. Specialized KMP/Android synchronization
        if self.detect_context(Path::new(".")) == ProjectContext::KmpAndroid {
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

    fn get_status_summary(path: &str) -> Result<String> {
        let output = Self::run_git(&["-C", path, "status", "--porcelain"])?;
        let status = Self::parse_status_output(&output);

        let staged = status.staged.len();
        let unstaged = status.unstaged.len();
        let untracked = status.untracked.len();

        let mut summary = Vec::new();
        if staged > 0 {
            summary.push(format!("+{staged}"));
        }
        if unstaged > 0 {
            summary.push(format!("~{unstaged}"));
        }
        if untracked > 0 {
            summary.push(format!("?{untracked}"));
        }

        if summary.is_empty() {
            Ok("clean".to_string())
        } else {
            Ok(summary.join(" "))
        }
    }

    fn parse_branches(output: &str) -> Vec<String> {
        let mut branches: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 1. First pass: Collect local branches
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains("origin/HEAD") {
                continue;
            }
            if !line.starts_with("origin/") {
                branches.push(line.to_string());
                seen.insert(line.to_string());
            }
        }

        // 2. Second pass: Add remote branches
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains("origin/HEAD") {
                continue;
            }
            if let Some(stripped) = line.strip_prefix("origin/")
                && !seen.contains(stripped)
            {
                branches.push(stripped.to_string());
                seen.insert(stripped.to_string());
            }
        }

        branches.sort();
        branches
    }

    pub(crate) fn parse_stash_list(output: &str) -> Vec<crate::domain::repository::StashEntry> {
        use crate::domain::repository::StashEntry;
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 3 {
                    // stash@{0}
                    let index_str = parts[0].trim().strip_prefix("stash@{")?.strip_suffix("}")?;
                    let index = index_str.parse::<usize>().ok()?;
                    Some(StashEntry {
                        index,
                        message: parts[1].trim().to_string(),
                        branch: parts[2].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn parse_git_history(output: &str) -> Vec<GitCommit> {
        output
            .lines()
            .filter_map(|line| {
                // The format is: graph\x00hash\x00author\x00date\x00message
                let parts: Vec<&str> = line.splitn(5, '\x00').collect();
                if parts.len() == 5 {
                    Some(GitCommit {
                        graph: parts[0].to_string(),
                        hash: parts[1].to_string(),
                        author: parts[2].to_string(),
                        date: parts[3].to_string(),
                        message: parts[4].to_string(),
                    })
                } else if parts.len() == 1 && !parts[0].trim().is_empty() {
                    // This handles lines that only contain graph components (lines between commits)
                    Some(GitCommit {
                        graph: parts[0].to_string(),
                        hash: String::new(),
                        author: String::new(),
                        date: String::new(),
                        message: String::new(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_metadata_path() -> PathBuf {
        Path::new(".worktree.json").to_path_buf()
    }

    fn load_metadata() -> std::collections::HashMap<String, WorktreeMetadata> {
        let path = Self::get_metadata_path();
        if !path.exists() {
            return std::collections::HashMap::new();
        }

        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    fn save_metadata(metadata: &std::collections::HashMap<String, WorktreeMetadata>) -> Result<()> {
        let path = Self::get_metadata_path();
        let content = serde_json::to_string_pretty(metadata)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn get_project_root_path() -> Result<PathBuf> {
        // Use git rev-parse --git-common-dir to find the bare repo location
        let output = Self::run_git(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let common_dir = Path::new(output.trim());

        // The project root is the parent of the .bare (or .git) directory
        common_dir
            .parent()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not determine project root from git common dir: {common_dir:?}"
                )
            })
    }

    fn parse_status_output(output: &str) -> GitStatus {
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
                "M " | "A " | "D " | "R " | "C " => staged.push((file, status.to_string())),
                " M" | " D" => unstaged.push((file, status.to_string())),
                "??" => untracked.push(file),
                "MM" => {
                    staged.push((file.clone(), "M ".to_string()));
                    unstaged.push((file, " M".to_string()));
                }
                "MD" => {
                    staged.push((file.clone(), "M ".to_string()));
                    unstaged.push((file, " D".to_string()));
                }
                _ => {
                    // Handle other porcelain status codes if they represent unstaged changes
                    if status.ends_with('M') || status.ends_with('D') {
                        unstaged.push((file, status.to_string()));
                    }
                }
            }
        }

        GitStatus {
            staged,
            unstaged,
            untracked,
        }
    }

    fn parse_worktree_entry(block: &str) -> Worktree {
        block.lines().fold(
            Worktree {
                path: String::new(),
                commit: String::new(),
                branch: String::new(),
                is_bare: false,
                is_detached: false,
                status_summary: None,
                size_bytes: 0,
                metadata: None,
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
        )
    }
}

impl ProjectRepository for GitProjectRepository {
    fn init_bare_repo(&self, url: Option<&str>, project_name: &str) -> Result<()> {
        if Path::new(project_name).exists() {
            return Err(anyhow::anyhow!(
                "Directory '{project_name}' already exists. HELP: Choose a different name or remove the existing directory."
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
        let root = self.get_project_root()?;
        let abs_path = root.join(path);
        let abs_path_str = abs_path.to_string_lossy();

        Self::run_git(&["worktree", "add", "--", &abs_path_str, branch]).context(format!(
            "Failed to add worktree '{path}'. HELP: Ensure the branch '{branch}' exists on origin."
        ))?;
        self.handle_context_files(&abs_path_str);
        Ok(())
    }

    fn add_new_worktree(&self, path: &str, branch: &str, base: &str) -> Result<()> {
        let root = self.get_project_root()?;
        let abs_path = root.join(path);
        let abs_path_str = abs_path.to_string_lossy();

        let res = Self::run_git(&["worktree", "add", "-b", branch, "--", &abs_path_str, base]);

        if res.is_err() && base == "HEAD" {
            debug!("Normal worktree add failed on HEAD, trying --orphan for fresh repository...");
            Self::run_git(&["worktree", "add", "--orphan", "-b", branch, &abs_path_str])
                .context(format!("Failed to create orphan worktree '{path}'. HELP: Ensure your Git version is 2.42+ or manually create the first commit."))?;
            self.handle_context_files(&abs_path_str);
            return Ok(());
        }

        res.context(format!("Failed to create new worktree '{path}' from '{base}'. HELP: Ensure the base branch '{base}' is valid."))?;
        self.handle_context_files(&abs_path_str);
        Ok(())
    }

    fn remove_worktree(&self, path: &str, force: bool) -> Result<()> {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push("--");
        args.push(path);

        Self::run_git(&args).context(format!("Failed to remove worktree '{path}'. HELP: Ensure the directory is not in use by another process."))?;
        Ok(())
    }

    fn sync_configs(&self, path: &str) -> Result<()> {
        let manifest_path = Path::new(".worktrees.sync");
        if !manifest_path.exists() {
            debug!("No .worktrees.sync manifest found, skipping generic synchronization.");
            return Ok(());
        }

        let abs_manifest_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(manifest_path);

        let metadata = std::fs::metadata(manifest_path)
            .context("Failed to read metadata of .worktrees.sync manifest.")?;
        let mtime = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        static SYNC_CONFIG_CACHE: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<PathBuf, (std::time::SystemTime, String)>>,
        > = std::sync::OnceLock::new();
        let cache = SYNC_CONFIG_CACHE
            .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));

        let content = {
            let mut guard = cache.lock().unwrap();
            if let Some((cached_mtime, cached_content)) = guard.get(&abs_manifest_path) {
                if *cached_mtime == mtime {
                    debug!("Using cached .worktrees.sync manifest.");
                    cached_content.clone()
                } else {
                    let content = std::fs::read_to_string(manifest_path)
                        .context("Failed to read .worktrees.sync manifest.")?;
                    guard.insert(abs_manifest_path, (mtime, content.clone()));
                    content
                }
            } else {
                let content = std::fs::read_to_string(manifest_path)
                    .context("Failed to read .worktrees.sync manifest.")?;
                guard.insert(abs_manifest_path, (mtime, content.clone()));
                content
            }
        };

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

            if source.is_absolute()
                || source
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                debug!(
                    ?source,
                    "Source path is unsafe (absolute or contains '..'), skipping."
                );
                continue;
            }

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
                            format!("Failed to symlink {source:?} to {destination:?}")
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
                            format!("Failed to copy {source:?} to {destination:?}")
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

        let mut worktrees = output
            .split("\n\n")
            .filter(|block| !block.is_empty())
            .map(|block| {
                let mut wt = Self::parse_worktree_entry(block);

                if !wt.path.is_empty() {
                    // wt.size_bytes = Self::calculate_dir_size(Path::new(&wt.path));
                    wt.size_bytes = 0;
                }

                if !wt.is_bare && !wt.path.is_empty() {
                    wt.status_summary = Self::get_status_summary(&wt.path).ok();
                }

                Ok(wt)
            })
            .collect::<Result<Vec<_>>>()?;
        let metadata_map = Self::load_metadata();
        for wt in &mut worktrees {
            if let Some(meta) = metadata_map.get(&wt.branch) {
                wt.metadata = Some(meta.clone());
            }
        }

        Ok(worktrees)
    }

    fn detect_context(&self, base_path: &Path) -> ProjectContext {
        use std::ffi::OsStr;
        const INDICATORS: &[&str] = &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "local.properties",
        ];

        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                if INDICATORS.iter().any(|&i| OsStr::new(i) == file_name) {
                    return ProjectContext::KmpAndroid;
                }
            }
        }
        ProjectContext::Standard
    }

    fn get_preferred_editor(&self) -> Result<Option<String>> {
        if let Some(path) = Self::resolve_config_path(".worktrees.editor", "editor")
            && path.exists()
        {
            let content = std::fs::read_to_string(path)?;
            return Ok(Some(content.trim().to_string()));
        }
        Ok(None)
    }

    fn set_preferred_editor(&self, editor: &str) -> Result<()> {
        let path = Self::resolve_config_path(".worktrees.editor", "editor")
            .ok_or_else(|| anyhow::anyhow!("Could not determine configuration directory"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)
                .context("Failed to open editor config file with secure permissions")?;

            // Ensure permissions are set even if file existed
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o600);
            file.set_permissions(perms)?;

            file.write_all(editor.as_bytes())
                .context("Failed to write editor config to file")?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(path, editor)?;
        }

        Ok(())
    }

    fn fetch(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "fetch", "--all", "--prune"])?;
        Ok(())
    }

    fn pull(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "pull"])?;
        Ok(())
    }

    fn push(&self, path: &str) -> Result<()> {
        Self::run_git(&["-C", path, "push"])?;
        Ok(())
    }

    fn get_status(&self, path: &str) -> Result<GitStatus> {
        let output = Self::run_git(&["-C", path, "status", "--porcelain"])?;
        Ok(Self::parse_status_output(&output))
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
            anyhow::anyhow!("Gemini API key not found. Set it with 'worktree config set-key <key>' or GEMINI_API_KEY environment variable.")
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
            "--graph",
            &format!("-{limit_str}"),
            "--pretty=format:%x00%h%x00%an%x00%ad%x00%s",
            "--date=short",
        ])?;

        Ok(Self::parse_git_history(&output))
    }

    fn list_branches(&self) -> Result<Vec<String>> {
        let output = Self::run_git(&["branch", "-a", "--format=%(refname:short)"])?;
        Ok(Self::parse_branches(&output))
    }

    fn switch_branch(&self, path: &str, branch: &str) -> Result<()> {
        Self::run_git(&["-C", path, "checkout", branch])?;
        Ok(())
    }

    fn rebase(&self, path: &str, upstream: &str) -> Result<()> {
        Self::run_git(&["-C", path, "rebase", upstream])?;
        Ok(())
    }

    fn get_conflict_diff(&self, path: &str) -> Result<String> {
        // Find unmerged files
        let output = Self::run_git(&["-C", path, "diff", "--name-only", "--diff-filter=U"])?;
        let mut full_diff = String::new();

        for file in output.lines() {
            if file.is_empty() {
                continue;
            }
            let diff = Self::run_git(&["-C", path, "diff", file])?;
            full_diff.push_str(&diff);
            full_diff.push('\n');
        }

        if full_diff.is_empty() {
            // Fallback: just try a generic diff if no unmerged files found but we are in conflict
            full_diff = Self::run_git(&["-C", path, "diff"])?;
        }

        Ok(full_diff)
    }

    fn explain_rebase_conflict(&self, diff: &str) -> Result<String> {
        debug!("Retrieving API key for conflict explanation...");
        let api_key = match self.get_api_key() {
            Ok(Some(key)) => key,
            _ => {
                return Ok("Rebase failed due to conflicts. Please resolve the conflicts manually by searching for conflict markers (<<<<<<<, =======, >>>>>>>) in the files listed above.".to_string());
            }
        };

        let client = super::gemini_client::GeminiClient::new(api_key);

        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async { client.explain_rebase_conflict(diff).await })
        }).or_else(|e| {
            debug!(error = %e, "Gemini explanation failed, using fallback.");
            Ok("Rebase failed due to conflicts. Please resolve them manually. (AI explanation unavailable)".to_string())
        })
    }

    fn list_stashes(&self, path: &str) -> Result<Vec<crate::domain::repository::StashEntry>> {
        let output = Self::run_git(&["-C", path, "stash", "list", "--pretty=format:%gd|%gs|%sb"])?;
        Ok(Self::parse_stash_list(&output))
    }

    fn apply_stash(&self, path: &str, index: usize) -> Result<()> {
        Self::run_git(&["-C", path, "stash", "apply", &format!("stash@{{{index}}}")])?;
        Ok(())
    }

    fn pop_stash(&self, path: &str, index: usize) -> Result<()> {
        Self::run_git(&["-C", path, "stash", "pop", &format!("stash@{{{index}}}")])?;
        Ok(())
    }

    fn drop_stash(&self, path: &str, index: usize) -> Result<()> {
        Self::run_git(&["-C", path, "stash", "drop", &format!("stash@{{{index}}}")])?;
        Ok(())
    }

    fn stash_save(&self, path: &str, message: Option<&str>) -> Result<()> {
        let mut args = vec!["-C", path, "stash", "push"];
        if let Some(msg) = message {
            args.push("-m");
            args.push(msg);
        }
        Self::run_git(&args)?;
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
            Ok(_) | Err(keyring::Error::NoEntry) => { /* empty or missing, continue */ }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "System keyring error ({e}). Please ensure your system keychain is unlocked."
                ));
            }
        }

        // 3. Check Config File (Legacy or New)
        if let Some(path) = Self::resolve_config_path(".worktrees.gemini_key", "gemini_key")
            && path.exists()
        {
            let content = std::fs::read_to_string(path)?;
            let key = content.trim().to_string();
            if !key.is_empty() {
                return Ok(Some(key));
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
        if let Some(path) = Self::resolve_config_path(".worktrees.gemini_key", "gemini_key")
            && let Some(parent) = path.parent()
        {
            std::fs::create_dir_all(parent)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(path)
                    .context("Failed to open API key file with secure permissions")?;

                let mut perms = file.metadata()?.permissions();
                perms.set_mode(0o600);
                file.set_permissions(perms)?;

                file.write_all(key.as_bytes())
                    .context("Failed to write API key to file")?;
            }

            #[cfg(not(unix))]
            {
                std::fs::write(path, key).context("Failed to store API key in fallback file")?;
            }
        }

        Ok(())
    }

    fn clean_worktrees(&self, dry_run: bool, artifacts_only: bool) -> Result<Vec<String>> {
        use std::fs;

        let root = self.get_project_root()?;
        let bare_path = root.join(".bare");
        if !bare_path.exists() {
            return Err(anyhow::anyhow!(
                "Not in a bare repository project. HELP: Run this command from the project root containing .bare/"
            ));
        }

        let mut cleaned_paths = Vec::new();

        if artifacts_only {
            let current_dir = std::env::current_dir().context("Failed to get current directory")?;
            let worktrees = self.list_worktrees()?;

            let artifact_dirs = [
                "node_modules",
                "target",
                "build",
                "dist",
                ".gradle",
                "bin",
                "obj",
            ];

            for wt in worktrees {
                if wt.is_bare {
                    continue;
                }

                let wt_path = Path::new(&wt.path);

                // Safety: Only clean worktrees that are NOT the current one
                if wt_path == current_dir {
                    debug!(path = ?wt_path, "Skipping current worktree for artifact cleaning");
                    continue;
                }

                if !wt_path.exists() {
                    continue;
                }

                for artifact in &artifact_dirs {
                    let target = wt_path.join(artifact);
                    if target.exists() {
                        let path_str = target.to_string_lossy().to_string();
                        if dry_run {
                            cleaned_paths.push(format!("[dry-run] build artifact: {path_str}"));
                        } else {
                            match fs::remove_dir_all(&target) {
                                Ok(()) => cleaned_paths.push(format!("cleaned: {path_str}")),
                                Err(e) => {
                                    error!(error = %e, path = %path_str, "Failed to remove artifact directory");
                                }
                            }
                        }
                    }
                }
            }
            return Ok(cleaned_paths);
        }

        // --- Stale Worktree Cleanup (Original Logic) ---
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
            let is_stale = if gitdir_file.exists() {
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
            } else {
                debug!(
                    worktree = %worktree_name,
                    "Missing gitdir file, marking as stale"
                );
                true
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

    fn get_project_root(&self) -> Result<PathBuf> {
        Self::get_project_root_path()
    }

    fn convert_to_bare(&self, name: Option<&str>, branch: Option<&str>) -> Result<PathBuf> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        let git_dir = current_dir.join(".git");

        if !git_dir.exists() || !git_dir.is_dir() {
            return Err(anyhow::anyhow!(
                "Not a standard Git repository (missing .git directory). HELP: Ensure you are in the root of a standard repository."
            ));
        }

        // Determine current branch if not provided
        let branch = if let Some(b) = branch {
            b.to_string()
        } else {
            let out = Self::run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
            out.trim().to_string()
        };

        // Determine project name and hub directory
        let project_name = current_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project");
        let hub_name = name.map_or_else(
            || format!("{project_name}-hub"),
            std::string::ToString::to_string,
        );
        let parent_dir = current_dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Could not find parent directory"))?;
        let hub_dir = parent_dir.join(&hub_name);

        if hub_dir.exists() {
            return Err(anyhow::anyhow!(
                "Target hub directory '{}' already exists. HELP: Choose a different name or remove the existing directory.",
                hub_dir.display()
            ));
        }

        // 1. Create new hub directory and move .git as .bare
        std::fs::create_dir_all(&hub_dir).context("Failed to create hub directory")?;
        let bare_dir = hub_dir.join(".bare");
        std::fs::rename(&git_dir, &bare_dir).context("Failed to move .git to .bare")?;

        // 2. Configure as bare
        Self::run_git(&[
            "-C",
            &bare_dir.to_string_lossy(),
            "config",
            "--bool",
            "core.bare",
            "true",
        ])
        .context("Failed to set core.bare to true")?;

        // 3. Create .git redirection file in the hub
        std::fs::write(hub_dir.join(".git"), "gitdir: ./.bare\n")
            .context("Failed to create .git redirection file in hub")?;

        // 4. Add the initial worktree
        let worktree_dir = hub_dir.join(&branch);
        Self::run_git(&[
            "-C",
            &hub_dir.to_string_lossy(),
            "worktree",
            "add",
            &worktree_dir.to_string_lossy(),
            &branch,
        ])
        .context(format!("Failed to add initial worktree '{branch}'"))?;

        Ok(hub_dir)
    }

    fn check_status(&self, path: &Path) -> RepoStatus {
        // 1. Check if it's a bare hub
        // A bare hub root has a .bare directory
        if path.join(".bare").exists() {
            return RepoStatus::BareHub;
        }

        // Or if we are inside a worktree of a bare hub
        // git rev-parse --git-common-dir should point to a .bare directory
        if let Ok(common_dir) = Self::run_git(&[
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--git-common-dir",
        ]) && common_dir.trim().ends_with(".bare")
        {
            return RepoStatus::BareHub;
        }

        // 2. Check if it's a standard git repo
        // git rev-parse --is-inside-work-tree should be true
        if let Ok(is_inside) = Self::run_git(&[
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--is-inside-work-tree",
        ]) && is_inside.trim() == "true"
        {
            return RepoStatus::StandardGit;
        }

        // Also check for .git directory directly in case we are at root
        if path.join(".git").exists() {
            return RepoStatus::StandardGit;
        }

        RepoStatus::NoRepo
    }

    fn watch(&self) -> Result<Receiver<RepositoryEvent>> {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (notify_tx, notify_rx) = crossbeam_channel::unbounded();
        let root = self.get_project_root()?;

        // Spawn a thread to handle watching
        std::thread::spawn(move || {
            let config = Config::default();
            let mut watcher: RecommendedWatcher = match Watcher::new(
                move |res| {
                    let _ = notify_tx.send(res);
                },
                config,
            ) {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to create watcher: {}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&root, RecursiveMode::Recursive) {
                error!("Failed to watch root: {}", e);
                return;
            }

            // Simple loop to receive events and forward them mapped to RepositoryEvent
            for res in notify_rx {
                match res {
                    Ok(event) => {
                        // Basic heuristic mapping
                        let mut meaningful_change = false;
                        for path in &event.paths {
                            let path_str = path.to_string_lossy();
                            if path_str.contains(".git") || path_str.contains(".bare") {
                                // Git metadata change
                                if path_str.contains("index.lock")
                                    || path_str.contains("HEAD.lock")
                                    || path_str.contains(".lock")
                                    || path_str.contains("node_modules")
                                    || path_str.contains("target")
                                {
                                    continue;
                                }
                                meaningful_change = true;
                                break;
                            }
                        }

                        if meaningful_change {
                            let _ = tx.send(RepositoryEvent::RescanRequired);
                        }
                    }
                    Err(e) => error!("Watch error: {}", e),
                }
            }
        });

        Ok(rx)
    }

    fn migrate_to_bare(&self, force: bool, dry_run: bool) -> Result<PathBuf> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        let git_dir = current_dir.join(".git");

        // 1. Validate Repo Status
        if !git_dir.exists() || !git_dir.is_dir() {
            return Err(anyhow::anyhow!(
                "Not a standard Git repository (missing .git directory). HELP: Ensure you are in the root of a standard repository."
            ));
        }

        // 2. Get Current Branch
        let branch = match Self::run_git(&["rev-parse", "--abbrev-ref", "HEAD"]) {
            Ok(out) => out.trim().to_string(),
            Err(_) => "main".to_string(), // Fallback
        };

        if dry_run {
            println!(
                "[Dry Run] Would migrate '{}' to Bare Hub structure.",
                current_dir.display()
            );
            println!("[Dry Run] 1. Move .git to .bare");
            println!("[Dry Run] 2. Configure .bare as bare repository");
            println!("[Dry Run] 3. Create worktree '{}' (no checkout)", branch);
            println!("[Dry Run] 4. Move all files to '{}'", branch);
            println!("[Dry Run] 5. Reset index in '{}'", branch);
            return Ok(current_dir.join(&branch));
        }

        // Check for potential issues (unless forced)
        if !force {
            // naive check for collision
            if current_dir.join(".bare").exists() {
                return Err(anyhow::anyhow!(
                    "Target '.bare' directory already exists. Use --force to overwrite."
                ));
            }
            if current_dir.join(&branch).exists() {
                return Err(anyhow::anyhow!(
                    "Target worktree directory '{}' already exists. Use --force to overwrite.",
                    branch
                ));
            }
        }

        debug!("Starting in-place migration for branch: {}", branch);

        // 3. Move .git to .bare
        let bare_dir = current_dir.join(".bare");
        std::fs::rename(&git_dir, &bare_dir).context("Failed to move .git to .bare")?;

        // 4. Configure .bare
        Self::run_git(&[
            "-C",
            &bare_dir.to_string_lossy(),
            "config",
            "--bool",
            "core.bare",
            "true",
        ])
        .context("Failed to set core.bare to true")?;

        // 5. Create Worktree (No Checkout)
        // We use --no-checkout so we don't overwrite existing files in the root yet,
        // and we create the folder where we will move everything.
        let worktree_dir = current_dir.join(&branch);
        Self::run_git(&[
            "-C",
            &bare_dir.to_string_lossy(),
            "worktree",
            "add",
            "--no-checkout",
            &worktree_dir.to_string_lossy(),
            &branch,
        ])
        .context("Failed to create worktree directory")?;

        // 6. Move Files
        // Identify files to move: everything except .bare and the new worktree dir
        let items = std::fs::read_dir(&current_dir).context("Failed to read current directory")?;
        for item in items {
            let entry = item?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str == ".bare"
                || file_name_str == branch.as_str()
                || file_name_str == ".git"
            {
                continue;
            }

            let dest = worktree_dir.join(&file_name);
            debug!("Moving {:?} to {:?}", path, dest);
            if let Err(e) = std::fs::rename(&path, &dest) {
                // If rename fails (e.g. across devices), try copy+delete
                debug!("Rename failed ({}), trying copy+delete...", e);
                // Note: This is risky for directories, mainly relying on rename working within same FS
                return Err(anyhow::anyhow!(
                    "Failed to move file {:?}: {}. Migration aborted mid-process.",
                    path,
                    e
                ));
            }
        }

        // 7. Reset Index in New Worktree
        // This makes the git index match the moved files
        Self::run_git(&["-C", &worktree_dir.to_string_lossy(), "reset", "HEAD"])
            .context("Failed to reset index in new worktree")?;

        // 8. Create .git file in root pointing to .bare (acting as main entry point?)
        // Actually, in Bare Hub, the root has NO .git file usually, or it has a .git dir if it was a repo.
        // But here we want the root to be just the holder.
        // The user might want a .git file in the root to make editors happy if they open the root?
        // No, the root is not a worktree. It's the hub.

        Ok(worktree_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_parse_git_history_normal() {
        let output = "* \x00abc1234\x00John Doe\x002023-01-01\x00Fix bug\n* \x00def4567\x00Jane Smith\x002023-01-02\x00Add feature";
        let commits = GitProjectRepository::parse_git_history(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].author, "John Doe");
        assert_eq!(commits[0].date, "2023-01-01");
        assert_eq!(commits[0].message, "Fix bug");
        assert_eq!(commits[1].hash, "def4567");
        assert_eq!(commits[1].author, "Jane Smith");
        assert_eq!(commits[1].date, "2023-01-02");
        assert_eq!(commits[1].message, "Add feature");
    }

    #[test]
    fn test_parse_git_history_single() {
        let output = "* \x00abc1234\x00John Doe\x002023-01-01\x00Fix bug";
        let commits = GitProjectRepository::parse_git_history(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc1234");
    }

    #[test]
    fn test_parse_git_history_empty() {
        let output = "";
        let commits = GitProjectRepository::parse_git_history(output);
        assert!(commits.is_empty());
    }

    #[test]
    fn test_parse_git_history_malformed() {
        let output = "* \x00abc1234\x00John Doe\x002023-01-01\n* \x00def4567\x00Jane Smith\x002023-01-02\x00Add feature\x00Extra";
        let commits = GitProjectRepository::parse_git_history(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "def4567");
        assert_eq!(commits[0].message, "Add feature\x00Extra");
    }

    #[test]
    fn test_parse_stash_list() {
        let output = "stash@{0}|WIP on main: abc1234 Initial commit|main\nstash@{1}|WIP on dev: def5678 Add feature|dev";
        let stashes = GitProjectRepository::parse_stash_list(output);

        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].index, 0);
        assert_eq!(stashes[0].message, "WIP on main: abc1234 Initial commit");
        assert_eq!(stashes[0].branch, "main");
        assert_eq!(stashes[1].index, 1);
        assert_eq!(stashes[1].message, "WIP on dev: def5678 Add feature");
        assert_eq!(stashes[1].branch, "dev");
    }

    #[test]
    fn test_parse_stash_list_empty() {
        let output = "";
        let stashes = GitProjectRepository::parse_stash_list(output);
        assert!(stashes.is_empty());
    }

    #[test]
    fn test_parse_stash_list_malformed() {
        let output = "stash@{0}|incomplete\nstash@{x}|bad index|branch";
        let stashes = GitProjectRepository::parse_stash_list(output);
        assert!(stashes.is_empty());
    }

    #[test]
    fn test_parse_git_history_with_pipes() {
        let output = "* \x00abc1234\x00John Doe\x002023-01-01\x00Message with | pipe";
        let commits = GitProjectRepository::parse_git_history(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].message, "Message with | pipe");
    }

    #[test]
    fn test_detect_context() {
        // Setup a temporary directory
        let temp_dir = std::env::temp_dir().join(format!("worktrees_test_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let repo = GitProjectRepository;

        // Test Standard
        assert_eq!(
            repo.detect_context(&temp_dir),
            crate::domain::repository::ProjectContext::Standard
        );

        // Test KMP/Android
        std::fs::write(temp_dir.join("build.gradle.kts"), "").unwrap();
        assert_eq!(
            repo.detect_context(&temp_dir),
            crate::domain::repository::ProjectContext::KmpAndroid
        );

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    #[serial_test::serial]
    fn test_editor_config_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        // Setup temp dir
        let temp_dir = std::env::temp_dir().join(format!(
            "worktrees_test_editor_perms_{}",
            std::process::id()
        ));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();

        // Backup existing HOME/USERPROFILE/XDG_CONFIG_HOME
        // Note: Environment variables are process-wide. This test should run in isolation or serially.
        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();

        unsafe {
            std::env::set_var("HOME", &temp_dir);
            std::env::set_var("XDG_CONFIG_HOME", &temp_dir);
        }

        let repo = GitProjectRepository;
        repo.set_preferred_editor("vim").unwrap();

        // Check file existence
        let path1 = temp_dir.join("worktrees").join("editor");
        let path2 = temp_dir.join(".worktrees.editor");

        let actual_path = if path1.exists() { path1 } else { path2 };
        assert!(actual_path.exists(), "Editor config file should be created");

        // Verify permissions
        let metadata = std::fs::metadata(&actual_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;

        // This assertion is expected to FAIL before the fix
        assert_eq!(
            mode, 0o600,
            "Editor config file permissions should be 600, but were {mode:o}"
        );

        // Restore env
        unsafe {
            if let Some(h) = old_home {
                std::env::set_var("HOME", h);
            } else {
                std::env::remove_var("HOME");
            }
            if let Some(x) = old_xdg {
                std::env::set_var("XDG_CONFIG_HOME", x);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_parse_branches() {
        let output = "main\norigin/HEAD\norigin/main\norigin/feature-branch\norigin/other-branch";
        let branches = GitProjectRepository::parse_branches(output);

        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0], "feature-branch");
        assert_eq!(branches[1], "main"); // deduplicated origin/main
        assert_eq!(branches[2], "other-branch");
        // ensure origin/HEAD is ignored
    }

    #[test]
    fn test_load_metadata() {
        use crate::domain::repository::WorktreeMetadata;

        // Setup temp dir
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_test_metadata_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();

        let metadata_path = temp_dir.join(".worktree.json");
        let mut metadata_map = std::collections::HashMap::new();
        metadata_map.insert(
            "dev".to_string(),
            WorktreeMetadata {
                created_at: Some("2023-10-27".to_string()),
                purpose: Some("Feature: Login UI".to_string()),
                description: Some("Development branch".to_string()),
                color: Some("#FF0000".to_string()),
                icon: Some("🚀".to_string()),
            },
        );

        let content = serde_json::to_string(&metadata_map).unwrap();
        std::fs::write(&metadata_path, content).unwrap();

        // Verify serialization/deserialization logic
        // We can't strictly test GitProjectRepository::load_metadata without mocking file system or running in the specific dir
        // But we can verify our serde logic is correct.

        let loaded: std::collections::HashMap<String, WorktreeMetadata> =
            serde_json::from_str(&std::fs::read_to_string(&metadata_path).unwrap()).unwrap();
        assert_eq!(
            loaded.get("dev").unwrap().description,
            Some("Development branch".to_string())
        );
        assert_eq!(loaded.get("dev").unwrap().icon, Some("🚀".to_string()));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn test_api_key_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        // Setup temp dir
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_test_perms_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();

        // Backup existing HOME/USERPROFILE/XDG_CONFIG_HOME
        // Note: Environment variables are process-wide. This test should run in isolation or serially.
        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();

        unsafe {
            std::env::set_var("HOME", &temp_dir);
            std::env::set_var("XDG_CONFIG_HOME", &temp_dir);
        }

        let repo = GitProjectRepository;
        repo.set_api_key("secret_key").unwrap();

        // Check file existence and permissions
        // set_api_key tries .worktrees.gemini_key in HOME or XDG_CONFIG_HOME/worktrees/gemini_key
        // Since we set both to temp_dir, it likely hits XDG first if dirs::config_dir uses it,
        // or falls back to HOME/.worktrees.gemini_key.

        let path1 = temp_dir.join("worktrees").join("gemini_key");
        let path2 = temp_dir.join(".worktrees.gemini_key");

        let actual_path = if path1.exists() { path1 } else { path2 };
        assert!(actual_path.exists(), "API key file should be created");

        // Verify initial creation is secure
        let metadata = std::fs::metadata(&actual_path).unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "Initial creation: API key file permissions should be 600, but were {mode:o}"
        );

        // Test existing file scenario: Sabotage permissions to 644
        let mut perms = metadata.permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&actual_path, perms).unwrap();

        // Verify sabotage worked
        let mode_sabotaged = std::fs::metadata(&actual_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode_sabotaged, 0o644,
            "Failed to sabotage permissions for test"
        );

        // Run set_api_key again - should fix permissions
        repo.set_api_key("new_secret_key").unwrap();

        // Verify fix
        let mode_fixed = std::fs::metadata(&actual_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode_fixed, 0o600,
            "Fixed API key file permissions should be 600, but were {mode_fixed:o}"
        );

        // Restore env
        unsafe {
            if let Some(h) = old_home {
                std::env::set_var("HOME", h);
            } else {
                std::env::remove_var("HOME");
            }
            if let Some(x) = old_xdg {
                std::env::set_var("XDG_CONFIG_HOME", x);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_convert_to_bare() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_convert_test_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();
        let repo_dir = temp_dir.join("my-app");
        std::fs::create_dir_all(&repo_dir).unwrap();

        // 1. Setup a standard repo
        let git_cmd = std::env::var("WORKTREES_GIT_PATH").unwrap_or_else(|_| "git".to_string());
        Command::new(&git_cmd)
            .args(["-C", &repo_dir.to_string_lossy(), "init"])
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args([
                "-C",
                &repo_dir.to_string_lossy(),
                "config",
                "user.email",
                "test@example.com",
            ])
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args([
                "-C",
                &repo_dir.to_string_lossy(),
                "config",
                "user.name",
                "Test User",
            ])
            .output()
            .unwrap();

        std::fs::write(repo_dir.join("file.txt"), "hello").unwrap();
        Command::new(&git_cmd)
            .args(["-C", &repo_dir.to_string_lossy(), "add", "file.txt"])
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["-C", &repo_dir.to_string_lossy(), "commit", "-m", "init"])
            .output()
            .unwrap();

        // Ensure the branch is named 'main'
        Command::new(&git_cmd)
            .args(["-C", &repo_dir.to_string_lossy(), "branch", "-m", "main"])
            .output()
            .unwrap();

        // 2. Perform conversion
        let repo = GitProjectRepository;
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo_dir).unwrap();

        let res = repo.convert_to_bare(Some("my-app-hub"), Some("main"));

        // Restore CWD regardless of success
        std::env::set_current_dir(original_cwd).unwrap();

        let hub_path = res.expect("Conversion failed");
        assert!(hub_path.exists());
        assert!(hub_path.ends_with("my-app-hub"));
        assert!(hub_path.join(".bare").exists());
        assert!(hub_path.join("main").exists());
        assert!(hub_path.join("main").join("file.txt").exists());
        assert!(!repo_dir.join(".git").exists());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_watch_reactivity() {
        let _lock = CWD_MUTEX.lock().unwrap();
        // Setup temp dir
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_watch_test_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let git_cmd = std::env::var("WORKTREES_GIT_PATH").unwrap_or_else(|_| "git".to_string());
        // Init bare repo structure manually to fake being a project root
        let _ = Command::new(&git_cmd)
            .args(["init", "--bare", ".bare"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();
        std::fs::write(temp_dir.join(".git"), "gitdir: ./.bare\n").unwrap();

        // We need to run the test inside temp_dir to simulate repo context
        let original_dir = std::env::current_dir().unwrap();
        if let Err(e) = std::env::set_current_dir(&temp_dir) {
            println!("Failed to set current dir: {e}");
            return;
        }

        let repo = GitProjectRepository;
        let rx_res = repo.watch();

        if let Ok(rx) = rx_res {
            // Modify a file
            std::thread::sleep(std::time::Duration::from_millis(1000));
            std::fs::write(temp_dir.join(".bare/config"), "[core]\n\tactive = true").unwrap();

            // Expect event
            let event = rx.recv_timeout(std::time::Duration::from_secs(5));
            assert!(event.is_ok(), "Should receive event");
            if let Ok(e) = event {
                assert_eq!(
                    e,
                    crate::domain::repository::RepositoryEvent::RescanRequired
                );
            }
        } else {
            println!("Watch setup failed: {:?}", rx_res.err());
        }

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_clean_artifacts() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_clean_test_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let src_dir = temp_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let git_cmd = std::env::var("WORKTREES_GIT_PATH").unwrap_or_else(|_| "git".to_string());

        // 1. Setup a standard repo
        Command::new(&git_cmd)
            .args(["init", "-b", "main"])
            .current_dir(&src_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&src_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["config", "user.name", "Test User"])
            .current_dir(&src_dir)
            .output()
            .unwrap();
        std::fs::write(src_dir.join("a"), "a").unwrap();
        Command::new(&git_cmd)
            .args(["add", "."])
            .current_dir(&src_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["commit", "-m", "init"])
            .current_dir(&src_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["branch", "dev"])
            .current_dir(&src_dir)
            .output()
            .unwrap();

        // 2. Convert to bare hub
        let repo = GitProjectRepository;
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&src_dir).unwrap();
        let hub_dir = repo.convert_to_bare(Some("my-hub"), Some("main")).unwrap();

        // 3. Add dev worktree
        Command::new(&git_cmd)
            .args(["worktree", "add", "../dev", "dev"])
            .current_dir(hub_dir.join("main"))
            .output()
            .unwrap();

        // 4. Create artifacts
        std::fs::create_dir_all(hub_dir.join("dev").join("target")).unwrap();
        std::fs::create_dir_all(hub_dir.join("main").join("target")).unwrap();

        std::env::set_current_dir(hub_dir.join("main")).unwrap();

        // 5. Run clean artifacts (dry-run)
        let cleaned = repo.clean_worktrees(true, true).unwrap();
        assert!(cleaned.iter().any(|s| s.contains("dev/target")));
        assert!(!cleaned.iter().any(|s| s.contains("main/target"))); // Should skip current

        // 6. Run clean artifacts (real)
        repo.clean_worktrees(false, true).unwrap();
        assert!(!hub_dir.join("dev").join("target").exists());
        assert!(hub_dir.join("main").join("target").exists()); // Should stay

        // Cleanup
        std::env::set_current_dir(original_cwd).unwrap();
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_migrate_in_place() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_migrate_test_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let git_cmd = std::env::var("WORKTREES_GIT_PATH").unwrap_or_else(|_| "git".to_string());

        // 1. Init Standard Repo
        Command::new(&git_cmd)
            .args(["init", "-b", "main"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["config", "user.name", "Test User"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();

        // 2. Create Content
        std::fs::write(temp_dir.join("committed.txt"), "committed").unwrap();
        Command::new(&git_cmd)
            .args(["add", "."])
            .current_dir(&temp_dir)
            .output()
            .unwrap();
        Command::new(&git_cmd)
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();

        // 3. Create Dirty State
        std::fs::write(temp_dir.join("committed.txt"), "modified").unwrap(); // Modified
        std::fs::write(temp_dir.join("untracked.txt"), "untracked").unwrap(); // Untracked

        // 4. Run Migration
        let repo = GitProjectRepository;
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let res = repo.migrate_to_bare(false, false);

        std::env::set_current_dir(original_cwd).unwrap();

        let worktree_path = res.expect("Migration failed");

        // 5. Verification
        assert!(temp_dir.join(".bare").exists(), ".bare not found");
        assert!(
            !temp_dir.join(".git").exists(),
            "Original .git still exists"
        );
        assert!(worktree_path.exists(), "Worktree dir not found");
        assert!(worktree_path.join("committed.txt").exists());
        assert!(worktree_path.join("untracked.txt").exists());

        // Read content to ensure file move was correct
        let content = std::fs::read_to_string(worktree_path.join("committed.txt")).unwrap();
        assert_eq!(content, "modified");

        // Verify Git Status inside worktree
        let status_out = Command::new(&git_cmd)
            .args(["status", "--porcelain"])
            .current_dir(&worktree_path)
            .output()
            .unwrap();
        let status = String::from_utf8_lossy(&status_out.stdout);

        // Should show 'M committed.txt' and '?? untracked.txt'
        assert!(status.contains("M committed.txt"));
        assert!(status.contains("?? untracked.txt"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_migrate_in_place_dry_run() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!(
            "worktrees_migrate_dry_run_test_{}",
            std::process::id()
        ));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();
        let git_cmd = std::env::var("WORKTREES_GIT_PATH").unwrap_or_else(|_| "git".to_string());

        // 1. Init Standard Repo
        Command::new(&git_cmd)
            .args(["init", "-b", "main"])
            .current_dir(&temp_dir)
            .output()
            .unwrap();

        // 2. Run Migration (Dry Run)
        let repo = GitProjectRepository;
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let res = repo.migrate_to_bare(false, true);

        std::env::set_current_dir(original_cwd).unwrap();

        let path = res.expect("Dry run failed");

        // 3. Verification
        // Nothing should have changed
        assert!(temp_dir.join(".git").exists(), "Original .git should exist");
        assert!(!temp_dir.join(".bare").exists(), ".bare should NOT exist");

        let expected = temp_dir.join("main");
        // Canonicalize if possible, or handle Mac /var vs /private/var
        let path_str = path.to_string_lossy();
        let expected_str = expected.to_string_lossy();

        if cfg!(target_os = "macos") {
            assert!(path_str.ends_with(expected_str.trim_start_matches("/private")));
        } else {
            assert_eq!(path, expected);
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_parse_worktree_entry_normal() {
        let block =
            "worktree /path/to/worktree\nHEAD abc123456789\nbranch refs/heads/feature-branch";
        let wt = GitProjectRepository::parse_worktree_entry(block);

        assert_eq!(wt.path, "/path/to/worktree");
        assert_eq!(wt.commit, "abc1234");
        assert_eq!(wt.branch, "feature-branch");
        assert!(!wt.is_bare);
        assert!(!wt.is_detached);
    }

    #[test]
    fn test_parse_worktree_entry_bare() {
        let block = "worktree /path/to/bare\nbare";
        let wt = GitProjectRepository::parse_worktree_entry(block);

        assert_eq!(wt.path, "/path/to/bare");
        assert!(wt.is_bare);
        assert!(wt.branch.is_empty());
        assert!(!wt.is_detached);
    }

    #[test]
    fn test_parse_worktree_entry_detached() {
        let block = "worktree /path/to/detached\nHEAD def5678\ndetached";
        let wt = GitProjectRepository::parse_worktree_entry(block);

        assert_eq!(wt.path, "/path/to/detached");
        assert_eq!(wt.commit, "def5678");
        assert!(wt.is_detached);
        assert!(!wt.is_bare);
    }

    #[test]
    fn test_parse_worktree_entry_complex_branch() {
        let block = "worktree /path/to/nested\nHEAD 1234567\nbranch refs/heads/group/feature";
        let wt = GitProjectRepository::parse_worktree_entry(block);

        assert_eq!(wt.branch, "group/feature");
    }

    #[test]
    fn test_sync_configs_path_traversal() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("worktrees_traversal_test_{}", std::process::id()));
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Layout:
        // temp_dir/
        //   outside.txt (content: "secret")
        //   repo/
        //     .worktrees.sync (content: "copy ../outside.txt")
        //     wt1/ (directory)

        let outside_file = temp_dir.join("outside.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        let repo_dir = temp_dir.join("repo");
        std::fs::create_dir(&repo_dir).unwrap();

        let wt1_dir = repo_dir.join("wt1");
        std::fs::create_dir(&wt1_dir).unwrap();

        let sync_file = repo_dir.join(".worktrees.sync");
        std::fs::write(&sync_file, "copy ../outside.txt").unwrap();

        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo_dir).unwrap();

        let repo = GitProjectRepository;
        // We pass "wt1" as the path.
        // Destination becomes "wt1/../outside.txt" -> "outside.txt" (in repo_dir)
        let res = repo.sync_configs("wt1");

        std::env::set_current_dir(original_cwd).unwrap();

        // If vulnerable, repo_dir/outside.txt should exist and contain "secret"
        let leaked_file = repo_dir.join("outside.txt");

        // Verify before cleanup
        assert!(res.is_ok(), "sync_configs should succeed");

        let exists = leaked_file.exists();

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();

        // The fix should PREVENT this file from being created.
        assert!(
            !exists,
            "Vulnerability fixed: Path traversal prevented writing outside worktree"
        );
    }

    #[test]
    fn test_parse_status_output_clean() {
        let output = "";
        let status = GitProjectRepository::parse_status_output(output);
        assert!(status.staged.is_empty());
        assert!(status.unstaged.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn test_parse_status_output_staged() {
        let output = "M  file1.txt\nA  file2.txt\nD  file3.txt\nR  old -> new\nC  copy.txt";
        let status = GitProjectRepository::parse_status_output(output);

        assert_eq!(status.staged.len(), 5);
        assert!(status.unstaged.is_empty());
        assert!(status.untracked.is_empty());

        let staged_map: std::collections::HashMap<_, _> = status.staged.into_iter().collect();
        assert_eq!(staged_map.get("file1.txt"), Some(&"M ".to_string()));
        assert_eq!(staged_map.get("file2.txt"), Some(&"A ".to_string()));
        assert_eq!(staged_map.get("file3.txt"), Some(&"D ".to_string()));
        assert_eq!(staged_map.get("old -> new"), Some(&"R ".to_string()));
        assert_eq!(staged_map.get("copy.txt"), Some(&"C ".to_string()));
    }

    #[test]
    fn test_parse_status_output_unstaged() {
        let output = " M file1.txt\n D file2.txt";
        let status = GitProjectRepository::parse_status_output(output);

        assert!(status.staged.is_empty());
        assert_eq!(status.unstaged.len(), 2);
        assert!(status.untracked.is_empty());

        let unstaged_map: std::collections::HashMap<_, _> = status.unstaged.into_iter().collect();
        assert_eq!(unstaged_map.get("file1.txt"), Some(&" M".to_string()));
        assert_eq!(unstaged_map.get("file2.txt"), Some(&" D".to_string()));
    }

    #[test]
    fn test_parse_status_output_untracked() {
        let output = "?? new_file.txt\n?? another_file.txt";
        let status = GitProjectRepository::parse_status_output(output);

        assert!(status.staged.is_empty());
        assert!(status.unstaged.is_empty());
        assert_eq!(status.untracked.len(), 2);

        assert!(status.untracked.contains(&"new_file.txt".to_string()));
        assert!(status.untracked.contains(&"another_file.txt".to_string()));
    }

    #[test]
    fn test_parse_status_output_mixed() {
        let output = "MM file1.txt\nMD file2.txt";
        let status = GitProjectRepository::parse_status_output(output);

        assert_eq!(status.staged.len(), 2);
        assert_eq!(status.unstaged.len(), 2);

        // MM -> Staged M, Unstaged M
        // MD -> Staged M, Unstaged D

        let staged_map: std::collections::HashMap<_, _> = status.staged.iter().cloned().collect();
        let unstaged_map: std::collections::HashMap<_, _> =
            status.unstaged.iter().cloned().collect();

        assert_eq!(staged_map.get("file1.txt"), Some(&"M ".to_string()));
        assert_eq!(unstaged_map.get("file1.txt"), Some(&" M".to_string()));

        assert_eq!(staged_map.get("file2.txt"), Some(&"M ".to_string()));
        assert_eq!(unstaged_map.get("file2.txt"), Some(&" D".to_string()));
    }

    #[test]
    fn test_parse_status_output_ignored_lines() {
        // Lines too short or irrelevant
        let output = " \n??\nIgnore";
        let status = GitProjectRepository::parse_status_output(output);
        assert!(status.staged.is_empty());
        assert!(status.unstaged.is_empty());
        assert!(status.untracked.is_empty());
    }
}
