mod app;
mod cli;
mod domain;
mod infrastructure;
mod ui;

use app::intent::Intent;
use app::model::AppState;
use app::reducer::Reducer;
use app::view::View;
use clap::Parser;
use cli::{Cli, Commands};
use domain::repository::ProjectRepository;
use infrastructure::git_repo::GitProjectRepository;
use miette::Result;
use ratatui::widgets::TableState;
use tracing::{error, info};

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
        Some(Commands::Push { intent }) => Intent::Push { intent },
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
        Some(Commands::Clean { dry_run }) => Intent::CleanWorktrees { dry_run },
        Some(Commands::Switch { name }) => match name {
            Some(n) => Intent::SwitchWorktree { name: n },
            None => {
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
                    selection_mode: true,
                    dashboard: app::model::DashboardState {
                        active_tab: app::model::DashboardTab::Info,
                        cached_status: None,
                        cached_history: None,
                    },
                };
                let result = View::render_tui(&GitProjectRepository, initial_state)
                    .map_err(|e| miette::miette!("{e:?}"))?;

                if let Some(path) = result {
                    println!("{}", path);
                }
                return Ok(());
            }
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
                selection_mode: false,
                dashboard: app::model::DashboardState {
                    active_tab: app::model::DashboardTab::Info,
                    cached_status: None,
                    cached_history: None,
                },
            };
            View::render_tui(&GitProjectRepository, initial_state)
                .map_err(|e| miette::miette!("{e:?}"))?;
            return Ok(());
        }
    };

    tokio::select! {
        res = reducer.handle(intent) => {
            res?;
        }
        _ = wait_for_shutdown() => {}
    }

    if !cli.json {
        View::render_feedback_prompt();
    }

    Ok(())
}
