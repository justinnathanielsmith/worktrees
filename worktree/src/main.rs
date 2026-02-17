use clap::Parser;
use miette::Result;
use ratatui::widgets::TableState;
use std::io::{self, Write};
use tracing::{error, info};
use worktree::app::intent::Intent;
use worktree::app::model::{AppState, RefreshType};
use worktree::app::reducer::Reducer;
use worktree::app::view::View;
use worktree::cli::{self, Cli, Commands};
use worktree::domain::repository::{ProjectRepository, RepoStatus};
use worktree::infrastructure::git_repo::GitProjectRepository;

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

fn check_and_handle_repo_state(repo: &GitProjectRepository) -> Result<bool> {
    let current_dir = std::env::current_dir().map_err(|e| miette::miette!(e))?;
    match repo.check_status(&current_dir) {
        RepoStatus::BareHub => Ok(true),
        RepoStatus::StandardGit => {
            print!(
                "This directory is a standard Git repository. The tool requires a Bare Hub setup.\nDo you want to MIGRATE it in-place to a Bare Hub? (This will move current files to a worktree) (y/N): "
            );
            io::stdout().flush().map_err(|e| miette::miette!(e))?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| miette::miette!(e))?;
            if input.trim().to_lowercase() == "y" {
                let path = repo
                    .migrate_to_bare(false, false) // force=false, dry_run=false
                    .map_err(|e| miette::miette!(e))?;
                println!("Migration successful! The Bare Hub is now set up.");
                println!("Main worktree is located at: {}", path.display());
                println!("Please navigate to that directory to continue.");
            }
            Ok(false)
        }
        RepoStatus::NoRepo => {
            print!(
                "No Git repository found. Do you want to create a new Bare Hub project? (y/N): "
            );
            io::stdout().flush().map_err(|e| miette::miette!(e))?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| miette::miette!(e))?;
            if input.trim().to_lowercase() == "y" {
                print!("Enter project name: ");
                io::stdout().flush().map_err(|e| miette::miette!(e))?;
                let mut name = String::new();
                io::stdin()
                    .read_line(&mut name)
                    .map_err(|e| miette::miette!(e))?;
                let name = name.trim();
                if name.is_empty() {
                    println!("Operation cancelled.");
                    return Ok(false);
                }
                repo.init_bare_repo(None, name)
                    .map_err(|e| miette::miette!(e))?;
                println!("Project initialized! Please navigate to '{name}' to continue.");
            }
            Ok(false)
        }
    }
}

fn render_tui_mode(
    repo: &GitProjectRepository,
    selection_mode: bool,
    quiet: bool,
) -> Result<Option<String>> {
    if !check_and_handle_repo_state(repo)? {
        return Ok(None);
    }
    if !quiet {
        View::render_banner();
    }
    let worktrees = repo
        .list_worktrees()
        .map_err(|e| miette::miette!("{e:?}"))?;
    let mut table_state = TableState::default();
    if !worktrees.is_empty() {
        table_state.select(Some(0));
    }
    let initial_state = AppState::ListingWorktrees {
        worktrees,
        table_state,
        refresh_needed: RefreshType::None,
        selection_mode,
        dashboard: worktree::app::model::DashboardState {
            active_tab: worktree::app::model::DashboardTab::Info,
            cached_status: None,
            cached_history: None,
            loading: false,
        },
        filter_query: String::new(),
        is_filtering: false,
        mode: worktree::app::model::AppMode::Normal,
    };
    View::render_tui(repo, initial_state).map_err(|e| miette::miette!("{e:?}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.json);

    let repo = GitProjectRepository;
    let reducer = Reducer::new(repo, cli.json, cli.quiet);

    let intent = match cli.command {
        Some(Commands::Init { url, name, warp }) => Intent::Initialize { url, name, warp },
        Some(Commands::Add { intent, branch }) => Intent::AddWorktree { intent, branch },
        Some(Commands::Remove { intent, force }) => Intent::RemoveWorktree { intent, force },
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
        Some(Commands::Clean { dry_run, artifacts }) => {
            Intent::CleanWorktrees { dry_run, artifacts }
        }
        Some(Commands::Switch { name, copy }) => {
            if let Some(n) = name {
                Intent::SwitchWorktree { name: n, copy }
            } else {
                let result = render_tui_mode(&GitProjectRepository, true, cli.quiet)?;
                if let Some(path) = result {
                    println!("{path}");
                }
                return Ok(());
            }
        }
        Some(Commands::Convert { name, branch }) => Intent::Convert { name, branch },
        Some(Commands::Migrate { force, dry_run }) => Intent::Migrate { force, dry_run },
        Some(Commands::Checkout { intent, branch }) => Intent::CheckoutWorktree { intent, branch },
        Some(Commands::Completions { shell }) => Intent::Completions { shell },
        Some(Commands::Open) => Intent::Open,
        Some(Commands::Rebase { upstream }) => Intent::Rebase { upstream },
        Some(Commands::Teleport { target }) => Intent::Teleport { target },
        None => {
            if cli.json {
                let worktrees = GitProjectRepository
                    .list_worktrees()
                    .map_err(|e| miette::miette!("{e:?}"))?;
                return View::render_json(&worktrees).map_err(|e| miette::miette!("{e:?}"));
            }
            // TUI Mode
            render_tui_mode(&GitProjectRepository, false, cli.quiet)?;
            return Ok(());
        }
    };

    tokio::select! {
        res = reducer.handle(intent) => {
            res?;
        }
        () = wait_for_shutdown() => {}
    }

    if !cli.json && !cli.quiet {
        View::render_feedback_prompt();
    }

    Ok(())
}
