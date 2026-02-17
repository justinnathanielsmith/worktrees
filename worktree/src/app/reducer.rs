use crate::app::intent::Intent;
use crate::app::model::AppState;
use crate::app::ports::{RatatuiView, ViewPort};
use crate::domain::repository::{ProjectRepository, Worktree};
use indicatif::{ProgressBar, ProgressStyle};
use miette::{IntoDiagnostic, Result};
use owo_colors::{OwoColorize, Stream::Stdout};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tracing::{error, info, instrument};

fn get_project_name(url: Option<&String>, name: Option<String>) -> String {
    name.unwrap_or_else(|| {
        url.and_then(|u| {
            Path::new(u.trim_end_matches('/'))
                .file_stem()
                .and_then(|s| s.to_str())
                .map(std::string::ToString::to_string)
        })
        .unwrap_or_else(|| "project".to_string())
    })
}

pub struct Reducer<R: ProjectRepository, V: ViewPort = RatatuiView> {
    repo: R,
    view: V,
    json_mode: bool,
    quiet_mode: bool,
}

impl<R: ProjectRepository + Clone + Send + Sync + 'static> Reducer<R, RatatuiView> {
    pub const fn new(repo: R, json_mode: bool, quiet_mode: bool) -> Self {
        Self {
            repo,
            view: RatatuiView,
            json_mode,
            quiet_mode,
        }
    }
}

