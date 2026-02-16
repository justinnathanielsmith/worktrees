mod app;
mod cli;
mod domain;
mod infrastructure;
mod ui;

use app::intent::Intent;
use app::model::AppState;
use app::view::View;
use clap::Parser;
use cli::{Cli, Commands};
use domain::repository::{ProjectRepository, Worktree};
use indicatif::{ProgressBar, ProgressStyle};
use infrastructure::git_repo::GitProjectRepository;
use miette::{IntoDiagnostic, Result};
use owo_colors::{OwoColorize, Stream::Stdout};
use ratatui::widgets::TableState;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tracing::{error, info, instrument};

fn setup_logging(json_mode: bool) {
    if json_mode {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .json()
            .flatten_event(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_target(false)
            .compact()
            .init();
    }
}

fn get_project_name(url: &str, name: Option<String>) -> String {
    name.unwrap_or_else(|| {
        Path::new(url.trim_end_matches('/'))
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("project")
            .to_string()
    })
}

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        let sigterm_res = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
        let sigint_res = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt());

        if let (Ok(mut sigterm), Ok(mut sigint)) = (sigterm_res, sigint_res) {
            tokio::select! {
                _ = sigterm.recv() => info!("Received SIGTERM, shutting down gracefully..."),
                _ = sigint.recv() => info!("Received SIGINT, shutting down gracefully..."),
            }
        } else {
            error!("Failed to register signal handlers");
        }
    }
    #[cfg(not(unix))]
    {
        if let Ok(_) = tokio::signal::ctrl_c().await {
            info!("Received Ctrl-C, shutting down gracefully...");
        } else {
            error!("Failed to listen for ctrl-c");
        }
    }
}

struct Reducer<R: ProjectRepository> {
    repo: R,
    json_mode: bool,
}

impl<R: ProjectRepository> Reducer<R> {
    fn new(repo: R, json_mode: bool) -> Self {
        Self { repo, json_mode }
    }

