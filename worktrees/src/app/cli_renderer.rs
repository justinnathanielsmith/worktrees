use crate::app::model::AppState;
use crate::domain::repository::Worktree;
use anyhow::Result;
use comfy_table::Table;
use owo_colors::OwoColorize;

pub struct CliRenderer;

impl CliRenderer {
    pub fn render_banner() {
        let lines = [
            r#"██╗    ██╗ ██████╗ ██████╗ ██╗  ██╗████████╗██████╗ ███████╗███████╗███████╗"#,
            r#"██║    ██║██╔═══██╗██╔══██╗██║ ██╔╝╚══██╔══╝██╔══██╗██╔════╝██╔════╝██╔════╝"#,
            r#"██║ █╗ ██║██║   ██║██████╔╝█████╔╝    ██║   ██████╔╝█████╗  █████╗  ███████╗"#,
            r#"██║███╗██║██║   ██║██╔══██╗██╔═██╗    ██║   ██╔══██╗██╔══╝  ██╔══╝  ╚════██║"#,
            r#"╚███╔███╔╝╚██████╔╝██║  ██║██║  ██╗   ██║   ██║  ██║███████╗███████╗███████║"#,
            r#" ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝"#,
        ];

        let colors = [
            (6, 182, 212),  // Cyan
            (34, 158, 234), // Sky-Blue
            (59, 130, 246), // Blue
            (99, 102, 241), // Indigo
            (139, 92, 246), // Violet
            (168, 85, 247), // Purple
        ];

        for (i, line) in lines.iter().enumerate() {
            let (r, g, b) = colors.get(i).unwrap_or(&(168, 85, 247));
            println!("{}", line.truecolor(*r, *g, *b).bold());
        }

        println!(
            "                    {}",
            "HI-RES WORKTREE INFRASTRUCTURE"
                .truecolor(6, 182, 212)
                .italic()
        );
        println!("{}\n", "━".repeat(76).truecolor(59, 130, 246).dimmed());
    }