impl<R: ProjectRepository + Clone + Send + Sync + 'static, V: ViewPort> Reducer<R, V> {
    pub fn new_with_view(repo: R, view: V, json_mode: bool, quiet_mode: bool) -> Self {
        Self {
            repo,
            view,
            json_mode,
            quiet_mode,
        }
    }

    #[instrument(skip(self))]
    pub async fn handle(&self, intent: Intent) -> Result<()> {
        info!(?intent, "Handling intent");
        let repo = self.repo.clone();
        let json_mode = self.json_mode;
        let quiet_mode = self.quiet_mode;

        // Clone intent for moving into closures/async blocks if needed
        // For now, we will keep the structure similar to main.rs but mark handle as async
        // and prepare for spawn_blocking in the next step.

        match intent {
            Intent::Initialize { url, name, warp } => {
                let project_name = get_project_name(url.as_ref(), name);
                if !json_mode && !quiet_mode {
                    self.view.render(AppState::Initializing {
                        project_name: project_name.clone(),
                    });
                }

                let pb = if !json_mode && !quiet_mode {
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

                let url_clone = url.clone();
                let project_name_clone = project_name.clone();
                let repo_clone = repo.clone();

                let res = tokio::task::spawn_blocking(move || {
                    repo_clone.init_bare_repo(url_clone.as_deref(), &project_name_clone)
                })
                .await
                .into_diagnostic()?;

                match res {
                    Ok(()) => {
                        if warp {
                            if crate::infrastructure::warp_integration::is_warp_terminal() {
                                if let Err(e) =
                                    crate::infrastructure::warp_integration::generate_warp_workflows(
                                        Path::new(&project_name),
                                    )
                                {
                                    error!(error = %e, "Failed to generate Warp workflows");
                                } else {
                                    info!("Warp workflows generated successfully");
                                }
                            } else {
                                info!("Skipping Warp workflow generation: Not running in Warp Terminal");
                                if !json_mode && !quiet_mode {
                                    println!(
                                        "{} Skipping Warp workflow generation: Not running in Warp Terminal",
                                        "⚠".yellow()
                                    );
                                }
                            }
                        }

                        if url.is_none() {
                            // Automatically create main worktree for fresh projects
                            if let Err(e) = repo.add_new_worktree("main", "main", "HEAD") {
                                error!(error = %e, "Failed to create default main worktree");
                            }
                        }
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        info!(%project_name, "Repository initialized successfully");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "success",
                                    "project": project_name,
                                    "path": format!("{}/.bare", project_name)
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else if !quiet_mode {
                            self.view.render(AppState::Initialized { project_name });
                        }
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        error!(error = %e, "Failed to initialize repository");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "error",
                                    "message": e.to_string()
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            self.view.render(AppState::Error(
                                e.to_string(),
                                Box::new(AppState::Welcome),
                            ));
                        }
                    }
                }
            }
            Intent::AddWorktree { intent, branch } => {
                let branch_name = branch.unwrap_or_else(|| intent.clone());
                if !json_mode {
                    self.view.render(AppState::AddingWorktree {
                        intent: intent.clone(),
                        branch: branch_name.clone(),
                    });
                }

                let pb = if !json_mode && !quiet_mode {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                            .template("{spinner:.magenta} {msg}")
                            .into_diagnostic()?,
                    );
                    pb.set_message(format!("Adding worktree: {intent}..."));
                    pb.enable_steady_tick(Duration::from_millis(100));
                    Some(pb)
                } else {
                    None
                };

                let intent_clone = intent.clone();
                let branch_name_clone = branch_name.clone();
                let repo_clone = repo.clone();

                let res = tokio::task::spawn_blocking(move || {
                    repo_clone.add_worktree(&intent_clone, &branch_name_clone)
                })
                .await
                .into_diagnostic()?;

                match res {
                    Ok(()) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        info!(%intent, %branch_name, "Worktree added successfully");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "success",
                                    "intent": intent,
                                    "branch": branch_name
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else if !quiet_mode {
                            self.view.render(AppState::WorktreeAdded { intent });
                        }
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        error!(error = %e, %intent, "Failed to add worktree");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "error",
                                    "message": e.to_string()
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            self.view.render(AppState::Error(
                                e.to_string(),
                                Box::new(AppState::Welcome),
                            ));
                        }
                    }
                }
            }
            Intent::RemoveWorktree { intent, force } => {
                if !json_mode && !quiet_mode {
                    self.view.render(AppState::RemovingWorktree {
                        intent: intent.clone(),
                    });
                }
                let intent_clone = intent.clone();
                let repo_clone = repo.clone();
                let force_clone = force;
                let res = tokio::task::spawn_blocking(move || {
                    repo_clone.remove_worktree(&intent_clone, force_clone)
                })
                .await
                .into_diagnostic()?;

                match res {
                    Ok(()) => {
                        info!(%intent, "Worktree removed successfully");
                        if json_mode {
                            self.view
                                .render_json(
                                    &serde_json::json!({ "status": "success", "intent": intent }),
                                )
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else if !quiet_mode {
                            self.view.render(AppState::WorktreeRemoved);
                        }
                    }
                    Err(e) => {
                        error!(error = %e, %intent, "Failed to remove worktree");
                        if json_mode {
                            self.view.render_json(
                                &serde_json::json!({ "status": "error", "message": e.to_string() }),
                            )
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            self.view.render(AppState::Error(
                                e.to_string(),
                                Box::new(AppState::Welcome),
                            ));
                        }
                    }
                }
            }
            Intent::ListWorktrees => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;
                info!(count = worktrees.len(), "Worktrees listed successfully");
                if json_mode {
                    self.view
                        .render_json(&worktrees)
                        .map_err(|e| miette::miette!("{e:?}"))?;
                } else {
                    self.view.render_banner();
                    if worktrees.is_empty() {
                        self.view.render(AppState::Welcome);
                    }
                    self.view.render_listing_table(&worktrees);
                    let tip = "Tip: Run with 'worktrees list' (no args) for interactive TUI";
                    if !quiet_mode {
                        println!("\n{}", tip.if_supports_color(Stdout, |t| t.dimmed()));
                    }
                }
            }
            Intent::SetupDefaults => {
                if !json_mode && !quiet_mode {
                    self.view.render(AppState::SettingUpDefaults);
                }

                let mut results = Vec::new();

                info!("Setting up default worktrees (main, dev)");
                let repo_clone = repo.clone();
                let main_res = tokio::task::spawn_blocking(move || {
                    match repo_clone.add_worktree("main", "main") {
                        Ok(()) => serde_json::json!({ "name": "main", "status": "ready" }),
                        Err(_) => serde_json::json!({ "name": "main", "status": "skipped" }),
                    }
                })
                .await
                .into_diagnostic()?;

                let status = main_res["status"].as_str().unwrap_or("unknown");
                if !json_mode {
                    if status == "ready" {
                        println!("   Main: {}", "READY".green().bold());
                    } else {
                        println!("   Main: {}", "SKIPPED".dimmed());
                    }
                }
                results.push(main_res);

                let repo_clone = repo.clone();
                let dev_res = tokio::task::spawn_blocking(move || {
                    match repo_clone.add_worktree("dev", "dev") {
                        Ok(()) => serde_json::json!({ "name": "dev", "status": "ready" }),
                        Err(_) => match repo_clone.add_new_worktree("dev", "dev", "main") {
                            Ok(()) => serde_json::json!({ "name": "dev", "status": "ready", "created_from": "main" }),
                            Err(_) => serde_json::json!({ "name": "dev", "status": "skipped" })
                        },
                    }
                }).await.into_diagnostic()?;

                let status = dev_res["status"].as_str().unwrap_or("unknown");
                if !json_mode {
                    if status == "ready" {
                        let created_from = dev_res.get("created_from").and_then(|v| v.as_str());
                        if created_from.is_some() {
                            println!("   Dev:  {}", "READY (Created from main)".green().bold());
                        } else {
                            println!("   Dev:  {}", "READY".green().bold());
                        }
                    } else {
                        println!("   Dev:  {}", "SKIPPED".dimmed());
                    }
                }
                results.push(dev_res);

                if json_mode {
                    self.view
                        .render_json(&results)
                        .map_err(|e| miette::miette!("{e:?}"))?;
                } else if !quiet_mode {
                    self.view.render(AppState::SetupComplete);
                }
            }
            Intent::RunCommand {
                intent,
                branch,
                command,
            } => {
                let branch_name = branch.unwrap_or_else(|| intent.clone());

                if !json_mode && !quiet_mode {
                    println!(
                        "{} Creating temporary worktree '{}' tracking '{}'...",
                        "➜".cyan().bold(),
                        intent,
                        branch_name
                    );
                }

                // 1. Add worktree
                let intent_clone = intent.clone();
                let branch_name_clone = branch_name.clone();
                let repo_clone = repo.clone();
                tokio::task::spawn_blocking(move || {
                    repo_clone.add_worktree(&intent_clone, &branch_name_clone)
                })
                .await
                .into_diagnostic()?
                .map_err(|e| miette::miette!("Failed to create temporary worktree: {}", e))?;

                if !json_mode && !quiet_mode {
                    println!(
                        "{} Executing command: {}",
                        "➜".cyan().bold(),
                        command.join(" ").bold()
                    );
                }

                // 2. Run command
                let command_clone = command.clone();
                let intent_clone = intent.clone();
                let status = tokio::task::spawn_blocking(move || {
                    Command::new(&command_clone[0])
                        .args(&command_clone[1..])
                        .current_dir(&intent_clone)
                        .spawn()
                        .map_err(|e| miette::miette!("Failed to spawn command: {}", e))?
                        .wait()
                        .map_err(|e| miette::miette!("Failed to wait for command: {}", e))
                })
                .await
                .into_diagnostic()??;

                if !json_mode && !quiet_mode {
                    println!("{} Cleaning up...", "➜".cyan().bold());
                }

                // 3. Remove worktree (force to ensure cleanup)
                let intent_clone = intent.clone();
                let repo_clone = repo.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    repo_clone.remove_worktree(&intent_clone, true)
                })
                .await
                .into_diagnostic()?;

                if !status.success() {
                    return Err(miette::miette!("Command failed with status: {}", status));
                }

                if !json_mode && !quiet_mode {
                    println!("{} Done.", "✔".green().bold());
                } else if json_mode {
                    self.view
                        .render_json(
                            &serde_json::json!({ "status": "success", "exit_code": status.code() }),
                        )
                        .map_err(|e| miette::miette!("{e:?}"))?;
                }
            }
            Intent::SyncConfigurations { intent } => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
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
                    if !json_mode && !quiet_mode {
                        println!(
                            "{} Synchronizing configuration for: {}",
                            "➜".cyan().bold(),
                            wt.branch.bold()
                        );
                    }
                    let repo_clone = repo.clone();
                    let path = wt.path.clone();
                    let res = tokio::task::spawn_blocking(move || repo_clone.sync_configs(&path))
                        .await
                        .into_diagnostic()?;

                    if let Err(e) = res {
                        error!(error = %e, path = %wt.path, "Configuration synchronization failed");
                        if !json_mode {
                            println!("   {} Error: {}", "❌".red(), e);
                        }
                    } else if !json_mode && !quiet_mode {
                        println!("   {} Synchronization complete.", "✔".green());
                    }
                }

                if json_mode {
                    self.view
                        .render_json(&serde_json::json!({ "status": "success" }))
                        .map_err(|e| miette::miette!("{e:?}"))?;
                }
            }
            Intent::Push { intent } => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;

                let target = if let Some(name) = intent {
                    worktrees
                        .into_iter()
                        .find(|wt| wt.branch == name || wt.path.ends_with(&name))
                } else {
                    return Err(miette::miette!(
                        "Please specify a worktree to push (e.g. 'worktrees push main')."
                    ));
                };

                if let Some(wt) = target {
                    if !json_mode && !quiet_mode {
                        println!(
                            "{} Pushing worktree: {}",
                            "➜".cyan().bold(),
                            wt.branch.bold()
                        );
                    }

                    let repo_clone = repo.clone();
                    let path = wt.path.clone();
                    let res = tokio::task::spawn_blocking(move || repo_clone.push(&path))
                        .await
                        .into_diagnostic()?;

                    match res {
                        Ok(()) => {
                            if !json_mode && !quiet_mode {
                                println!("   {} Push complete.", "✔".green());
                            }
                            if json_mode {
                                self.view.render_json(&serde_json::json!({ "status": "success", "branch": wt.branch }))
                                    .map_err(|e| miette::miette!("{e:?}"))?;
                            }
                        }
                        Err(e) => {
                            error!(error = %e, branch = %wt.branch, "Push failed");
                            if !json_mode {
                                println!("   {} Error: {}", "❌".red(), e);
                            }
                            return Err(miette::miette!("Push failed: {}", e));
                        }
                    }
                } else {
                    return Err(miette::miette!("Worktree not found."));
                }
            }

            Intent::Pull { intent } => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;

                let target = if let Some(name) = intent {
                    worktrees
                        .into_iter()
                        .find(|wt| wt.branch == name || wt.path.ends_with(&name))
                } else {
                    return Err(miette::miette!(
                        "Please specify a worktree to pull (e.g. 'worktrees pull main')."
                    ));
                };

                if let Some(wt) = target {
                    if !json_mode && !quiet_mode {
                        println!(
                            "{} Pulling worktree: {}",
                            "➜".cyan().bold(),
                            wt.branch.bold()
                        );
                    }

                    let repo_clone = repo.clone();
                    let path = wt.path.clone();
                    let res = tokio::task::spawn_blocking(move || repo_clone.pull(&path))
                        .await
                        .into_diagnostic()?;

                    match res {
                        Ok(()) => {
                            if !json_mode && !quiet_mode {
                                println!("   {} Pull complete.", "✔".green());
                            }
                            if json_mode {
                                self.view.render_json(&serde_json::json!({ "status": "success", "branch": wt.branch }))
                                    .map_err(|e| miette::miette!("{e:?}"))?;
                            }
                        }
                        Err(e) => {
                            error!(error = %e, branch = %wt.branch, "Pull failed");
                            if !json_mode {
                                println!("   {} Error: {}", "❌".red(), e);
                            }
                            return Err(miette::miette!("Pull failed: {}", e));
                        }
                    }
                } else {
                    return Err(miette::miette!("Worktree not found."));
                }
            }
            Intent::Config { key, show } => {
                if let Some(k) = key {
                    let k_clone = k.clone();
                    let repo_clone = repo.clone();
                    tokio::task::spawn_blocking(move || repo_clone.set_api_key(&k_clone))
                        .await
                        .into_diagnostic()?
                        .map_err(|e| miette::miette!("Failed to set API key: {}", e))?;

                    if !json_mode && !quiet_mode {
                        println!("{} Gemini API key set successfully.", "✔".green().bold());
                    } else if json_mode {
                        self.view
                            .render_json(
                                &serde_json::json!({ "status": "success", "action": "set_key" }),
                            )
                            .map_err(|e| miette::miette!("{e:?}"))?;
                    }
                } else if show {
                    let repo_clone = repo.clone();
                    let k = tokio::task::spawn_blocking(move || repo_clone.get_api_key())
                        .await
                        .into_diagnostic()?
                        .map_err(|e| miette::miette!("Failed to get API key: {}", e))?;
                    if json_mode {
                        self.view
                            .render_json(&serde_json::json!({ "status": "success", "key": k }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                    } else if let Some(val) = k {
                        println!("{} Current API key: {}", "➜".cyan().bold(), val);
                    } else {
                        println!("{} No API key found.", "⚠".yellow().bold());
                    }
                }
            }
            Intent::CleanWorktrees { dry_run, artifacts } => {
                if !json_mode {
                    if artifacts {
                        if dry_run {
                            println!(
                                "{} Scanning for build artifacts in inactive worktrees (dry-run)...",
                                "➜".cyan().bold()
                            );
                        } else {
                            println!(
                                "{} Cleaning build artifacts from inactive worktrees...",
                                "➜".cyan().bold()
                            );
                        }
                    } else if dry_run {
                        println!(
                            "{} Scanning for stale worktrees (dry-run)...",
                            "➜".cyan().bold()
                        );
                    } else if !quiet_mode {
                        println!("{} Cleaning stale worktrees...", "➜".cyan().bold());
                    }
                }

                let repo_clone = repo.clone();
                let stale_worktrees = tokio::task::spawn_blocking(move || {
                    repo_clone.clean_worktrees(dry_run, artifacts)
                })
                .await
                .into_diagnostic()?
                .map_err(|e| miette::miette!("Failed to clean worktrees: {}", e))?;

                if stale_worktrees.is_empty() {
                    if !json_mode && !quiet_mode {
                        println!("{} No stale worktrees found.", "✔".green().bold());
                    } else if json_mode {
                        self.view
                            .render_json(&serde_json::json!({
                                "status": "success",
                                "stale_count": 0,
                                "stale_worktrees": []
                            }))
                            .map_err(|e| miette::miette!("{e:?}"))?;
                    }
                } else if !json_mode {
                    if dry_run {
                        println!(
                            "\n{} Found {} stale worktree(s) that would be removed:",
                            "⚠".yellow().bold(),
                            stale_worktrees.len()
                        );
                    } else {
                        println!(
                            "\n{} Removed {} stale worktree(s):",
                            "✔".green().bold(),
                            stale_worktrees.len()
                        );
                    }
                    for wt in &stale_worktrees {
                        println!("   • {}", wt.dimmed());
                    }
                    if dry_run {
                        println!(
                            "\n{} Run without --dry-run to actually remove these worktrees.",
                            "Tip:".cyan().bold()
                        );
                    }
                } else {
                    self.view
                        .render_json(&serde_json::json!({
                            "status": "success",
                            "dry_run": dry_run,
                            "stale_count": stale_worktrees.len(),
                            "stale_worktrees": stale_worktrees
                        }))
                        .map_err(|e| miette::miette!("{e:?}"))?;
                }
            }
            Intent::SwitchWorktree { name, copy } => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;

                let candidates: Vec<&Worktree> =
                    worktrees.iter().filter(|wt| !wt.is_bare).collect();

                let needle = name.to_lowercase();

                // Priority: exact branch → exact dir name → substring branch → substring path
                let matched = candidates
                    .iter()
                    .find(|wt| wt.branch.to_lowercase() == needle)
                    .or_else(|| {
                        candidates.iter().find(|wt| {
                            Path::new(&wt.path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.to_lowercase() == needle)
                        })
                    })
                    .or_else(|| {
                        candidates
                            .iter()
                            .find(|wt| wt.branch.to_lowercase().contains(&needle))
                    })
                    .or_else(|| {
                        candidates
                            .iter()
                            .find(|wt| wt.path.to_lowercase().contains(&needle))
                    });

                match matched {
                    Some(wt) => {
                        if copy {
                            // Copy path to clipboard using pbcopy on macOS
                            #[cfg(target_os = "macos")]
                            {
                                use std::io::Write;
                                use std::process::Stdio;
                                let mut child = Command::new("pbcopy")
                                    .stdin(Stdio::piped())
                                    .spawn()
                                    .map_err(|e| {
                                        miette::miette!("Failed to spawn pbcopy: {}", e)
                                    })?;

                                if let Some(mut stdin) = child.stdin.take() {
                                    stdin.write_all(wt.path.as_bytes()).map_err(|e| {
                                        miette::miette!("Failed to write to pbcopy: {}", e)
                                    })?;
                                }
                                child.wait().map_err(|e| {
                                    miette::miette!("Failed to wait for pbcopy: {}", e)
                                })?;
                            }
                        }

                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "success",
                                    "path": wt.path,
                                    "branch": wt.branch,
                                    "copied": copy
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            // Print ONLY the path to stdout for shell integration
                            println!("{}", wt.path);
                        }
                    }
                    None => {
                        return Err(miette::miette!(
                            "No worktree found matching '{}'. Run 'worktrees list' to see available worktrees.",
                            name
                        ));
                    }
                }
            }
            Intent::Convert { name, branch } => {
                if !json_mode && !quiet_mode {
                    println!(
                        "{} Converting standard repository to Bare Hub structure...",
                        "➜".cyan().bold()
                    );
                }

                let pb = if !json_mode && !quiet_mode {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                            .template("{spinner:.cyan} {msg}")
                            .into_diagnostic()?,
                    );
                    pb.set_message("Migrating .git to .bare and setting up hub...");
                    pb.enable_steady_tick(Duration::from_millis(100));
                    Some(pb)
                } else {
                    None
                };

                let name_clone = name.clone();
                let branch_clone = branch.clone();
                let repo_clone = repo.clone();

                let res = tokio::task::spawn_blocking(move || {
                    repo_clone.convert_to_bare(name_clone.as_deref(), branch_clone.as_deref())
                })
                .await
                .into_diagnostic()?;

                match res {
                    Ok(hub_path) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        info!(path = ?hub_path, "Repository converted successfully");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "success",
                                    "hub_path": hub_path.to_string_lossy()
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            println!("{} Conversion complete!", "✔".green().bold());
                            println!(
                                "{} New hub created at: {}",
                                "➜".cyan().bold(),
                                hub_path.to_string_lossy().bold()
                            );
                            println!(
                                "\n{} You can now move into the new hub and start working:",
                                "Tip:".cyan().bold()
                            );
                            println!("   cd {}", hub_path.to_string_lossy());
                        }
                    }
                    Err(e) => {
                        if let Some(pb) = pb {
                            pb.finish_and_clear();
                        }
                        error!(error = %e, "Failed to convert repository");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "error",
                                    "message": e.to_string()
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            self.view.render(AppState::Error(
                                e.to_string(),
                                Box::new(AppState::Welcome),
                            ));
                        }
                    }
                }
            }
            Intent::Migrate { force, dry_run } => {
                let repo_clone = repo.clone();
                let force_clone = force;
                let dry_run_clone = dry_run;

                if !json_mode && !quiet_mode {
                    println!(
                        "{} Migrating repository to Bare Hub structure (in-place)...",
                        "➜".cyan().bold()
                    );
                }

                let pb = if !json_mode && !quiet_mode && !dry_run {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                            .template("{spinner:.cyan} {msg}")
                            .into_diagnostic()?,
                    );
                    pb.set_message("Migrating in-place...");
                    pb.enable_steady_tick(Duration::from_millis(100));
                    Some(pb)
                } else {
                    None
                };

                let res = tokio::task::spawn_blocking(move || {
                    repo_clone.migrate_to_bare(force_clone, dry_run_clone)
                })
                .await
                .into_diagnostic()?;

                if let Some(pb) = pb {
                    pb.finish_and_clear();
                }

                match res {
                    Ok(path) => {
                        if dry_run {
                            if !json_mode {
                                println!(
                                    "\n{} Dry run complete. Migration would create worktree at: {}",
                                    "✔".green().bold(),
                                    path.display()
                                );
                            } else {
                                self.view
                                    .render_json(&serde_json::json!({
                                        "status": "success",
                                        "dry_run": true,
                                        "would_create": path
                                    }))
                                    .map_err(|e| miette::miette!("{e:?}"))?;
                            }
                        } else {
                            info!("Repository migrated successfully");
                            if json_mode {
                                self.view
                                    .render_json(&serde_json::json!({
                                        "status": "success",
                                        "path": path,
                                        "message": "Repository migrated to Bare Hub structure."
                                    }))
                                    .map_err(|e| miette::miette!("{e:?}"))?;
                            } else if !quiet_mode {
                                println!(
                                    "\n{} Migration complete! You are now in a Bare Hub.",
                                    "✔".green().bold()
                                );
                                println!("   New main worktree: {}", path.display().bold());
                                println!("   To start working, cd into the new worktree.");
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to migrate repository");
                        if json_mode {
                            self.view.render_json(
                                &serde_json::json!({ "status": "error", "message": e.to_string() }),
                            )
                            .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            println!("\n{} Migration failed: {}", "❌".red().bold(), e);
                            if e.to_string().contains("already exists") {
                                println!("   Tip: Use --force to overwrite existing directories.");
                            }
                        }
                    }
                }
            }
            Intent::CheckoutWorktree { intent, branch } => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;

                let needle = intent.to_lowercase();
                let wt = worktrees
                    .iter()
                    .filter(|wt| !wt.is_bare)
                    .find(|wt| {
                        wt.branch.to_lowercase() == needle
                            || Path::new(&wt.path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.to_lowercase() == needle)
                    })
                    .ok_or_else(|| miette::miette!("Worktree '{}' not found.", intent))?;

                let path = wt.path.clone();
                let branch_clone = branch.clone();
                let repo_clone2 = repo.clone();
                let res = tokio::task::spawn_blocking(move || {
                    repo_clone2.switch_branch(&path, &branch_clone)
                })
                .await
                .into_diagnostic()?;

                match res {
                    Ok(()) => {
                        info!(%intent, %branch, "Worktree branch switched successfully");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "success",
                                    "intent": intent,
                                    "branch": branch
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            println!(
                                "{} Worktree '{}' switched to branch '{}'.",
                                "✔".green().bold(),
                                intent,
                                branch
                            );
                        }
                    }
                    Err(e) => {
                        error!(error = %e, %intent, %branch, "Failed to switch worktree branch");
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "error",
                                    "message": e.to_string()
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        } else {
                            self.view.render(AppState::Error(
                                e.to_string(),
                                Box::new(AppState::Welcome),
                            ));
                        }
                    }
                }
            }
            Intent::Completions { shell } => {
                use clap::CommandFactory;
                let mut cmd = crate::cli::Cli::command();
                let bin_name = cmd.get_name().to_string();
                clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
            }
            Intent::Open => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;

                let root = repo
                    .get_project_root()
                    .map_err(|e| miette::miette!("{e:?}"))?;
                let project_name = root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("project");

                let yaml = crate::app::warp::generate_config(project_name, &worktrees);

                if json_mode {
                    self.view
                        .render_json(&serde_json::json!({
                            "status": "success",
                            "config": yaml
                        }))
                        .map_err(|e| miette::miette!("{e:?}"))?;
                } else {
                    println!("\n{}", "Generated Warp Launch Configuration:".cyan().bold());
                    println!("---");
                    println!("{yaml}");
                    println!("---");
                    println!("\n{}", "To use this configuration:".yellow().bold());
                    println!("1. Save the above content to a file, e.g., `warp-launch.yaml`.");
                    println!(
                        "2. Use `warp-cli launch-config warp-launch.yaml` if you have Warp CLI installed."
                    );
                    println!("3. Or copy/paste into Warp's Launch Configuration editor.");
                }
            }
            Intent::Rebase { upstream } => {
                let upstream_branch = upstream.unwrap_or_else(|| "main".to_string());
                let current_dir = std::env::current_dir().into_diagnostic()?;
                let path = current_dir.to_string_lossy().to_string();

                if !json_mode && !quiet_mode {
                    println!(
                        "{} Rebasing current worktree onto '{}'...",
                        "➜".cyan().bold(),
                        upstream_branch.bold()
                    );
                }

                let repo_clone = repo.clone();
                let upstream_clone = upstream_branch.clone();
                let path_clone = path.clone();

                let res = tokio::task::spawn_blocking(move || {
                    repo_clone.rebase(&path_clone, &upstream_clone)
                })
                .await
                .into_diagnostic()?;

                match res {
                    Ok(()) => {
                        if !json_mode && !quiet_mode {
                            println!("{} Rebase complete.", "✔".green().bold());
                        }
                        if json_mode {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "success",
                                    "upstream": upstream_branch
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Rebase failed");
                        if !json_mode {
                            println!("\n{} Rebase failed: {}", "❌".red().bold(), e);
                            println!(
                                "{} Analyzing conflicts with Gemini AI...",
                                "➜".cyan().bold()
                            );

                            let repo_clone = repo.clone();
                            let path_clone = path;
                            let explanation_res = tokio::task::spawn_blocking(move || {
                                let diff = repo_clone.get_conflict_diff(&path_clone)?;
                                repo_clone.explain_rebase_conflict(&diff)
                            })
                            .await
                            .into_diagnostic()?;

                            match explanation_res {
                                Ok(explanation) => {
                                    println!("\n{}", "AI Conflict Explanation:".yellow().bold());
                                    println!("{}", explanation);
                                }
                                Err(qe) => {
                                    error!(error = %qe, "Gemini explanation failed");
                                }
                            }
                        } else {
                            self.view
                                .render_json(&serde_json::json!({
                                    "status": "error",
                                    "message": e.to_string()
                                }))
                                .map_err(|e| miette::miette!("{e:?}"))?;
                        }
                    }
                }
            }
            Intent::Teleport { target } => {
                let repo_clone = repo.clone();
                let worktrees = tokio::task::spawn_blocking(move || repo_clone.list_worktrees())
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!("{e:?}"))?;

                let candidates: Vec<&Worktree> =
                    worktrees.iter().filter(|wt| !wt.is_bare).collect();

                // 1. Resolve Current Worktree
                let current_dir = std::env::current_dir().into_diagnostic()?;
                let current_dir_canonical = current_dir.canonicalize().unwrap_or(current_dir);

                let source_wt = candidates.iter().find(|wt| {
                    Path::new(&wt.path)
                        .canonicalize()
                        .map(|p| p == current_dir_canonical)
                        .unwrap_or(false)
                });

                let source_wt = match source_wt {
                    Some(wt) => wt,
                    None => {
                        return Err(miette::miette!(
                            "Not currently in a managed worktree. Teleport must be run from a worktree directory."
                        ));
                    }
                };

                // 2. Resolve Target Worktree (using fuzzy match logic like Switch)
                let needle = target.to_lowercase();
                let matched_target = candidates
                    .iter()
                    .find(|wt| wt.branch.to_lowercase() == needle)
                    .or_else(|| {
                        candidates.iter().find(|wt| {
                            Path::new(&wt.path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.to_lowercase() == needle)
                        })
                    })
                    .or_else(|| {
                        candidates
                            .iter()
                            .find(|wt| wt.branch.to_lowercase().contains(&needle))
                    })
                    .or_else(|| {
                        candidates
                            .iter()
                            .find(|wt| wt.path.to_lowercase().contains(&needle))
                    });

                let target_wt = match matched_target {
                    Some(wt) => wt,
                    None => {
                        return Err(miette::miette!(
                            "Target worktree '{}' not found.",
                            target
                        ));
                    }
                };

                if source_wt.path == target_wt.path {
                    return Err(miette::miette!("Already in target worktree '{}'.", target_wt.branch));
                }

                // 3. Verify changes exist
                let status = repo.get_status(&source_wt.path).map_err(|e| miette::miette!(e))?;
                if status.staged.is_empty() && status.unstaged.is_empty() && status.untracked.is_empty() {
                    if !json_mode && !quiet_mode {
                        println!("{} Current worktree is clean. Nothing to teleport.", "ℹ".blue());
                    }
                    return Ok(());
                }

                if !json_mode && !quiet_mode {
                    println!(
                        "{} Teleporting changes from '{}' to '{}'...",
                        "➜".cyan().bold(),
                        source_wt.branch.bold(),
                        target_wt.branch.bold()
                    );
                }

                let source_path = source_wt.path.clone();
                let target_path = target_wt.path.clone();
                let target_branch = target_wt.branch.clone();
                let repo_clone2 = repo.clone();

                let res = tokio::task::spawn_blocking(move || {
                    // Stash changes
                    let msg = format!("Teleport to {}", target_branch);
                    repo_clone2.stash_save(&source_path, Some(&msg))?;

                    // Verify stash exists (it should be at index 0)
                    let stashes = repo_clone2.list_stashes(&source_path)?;
                    if stashes.is_empty() {
                         return Err(anyhow::anyhow!("Failed to create stash for teleport."));
                    }

                    // Apply to target
                    match repo_clone2.apply_stash(&target_path, 0) {
                        Ok(()) => {
                            // Only drop if apply succeeded
                            repo_clone2.drop_stash(&source_path, 0)?;
                            Ok(())
                        }
                        Err(e) => {
                            Err(anyhow::anyhow!("Failed to apply changes to target '{}': {}. Changes preserved in stash@{{0}}.", target_branch, e))
                        }
                    }
                }).await.into_diagnostic()?;

                res.map_err(|e| miette::miette!(e.to_string()))?;

                if !json_mode && !quiet_mode {
                    println!("{} Teleport complete!", "✔".green().bold());
                } else if json_mode {
                    self.view.render_json(&serde_json::json!({
                        "status": "success",
                        "from": source_wt.branch,
                        "to": target_wt.branch
                    })).map_err(|e| miette::miette!("{e:?}"))?;
                }
            }
            Intent::ApplyStash { path, index } => {
                let repo_clone = repo.clone();
                let path_clone = path;
                tokio::task::spawn_blocking(move || repo_clone.apply_stash(&path_clone, index))
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!(e.to_string()))?;
            }
            Intent::PopStash { path, index } => {
                let repo_clone = repo.clone();
                let path_clone = path;
                tokio::task::spawn_blocking(move || repo_clone.pop_stash(&path_clone, index))
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!(e.to_string()))?;
            }
            Intent::DropStash { path, index } => {
                let repo_clone = repo.clone();
                let path_clone = path;
                tokio::task::spawn_blocking(move || repo_clone.drop_stash(&path_clone, index))
                    .await
                    .into_diagnostic()?
                    .map_err(|e| miette::miette!(e.to_string()))?;
            }
            Intent::StashSave { path, message } => {
                let repo_clone = repo.clone();
                let path_clone = path;
                tokio::task::spawn_blocking(move || {
                    repo_clone.stash_save(&path_clone, message.as_deref())
                })
                .await
                .into_diagnostic()?
                .map_err(|e| miette::miette!(e.to_string()))?;
            }
            Intent::ViewStashes { .. } => {}
            Intent::ChangeMode(_) => {
                // This is primarily handled in listing.rs for TUI
                // but we keep the variant here for intent completeness.
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::{ProjectRepository, RepositoryEvent, Worktree};
    use anyhow::Result;
    use crossbeam_channel::Receiver;

    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct CallTracker {
        calls: Vec<String>,
    }

    #[derive(Clone)]
    struct MockRepo {
        tracker: Arc<Mutex<CallTracker>>,
    }

    impl MockRepo {
        fn new(tracker: Arc<Mutex<CallTracker>>) -> Self {
            Self { tracker }
        }
    }

    impl ProjectRepository for MockRepo {
        fn init_bare_repo(&self, url: Option<&str>, name: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("init:{url:?}|{name}"));
            Ok(())
        }
        fn add_worktree(&self, intent: &str, branch: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("add:{intent}|{branch}"));
            Ok(())
        }
        fn add_new_worktree(&self, intent: &str, branch: &str, base: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("add_new:{intent}|{branch}|{base}"));
            Ok(())
        }
        fn remove_worktree(&self, intent: &str, force: bool) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("remove:{intent}|force:{force}"));
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
                    size_bytes: 0,
                    metadata: None,
                },
                Worktree {
                    path: "dev".to_string(),
                    commit: "abcdef0".to_string(),
                    branch: "dev".to_string(),
                    is_bare: false,
                    is_detached: false,
                    status_summary: Some("~1".to_string()),
                    size_bytes: 0,
                    metadata: None,
                },
            ])
        }
        fn sync_configs(&self, path: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("sync:{path}"));
            Ok(())
        }

        fn get_project_root(&self) -> anyhow::Result<std::path::PathBuf> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push("get_project_root".to_string());
            Ok(std::path::PathBuf::from("/mock/root"))
        }

        fn pull(&self, path: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("pull:{path}"));
            Ok(())
        }
        fn detect_context(
            &self,
            _base_path: &std::path::Path,
        ) -> crate::domain::repository::ProjectContext {
            crate::domain::repository::ProjectContext::Standard
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
        fn push(&self, path: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("push:{path}"));
            Ok(())
        }
        fn get_status(&self, _path: &str) -> anyhow::Result<crate::domain::repository::GitStatus> {
            Ok(crate::domain::repository::GitStatus {
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
        ) -> anyhow::Result<Vec<crate::domain::repository::GitCommit>> {
            Ok(vec![])
        }
        fn list_branches(&self) -> anyhow::Result<Vec<String>> {
            Ok(vec!["main".to_string(), "dev".to_string()])
        }
        fn switch_branch(&self, path: &str, branch: &str) -> anyhow::Result<()> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("switch:{path}|{branch}"));
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
        fn clean_worktrees(&self, _dry_run: bool, _artifacts: bool) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }
        fn convert_to_bare(
            &self,
            name: Option<&str>,
            branch: Option<&str>,
        ) -> Result<std::path::PathBuf> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("convert:{name:?}|{branch:?}"));
            Ok(std::path::PathBuf::from("hub"))
        }

        fn migrate_to_bare(&self, force: bool, dry_run: bool) -> Result<std::path::PathBuf> {
            self.tracker
                .lock()
                .unwrap()
                .calls
                .push(format!("migrate:force={force}|dry_run={dry_run}"));
            Ok(std::path::PathBuf::from("migrated_hub"))
        }

        fn check_status(&self, _path: &Path) -> crate::domain::repository::RepoStatus {
            crate::domain::repository::RepoStatus::BareHub
        }

        fn rebase(&self, _path: &str, _upstream: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn get_conflict_diff(&self, _path: &str) -> anyhow::Result<String> {
            Ok("conflict diff".to_string())
        }

        fn explain_rebase_conflict(&self, _diff: &str) -> anyhow::Result<String> {
            Ok("AI explanation".to_string())
        }

        fn watch(&self) -> Result<Receiver<RepositoryEvent>> {
            let (_, rx) = crossbeam_channel::unbounded();
            Ok(rx)
        }

        fn list_stashes(
            &self,
            _path: &str,
        ) -> anyhow::Result<Vec<crate::domain::repository::StashEntry>> {
            Ok(vec![])
        }

        fn apply_stash(&self, _path: &str, _index: usize) -> anyhow::Result<()> {
            Ok(())
        }

        fn pop_stash(&self, _path: &str, _index: usize) -> anyhow::Result<()> {
            Ok(())
        }

        fn drop_stash(&self, _path: &str, _index: usize) -> anyhow::Result<()> {
            Ok(())
        }

        fn stash_save(&self, _path: &str, _message: Option<&str>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_get_project_name() {
        assert_eq!(
            get_project_name(Some(&"https://github.com/user/repo.git".to_string()), None),
            "repo"
        );
        assert_eq!(
            get_project_name(Some(&"git@github.com:user/my-project".to_string()), None),
            "my-project"
        );
        assert_eq!(
            get_project_name(
                Some(&"https://github.com/user/repo.git".to_string()),
                Some("custom".to_string())
            ),
            "custom"
        );
        assert_eq!(
            get_project_name(Some(&"/path/to/local/repo".to_string()), None),
            "repo"
        );
        assert_eq!(
            get_project_name(None, Some("my-project".to_string())),
            "my-project"
        );
        assert_eq!(get_project_name(None, None), "project");
    }

    #[tokio::test]
    async fn test_reducer_handle_init_fresh() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::Initialize {
                url: None,
                name: Some("fresh-project".to_string()),
                warp: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert_eq!(calls[0], "init:None|fresh-project");
        assert_eq!(calls[1], "add_new:main|main|HEAD");
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_init() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::Initialize {
                url: Some("https://github.com/user/repo.git".to_string()),
                name: None,
                warp: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        assert_eq!(
            tracker.lock().unwrap().calls,
            vec!["init:Some(\"https://github.com/user/repo.git\")|repo"]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_add() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::AddWorktree {
                intent: "feat-x".to_string(),
                branch: Some("feature/x".to_string()),
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        assert_eq!(tracker.lock().unwrap().calls, vec!["add:feat-x|feature/x"]);
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_setup() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::SetupDefaults)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"add:main|main".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_run() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        // Create a dummy directory to satisfy .current_dir(&intent)
        let temp_dir = "temp-run-test";
        let _ = std::fs::create_dir(temp_dir);

        let res = reducer
            .handle(Intent::RunCommand {
                intent: temp_dir.to_string(),
                branch: Some("main".to_string()),
                command: vec!["echo".to_string(), "hello".to_string()],
            })
            .await;

        // Cleanup the dummy directory
        let _ = std::fs::remove_dir(temp_dir);

        res.map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert_eq!(calls[0], format!("add:{temp_dir}|main"));
        assert_eq!(calls[1], format!("remove:{temp_dir}|force:true"));
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_sync() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::SyncConfigurations {
                intent: Some("main".to_string()),
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"list".to_string()));
        assert!(calls.contains(&"sync:main".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_push() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::Push {
                intent: Some("main".to_string()),
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"list".to_string()));
        assert!(calls.contains(&"push:main".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_clean() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::CleanWorktrees {
                dry_run: true,
                artifacts: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_switch() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::SwitchWorktree {
                name: "dev".to_string(),
                copy: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"list".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_reducer_handle_switch_not_found() {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        let result = reducer
            .handle(Intent::SwitchWorktree {
                name: "nonexistent".to_string(),
                copy: false,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reducer_handle_checkout() -> Result<()> {
        let tracker = Arc::new(Mutex::new(CallTracker::default()));
        let repo = MockRepo::new(tracker.clone());
        let reducer = Reducer::new(repo, false, false);

        reducer
            .handle(Intent::CheckoutWorktree {
                intent: "dev".to_string(),
                branch: "feature-y".to_string(),
            })
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let calls = tracker.lock().unwrap().calls.clone();
        assert!(calls.contains(&"list".to_string()));
        assert!(calls.contains(&"switch:dev|feature-y".to_string()));
        Ok(())
    }
}