    #[instrument(skip(self))]
    fn handle(&self, intent: Intent) -> Result<()> {
        info!(?intent, "Handling intent");
        match intent {
            Intent::Initialize { url, name } => {
                let project_name = get_project_name(&url, name);
                if !self.json_mode {
                    View::render(AppState::Initializing {
                        project_name: project_name.clone(),
                    });
                }

                let pb = if !self.json_mode {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                            .template("{spinner:.cyan} {msg}")
                            .into_diagnostic()?,
                    );
                    pb.set_message("Initializing bare repository...");
                    pb.enable_steady_tick(Duration::from_millis(100));
                    Some(pb)
                } else {
                    None
                };

                match self.repo.init_bare_repo(&url, &project_name) {
                    Ok(_) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        info!(%project_name, "Repository initialized successfully");
                        if self.json_mode {
                            View::render_json(&serde_json::json!({
                                "status": "success",
                                "project": project_name,
                                "path": format!("{}/.bare", project_name)
                            }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            View::render(AppState::Initialized { project_name });
                        }
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        error!(error = %e, "Failed to initialize repository");
                        if self.json_mode {
                            View::render_json(&serde_json::json!({
                                "status": "error",
                                "message": e.to_string()
                            }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            View::render(AppState::Error(e.to_string()));
                        }
                    }
                }
            }
            Intent::AddWorktree { intent, branch } => {
                let branch_name = branch.unwrap_or_else(|| intent.clone());
                if !self.json_mode {
                    View::render(AppState::AddingWorktree {
                        intent: intent.clone(),
                        branch: branch_name.clone(),
                    });
                }

                let pb = if !self.json_mode {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                            .template("{spinner:.magenta} {msg}")
                            .into_diagnostic()?,
                    );
                    pb.set_message(format!("Adding worktree: {}...", intent));
                    pb.enable_steady_tick(Duration::from_millis(100));
                    Some(pb)
                } else {
                    None
                };

                match self.repo.add_worktree(&intent, &branch_name) {
                    Ok(_) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        info!(%intent, %branch_name, "Worktree added successfully");
                        if self.json_mode {
                            View::render_json(&serde_json::json!({
                                "status": "success",
                                "intent": intent,
                                "branch": branch_name
                            }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            View::render(AppState::WorktreeAdded { intent });
                        }
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        error!(error = %e, %intent, "Failed to add worktree");
                        if self.json_mode {
                            View::render_json(&serde_json::json!({
                                "status": "error",
                                "message": e.to_string()
                            }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            View::render(AppState::Error(e.to_string()));
                        }
                    }
                }
            }
            Intent::RemoveWorktree { intent } => {
                if !self.json_mode {
                    View::render(AppState::RemovingWorktree {
                        intent: intent.clone(),
                    });
                }
                match self.repo.remove_worktree(&intent, false) {
                    Ok(_) => {
                        info!(%intent, "Worktree removed successfully");
                        if self.json_mode {
                            View::render_json(
                                &serde_json::json!({ "status": "success", "intent": intent }),
                            )
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            View::render(AppState::WorktreeRemoved);
                        }
                    }
                    Err(e) => {
                        error!(error = %e, %intent, "Failed to remove worktree");
                        if self.json_mode {
                            View::render_json(
                                &serde_json::json!({ "status": "error", "message": e.to_string() }),
                            )
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            View::render(AppState::Error(e.to_string()));
                        }
                    }
                }
            }
            Intent::ListWorktrees => {
                let worktrees = self
                    .repo
                    .list_worktrees()
                    .map_err(|e| miette::miette!("{e:?}"))?;
                info!(count = worktrees.len(), "Worktrees listed successfully");
                if self.json_mode {
                    View::render_json(&worktrees).map_err(|e| miette::miette!("{e:?}"))?;
                } else {
                    View::render_banner();
                    if worktrees.is_empty() {
                        View::render(AppState::Welcome);
                    }
                    View::render_listing_table(&worktrees);
                    println!(
                        "\n{}",
                        "Tip: Run with 'worktrees list' (no args) for interactive TUI"
                            .if_supports_color(Stdout, |t| t.dimmed())
                    );
                }
            }
            Intent::SetupDefaults => {
                if !self.json_mode {
                    View::render(AppState::SettingUpDefaults);
                }

                let mut results = Vec::new();

                info!("Setting up default worktrees (main, dev)");
                let main_res = match self.repo.add_worktree("main", "main") {
                    Ok(_) => {
                        if !self.json_mode {
                            println!("   Main: {}", "READY".green().bold());
                        }
                        serde_json::json!({ "name": "main", "status": "ready" })
                    }
                    Err(_) => {
                        if !self.json_mode {
                            println!("   Main: {}", "SKIPPED".dimmed());
                        }
                        serde_json::json!({ "name": "main", "status": "skipped" })
                    }
                };
                results.push(main_res);

                let dev_res = match self.repo.add_worktree("dev", "dev") {
                    Ok(_) => {
                        if !self.json_mode {
                            println!("   Dev:  {}", "READY".green().bold());
                        }
                        serde_json::json!({ "name": "dev", "status": "ready" })
                    }
                    Err(_) => match self.repo.add_new_worktree("dev", "dev", "main") {
                        Ok(_) => {
                            if !self.json_mode {
                                println!("   Dev:  {}", "READY (Created from main)".green().bold());
                            }
                            serde_json::json!({ "name": "dev", "status": "ready", "created_from": "main" })
                        }
                        Err(_) => {
                            if !self.json_mode {
                                println!("   Dev:  {}", "SKIPPED".dimmed());
                            }
                            serde_json::json!({ "name": "dev", "status": "skipped" })
                        }
                    },
                };
                results.push(dev_res);

                if self.json_mode {
                    View::render_json(&results).map_err(|e| miette::miette!("{e:?}"))?;
                } else {
                    View::render(AppState::SetupComplete);
                }
            }
            Intent::RunCommand {
                intent,
                branch,
                command,
            } => {
                let branch_name = branch.unwrap_or_else(|| intent.clone());

                if !self.json_mode {
                    println!(
                        "{} Creating temporary worktree '{}' tracking '{}'...",
                        "➜".cyan().bold(),
                        intent,
                        branch_name
                    );
                }

                // 1. Add worktree
                self.repo
                    .add_worktree(&intent, &branch_name)
                    .map_err(|e| miette::miette!("Failed to create temporary worktree: {}", e))?;

                if !self.json_mode {
                    println!(
                        "{} Executing command: {}",
                        "➜".cyan().bold(),
                        command.join(" ").bold()
                    );
                }

                // 2. Run command
                let status = Command::new(&command[0])
                    .args(&command[1..])
                    .current_dir(&intent)
                    .spawn()
                    .map_err(|e| miette::miette!("Failed to spawn command: {}", e))?
                    .wait()
                    .map_err(|e| miette::miette!("Failed to wait for command: {}", e))?;

                if !self.json_mode {
                    println!("{} Cleaning up...", "➜".cyan().bold());
                }

                // 3. Remove worktree (force to ensure cleanup)
                let _ = self.repo.remove_worktree(&intent, true);

                if !status.success() {
                    return Err(miette::miette!("Command failed with status: {}", status));
                }

                if !self.json_mode {
                    println!("{} Done.", "✔".green().bold());
                } else {
                    View::render_json(
                        &serde_json::json!({ "status": "success", "exit_code": status.code() }),
                    )
                    .map_err(|e| miette::miette!("{e:?}"))?;
                }
            }
            Intent::SyncConfigurations { intent } => {
                let worktrees = self
                    .repo
                    .list_worktrees()
                    .map_err(|e| miette::miette!("{e:?}"))?;
                let targets: Vec<Worktree> = if let Some(name) = intent {
                    worktrees
                        .into_iter()
                        .filter(|wt| wt.branch == name || wt.path.ends_with(&name))
                        .collect()
                } else {
                    worktrees.into_iter().filter(|wt| !wt.is_bare).collect()
                };

                if targets.is_empty() {
                    return Err(miette::miette!(
                        "No matching worktrees found to synchronize."
                    ));
                }

                for wt in targets {
                    if !self.json_mode {
                        println!(
                            "{} Synchronizing configuration for: {}",
                            "➜".cyan().bold(),
                            wt.branch.bold()
                        );
                    }
                    if let Err(e) = self.repo.sync_configs(&wt.path) {
                        error!(error = %e, path = %wt.path, "Configuration synchronization failed");
                        if !self.json_mode {
                            println!("   {} Error: {}", "❌".red(), e);
                        }
                    } else if !self.json_mode {
                        println!("   {} Synchronization complete.", "✔".green());
                    }
                }

                if self.json_mode {
                    View::render_json(&serde_json::json!({ "status": "success" }))
                        .map_err(|e| miette::miette!("{e:?}"))?;
                }
            }
            Intent::Config { key, show } => {
                if let Some(k) = key {
                    self.repo
                        .set_api_key(&k)
                        .map_err(|e| miette::miette!("Failed to set API key: {}", e))?;
                    if !self.json_mode {
                        println!("{} Gemini API key set successfully.", "✔".green().bold());
                    } else {
                        View::render_json(
                            &serde_json::json!({ "status": "success", "action": "set_key" }),
                        )
                        .map_err(|e| miette::miette!("{e:?}"))?;
                    }
                } else if show {
                    let k = self
                        .repo
                        .get_api_key()
                        .map_err(|e| miette::miette!("Failed to get API key: {}", e))?;
                    if !self.json_mode {
                        if let Some(val) = k {
                            println!("{} Current API key: {}", "➜".cyan().bold(), val);
                        } else {
                            println!("{} No API key found.", "⚠".yellow().bold());
                        }
                    } else {
                        View::render_json(&serde_json::json!({ "status": "success", "key": k }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.json);

    let repo = GitProjectRepository;
    let reducer = Reducer::new(repo, cli.json);

    let intent = match cli.command {
        Some(Commands::Init { url, name }) => Intent::Initialize { url, name },
        Some(Commands::Add { intent, branch }) => Intent::AddWorktree { intent, branch },
        Some(Commands::Remove { intent }) => Intent::RemoveWorktree { intent },
        Some(Commands::List) => Intent::ListWorktrees,
        Some(Commands::Setup) => Intent::SetupDefaults,
        Some(Commands::Run {
            intent,
            branch,
            command,
        }) => Intent::RunCommand {
            intent,
            branch,
            command,
        },
        Some(Commands::Sync { intent }) => Intent::SyncConfigurations { intent },
        Some(Commands::Config { action }) => match action {
            cli::ConfigAction::SetKey { key } => Intent::Config {
                key: Some(key),
                show: false,
            },
            cli::ConfigAction::GetKey => Intent::Config {
                key: None,
                show: true,
            },
        },
        None => {
            if cli.json {
                let worktrees = GitProjectRepository
                    .list_worktrees()
                    .map_err(|e| miette::miette!("{e:?}"))?;
                return View::render_json(&worktrees).map_err(|e| miette::miette!("{e:?}"));
            }
            // TUI Mode
            View::render_banner();
            let worktrees = GitProjectRepository
                .list_worktrees()
                .map_err(|e| miette::miette!("{e:?}"))?;
            let mut table_state = TableState::default();
            if !worktrees.is_empty() {
                table_state.select(Some(0));
            }
            let initial_state = AppState::ListingWorktrees {
                worktrees,
                table_state,
                refresh_needed: false,
            };
            return View::render_tui(&GitProjectRepository, initial_state)
                .map_err(|e| miette::miette!("{e:?}"));
        }
    };

    tokio::select! {
        res = async { reducer.handle(intent) } => {
            res?;
        }
        _ = wait_for_shutdown() => {}
    }

    if !cli.json {
        View::render_feedback_prompt();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use domain::repository::{ProjectRepository, Worktree};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct CallTracker {
        calls: Vec<String>,
    }

    struct MockRepo {
        tracker: Arc<Mutex<CallTracker>>,
    }

    impl MockRepo {
        fn new(tracker: Arc<Mutex<CallTracker>>) -> Self {
            Self { tracker }
        }
    }

    impl ProjectRepository for MockRepo {
        fn init_bare_repo(&self, url: &str, name: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("init:{}|{}", url, name));
            Ok(())
        }
        fn add_worktree(&self, intent: &str, branch: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("add:{}|{}", intent, branch));
            Ok(())
        }
        fn add_new_worktree(&self, intent: &str, branch: &str, base: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("add_new:{}|{}|{}", intent, branch, base));
            Ok(())
        }
        fn remove_worktree(&self, intent: &str, force: bool) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("remove:{}|force:{}", intent, force));
            Ok(())
        }
        fn list_worktrees(&self) -> anyhow::Result<Vec<Worktree>> {
            self.tracker.lock().unwrap().calls.push("list".to_string());
            Ok(vec![
                Worktree {
                    path: "main".to_string(),
                    commit: "1234567".to_string(),
                    branch: "main".to_string(),
                    is_bare: false,
                    is_detached: false,
                    status_summary: Some("clean".to_string()),
                },
                Worktree {
                    path: "dev".to_string(),
                    commit: "abcdef0".to_string(),
                    branch: "dev".to_string(),
                    is_bare: false,
                    is_detached: false,
                    status_summary: Some("~1".to_string()),
                },
            ])
        }
        fn sync_configs(&self, path: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("sync:{}", path));
            Ok(())
        }
        fn detect_context(&self) -> domain::repository::ProjectContext {
            domain::repository::ProjectContext::Standard
        }
        fn get_preferred_editor(&self) -> anyhow::Result<Option<String>> {
            Ok(None)
        }
        fn set_preferred_editor(&self, _editor: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn fetch(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_status(&self, _path: &str) -> anyhow::Result<domain::repository::GitStatus> {
            Ok(domain::repository::GitStatus {
                staged: vec![],
                unstaged: vec![],
                untracked: vec![],
            })
        }
        fn stage_all(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn unstage_all(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn stage_file(&self, _path: &str, _file: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn unstage_file(&self, _path: &str, _file: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn commit(&self, _path: &str, _message: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_history(
            &self,
            _path: &str,
            _limit: usize,
        ) -> anyhow::Result<Vec<domain::repository::GitCommit>> {
            Ok(vec![])
        }
        fn list_branches(&self) -> anyhow::Result<Vec<String>> {
            Ok(vec!["main".to_string(), "dev".to_string()])
        }
        fn switch_branch(&self, _path: &str, _branch: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_diff(&self, _path: &str) -> anyhow::Result<String> {
            Ok("diff".to_string())
        }
        fn generate_commit_message(&self, _diff: &str, _branch: &str) -> anyhow::Result<String> {
            Ok("feat: mock commit message".to_string())
        }
        fn get_api_key(&self) -> anyhow::Result<Option<String>> {
            Ok(Some("key".to_string()))
        }
        fn set_api_key(&self, _key: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_get_project_name() {
        assert_eq!(
            get_project_name("https://github.com/user/repo.git", None),
            "repo"
        );
        assert_eq!(
            get_project_name("git@github.com:user/my-project", None),
            "my-project"
        );
        assert_eq!(
            get_project_name(
                "https://github.com/user/repo.git",
                Some("custom".to_string())
            ),
            "custom"
        );
        assert_eq!(get_project_name("/path/to/local/repo", None), "repo");
    }

    #[test]
    fn test_reducer_handle_init() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false);

        reducer
            .handle(Intent::Initialize {
                url: "https://github.com/user/repo.git".to_string(),
                name: None,
            })
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        assert_eq!(
            tracker.lock().unwrap().calls,
            vec!["init:https://github.com/user/repo.git|repo"]
        );
        Ok(())
    }

    #[test]
    fn test_reducer_handle_add() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false);

        reducer
            .handle(Intent::AddWorktree {
                intent: "feat-x".to_string(),
                branch: Some("feature/x".to_string()),
            })
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        assert_eq!(tracker.lock().unwrap().calls, vec!["add:feat-x|feature/x"]);
        Ok(())
    }

    #[test]
    fn test_reducer_handle_setup() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false);

        reducer
            .handle(Intent::SetupDefaults)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"add:main|main".to_string()));
        Ok(())
    }

    #[test]
    fn test_reducer_handle_run() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false);

        // Create a dummy directory to satisfy .current_dir(&intent)
        let temp_dir = "temp-run-test";
        let _ = std::fs::create_dir(temp_dir);

        let res = reducer.handle(Intent::RunCommand {
            intent: temp_dir.to_string(),
            branch: Some("main".to_string()),
            command: vec!["echo".to_string(), "hello".to_string()],
        });

        // Cleanup the dummy directory
        let _ = std::fs::remove_dir(temp_dir);

        res.map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert_eq!(calls[0], format!("add:{}|main", temp_dir));
        assert_eq!(calls[1], format!("remove:{}|force:true", temp_dir));
        Ok(())
    }

    #[test]
    fn test_reducer_handle_sync() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false);

        reducer
            .handle(Intent::SyncConfigurations {
                intent: Some("main".to_string()),
            })
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"list".to_string()));
        assert!(calls.contains(&"sync:main".to_string()));
        Ok(())
    }

    #[test]
    fn test_cli_parsing() -> Result<()> {
        use clap::Parser;

        // Test init
        let cli = Cli::try_parse_from(["worktrees", "init", "url"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Init { url, name } => {
                assert_eq!(url, "url");
                assert_eq!(name, None);
            }
            _ => anyhow::bail!("Expected Init"),
        }

        // Test add
        let cli = Cli::try_parse_from(["worktrees", "add", "feat", "branch"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Add { intent, branch } => {
                assert_eq!(intent, "feat");
                assert_eq!(branch, Some("branch".to_string()));
            }
            _ => anyhow::bail!("Expected Add"),
        }

        // Test list
        let cli = Cli::try_parse_from(["worktrees", "list"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        assert!(matches!(
            cli.command
                .ok_or_else(|| anyhow::anyhow!("Missing command"))?,
            Commands::List
        ));
        Ok(())
    }
}