    pub fn render_json<T: serde::Serialize>(data: &T) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        println!("{}", json);
        Ok(())
    }

    pub fn render_listing_table(worktrees: &[Worktree]) {
        let mut table = Table::new();
        table.set_header(vec!["Branch", "Commit", "Path", "Status"]);

        for wt in worktrees {
            let status = if wt.is_bare {
                "Bare"
            } else if wt.is_detached {
                "Detached"
            } else {
                "Active"
            };
            table.add_row(vec![&wt.branch, &wt.commit, &wt.path, status]);
        }

        println!("{}", table);
    }

    pub fn render_feedback_prompt() {
        println!("\n{}", "━".repeat(60).cyan().dimmed());
        println!("{}", "Thank you for using the Worktree Manager.".bold());
        println!(
            "{}",
            "Feedback: https://github.com/justin-smith/worktrees/issues"
                .blue()
                .underline()
        );
    }

    pub fn render(state: AppState) {
        match state {
            AppState::Initializing { project_name } => {
                println!(
                    "{} {} [{} {}]",
                    "🚀".blue(),
                    format!("INITIALIZING BARE REPOSITORY: {}", project_name)
                        .blue()
                        .bold(),
                    "STATUS:".dimmed(),
                    "PREPARING".purple()
                );
            }
            AppState::Initialized { project_name } => {
                println!(
                    "\n{} {}",
                    "✅".green(),
                    "BARE REPOSITORY ESTABLISHED".green().bold()
                );
                println!(
                    "   {} {}",
                    "├─ Location:".dimmed(),
                    format!("{}/.bare", project_name).white()
                );
                println!(
                    "   {} {}",
                    "└─ Action:  ".dimmed(),
                    format!("cd {} && wt setup", project_name).blue().bold()
                );
            }
            AppState::AddingWorktree { intent, branch } => {
                println!(
                    "{} {} [{} {}]",
                    "📁".purple(),
                    format!("ADDING WORKTREE: {} (branch: {})", intent, branch)
                        .purple()
                        .bold(),
                    "STATUS:".dimmed(),
                    "CREATING".blue()
                );
            }
            AppState::WorktreeAdded { intent } => {
                println!(
                    "   {} {} {}",
                    "┗━".dimmed(),
                    "SUCCESS:".green().bold(),
                    format!("Worktree active at ./{}", intent).white()
                );
            }
            AppState::RemovingWorktree { intent } => {
                println!(
                    "{} {} [{} {}]",
                    "🔥".red(),
                    format!("REMOVING WORKTREE: {}", intent).red().bold(),
                    "STATUS:".dimmed(),
                    "DELETING".purple()
                );
            }
            AppState::WorktreeRemoved => {
                println!("   {} {}", "┗━".dimmed(), "WORKTREE REMOVED".green().bold());
            }
            AppState::Syncing { branch, .. } => {
                println!(
                    "{} {} [{} {}]",
                    "🔄".cyan(),
                    format!("SYNCING CONFIGURATIONS: {}", branch).cyan().bold(),
                    "STATUS:".dimmed(),
                    "SYNCHRONIZING".yellow()
                );
            }
            AppState::SyncComplete { branch, .. } => {
                println!(
                    "   {} {} {}",
                    "┗━".dimmed(),
                    "SUCCESS:".green().bold(),
                    format!("Synced configurations for {}", branch).white()
                );
            }
            AppState::Pushing { branch, .. } => {
                println!(
                    "{} {} [{} {}]",
                    "⬆".cyan(),
                    format!("PUSHING: {}", branch).cyan().bold(),
                    "STATUS:".dimmed(),
                    "UPLOADING".yellow()
                );
            }
            AppState::PushComplete { branch, .. } => {
                println!(
                    "   {} {} {}",
                    "┗━".dimmed(),
                    "SUCCESS:".green().bold(),
                    format!("Pushed {} to remote", branch).white()
                );
            }
            AppState::SelectingEditor { branch, .. } => {
                println!(
                    "{} {} [{} {}]",
                    "🔍".cyan(),
                    format!("SELECTING EDITOR FOR: {}", branch).cyan().bold(),
                    "STATUS:".dimmed(),
                    "PENDING SELECTION".yellow()
                );
            }
            AppState::OpeningEditor { branch, editor, .. } => {
                println!(
                    "{} {} [{} {}]",
                    "📝".yellow(),
                    format!("OPENING WORKTREE: {}", branch).yellow().bold(),
                    "EDITOR:".dimmed(),
                    editor.purple().bold()
                );
            }
            AppState::ListingWorktrees { worktrees, .. } => {
                // Fallback / Log view
                println!(
                    "{} {} [{} {}]",
                    "📋".blue(),
                    "ACTIVE WORKTREES".blue().bold(),
                    "TOTAL:".dimmed(),
                    worktrees.len().to_string().purple().bold()
                );
            }
            AppState::SettingUpDefaults => {
                println!(
                    "{} {}",
                    "⚡".purple(),
                    "SETTING UP DEFAULT WORKTREES".purple().bold()
                );
            }
            AppState::SetupComplete => {
                println!("\n{} {}", "🚀".blue(), "SETUP COMPLETE.".blue().bold());
                println!("   {}", "All default worktrees have been created.".dimmed());
            }
            AppState::Error(msg, _) => {
                eprintln!("\n{} {} {}", "❌".red(), "ERROR:".red().bold(), msg.red());
                eprintln!(
                    "   {} {}",
                    "└─".dimmed(),
                    "Check git state and permissions.".dimmed()
                );
            }
            AppState::Welcome
            | AppState::Confirming { .. }
            | AppState::Help { .. }
            | AppState::Fetching { .. }
            | AppState::Pulling { .. }
            | AppState::PullComplete { .. }
            | AppState::Prompting { .. }
            | AppState::ViewingStatus { .. }
            | AppState::ViewingHistory { .. }
            | AppState::SwitchingBranch { .. }
            | AppState::Committing { .. }
            | AppState::Timed { .. }
            | AppState::Exiting(_) => {
                // These are handled by render_tui, no-op for CLI log view
            }
        }
    }
}
