use crate::domain::repository::{ProjectContext, ProjectRepository, Worktree};
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
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
            return Err(anyhow::anyhow!("Git error: {}. HELP: Check your network connection or repository permissions.", err));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
                let _ = std::fs::copy(local_props, dest).context("Failed to copy local.properties.");
            }

            // 2. Sync and Optimize gradle.properties
            let gradle_props = Path::new("gradle.properties");
            let dest_gradle = Path::new(path).join("gradle.properties");
            
            if gradle_props.exists() {
                let _ = std::fs::copy(gradle_props, &dest_gradle).context("Failed to copy gradle.properties.");
            }

            // 3. Ensure Build Caching is enabled for Worktree performance
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dest_gradle) 
            {
                // Check if already contains caching flag (simple check)
                if std::fs::read_to_string(&dest_gradle).map(|c| !c.contains("org.gradle.caching")).unwrap_or(false) {
                    let _ = writeln!(file, "\n# Optimized for Worktrees\norg.gradle.caching=true");
                }
            }
        }
    }
}

impl ProjectRepository for GitProjectRepository {
    fn init_bare_repo(&self, url: &str, project_name: &str) -> Result<()> {
        if Path::new(project_name).exists() {
            return Err(anyhow::anyhow!("Directory '{}' already exists. HELP: Choose a different name or remove the existing directory.", project_name));
        }

        std::fs::create_dir(project_name).context("Failed to create project directory. HELP: Ensure you have write permissions in the current folder.")?;
        std::env::set_current_dir(project_name).context("Failed to enter project directory.")?;

        Self::run_git(&["clone", "--bare", "--", url, ".bare"]).context("Failed to clone bare repository. HELP: Verify the repository URL and your SSH/HTTP credentials.")?;
        std::fs::write(".git", "gitdir: ./.bare\n").context("Failed to write .git redirection file.")?;
        Self::run_git(&[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ])?;
        Self::run_git(&["fetch", "origin"])?;

        Ok(())
    }

    fn add_worktree(&self, path: &str, branch: &str) -> Result<()> {
        Self::run_git(&["worktree", "add", "--", path, branch]).context(format!("Failed to add worktree '{}'. HELP: Ensure the branch '{}' exists on origin.", path, branch))?;
        self.handle_context_files(path);
        Ok(())
    }

    fn add_new_worktree(&self, path: &str, branch: &str, base: &str) -> Result<()> {
        Self::run_git(&["worktree", "add", "-b", branch, "--", path, base]).context(format!("Failed to create new worktree '{}' from '{}'. HELP: Ensure the base branch '{}' is valid.", path, base, base))?;
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
                debug!(?source, "Source file defined in manifest does not exist, skipping.");
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
                        std::os::unix::fs::symlink(source, &destination)
                            .with_context(|| format!("Failed to symlink {:?} to {:?}", source, destination))?;
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
                        std::fs::copy(source, &destination)
                            .with_context(|| format!("Failed to copy {:?} to {:?}", source, destination))?;
                    }
                }
                _ => debug!(?action, "Unknown action in manifest, skipping."),
            }
        }

        Ok(())
    }

    fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let output = Self::run_git(&["worktree", "list", "--porcelain"])?;
        
        Ok(output
            .split("\n\n")
            .filter(|block| !block.is_empty())
            .map(|block| {
                block.lines().fold(
                    Worktree {
                        path: String::new(),
                        commit: String::new(),
                        branch: String::new(),
                        is_bare: false,
                        is_detached: false,
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
            })
            .collect())
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
        let path = Path::new(".worktrees.editor");
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(Some(content.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    fn set_preferred_editor(&self, editor: &str) -> Result<()> {
        std::fs::write(".worktrees.editor", editor)?;
        Ok(())
    }
}
