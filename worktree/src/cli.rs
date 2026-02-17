use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "worktree")]
#[command(
    version,
    about = "Giga Chad's Bare Repository Worktree Manager",
    long_about = "A high-performance CLI tool for managing Git worktrees in a Bare Repository architecture. Optimized for parallel development workflows."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output in JSON format for machine readability
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress all informational output (Silent Mode)
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new bare repository from a remote URL
    ///
    /// Example: worktree init <https://github.com/user/repo.git>
    Init {
        /// URL of the remote repository to clone (optional)
        url: Option<String>,
        /// Directory name for the project (defaults to repo name or 'project')
        #[arg(short, long)]
        name: Option<String>,
        /// Generate Warp Workflows (.warp/workflows/worktrees.yaml)
        #[arg(long)]
        warp: bool,
    },
    /// Add a new worktree for a specific intent/feature
    ///
    /// Example: worktree add feature-login
    Add {
        /// The name/intent of the worktree (e.g., dev, main, feat-x)
        intent: String,
        /// The branch to track (defaults to intent name)
        branch: Option<String>,
    },
    /// Remove an existing worktree and its associated files
    ///
    /// Example: worktree remove feature-login
    Remove {
        /// The name of the worktree to remove
        intent: String,
        /// Force removal even if the worktree has uncommitted changes
        #[arg(short, long)]
        force: bool,
    },
    /// List all active worktrees and their status
    List,
    /// Run a command in a temporary worktree and remove it afterward
    ///
    /// Example: worktree run temp-check "cargo test"
    Run {
        /// The name of the temporary worktree
        intent: String,
        /// The branch to track (defaults to intent name)
        #[arg(long, short)]
        branch: Option<String>,
        /// The command to execute in the worktree
        #[arg(required = true, num_args = 1..)]
        command: Vec<String>,
    },
    /// Synchronize configuration files to a specific worktree or all worktrees
    ///
    /// Example: worktree sync feature-login
    Sync {
        /// The name of the worktree to sync (omit to sync all)
        intent: Option<String>,
    },
    /// Push changes to the remote repository
    ///
    /// Example: worktree push feature-login
    Push {
        /// The name of the worktree to push (defaults to current directory if valid worktree, or fails)
        intent: Option<String>,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Setup the canonical environment with 'main' and 'dev' worktrees
    ///
    /// This is the recommended first step after 'init'.
    Setup,
    /// Clean up stale worktrees (directories with missing metadata or deleted branches)
    ///
    /// Example: worktree clean --dry-run
    Clean {
        /// Show what would be deleted without actually removing anything
        #[arg(long)]
        dry_run: bool,
        /// Remove build artifacts (`node_modules`, target, build, etc.) from inactive worktrees
        #[arg(long)]
        artifacts: bool,
    },
    /// Switch to a worktree by name (prints path to stdout for shell integration)
    ///
    /// Example: cd $(worktree switch dev)
    Switch {
        /// The name or branch of the worktree to switch to
        name: Option<String>,
        /// Copy the worktree path to the clipboard (Warp-optimized)
        #[arg(short, long)]
        copy: bool,
    },
    /// Convert an existing standard repository into a bare hub structure
    ///
    /// This will move the .git folder to .bare and create a new hub directory.
    ///
    /// Example: worktree convert --name my-project-hub
    Convert {
        /// Optional name for the new hub directory (defaults to {project}-hub)
        #[arg(short, long)]
        name: Option<String>,
        /// Optional main branch name (defaults to current branch)
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// In-place migration of a standard repository to a Bare Hub structure
    ///
    /// This safely converts the current repository into a Bare Hub without moving the root folder.
    /// All existing files are moved into a new worktree for the current branch.
    ///
    /// Example: worktree migrate
    Migrate {
        /// Force migration even if potential issues are detected
        #[arg(short, long)]
        force: bool,
        /// Perform a dry-run to see what would happen
        #[arg(long)]
        dry_run: bool,
    },
    /// Switch a worktree to a different branch
    ///
    /// Example: worktree checkout feature-login develop
    Checkout {
        /// The name or intent of the worktree
        intent: String,
        /// The branch to switch to
        branch: String,
    },
    /// Generate shell completions
    ///
    /// Example: worktree completions zsh > _worktree
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Open the current project in Warp with a grid of standard worktrees
    ///
    /// Example: worktree open
    Open,
    /// Rebase the current worktree onto an upstream branch
    ///
    /// Example: worktree rebase main
    Rebase {
        /// The upstream branch to rebase onto (defaults to 'main')
        upstream: Option<String>,
    },
    /// Move uncommitted changes (patch) from current worktree to another
    ///
    /// Example: worktree teleport feature-xyz
    Teleport {
        /// The name or branch of the target worktree
        target: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set the Gemini API key
    SetKey {
        /// The API key to use
        key: String,
    },
    /// Get the current Gemini API key
    GetKey,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_cli_parsing() -> Result<()> {
        // Test init
        let cli = Cli::try_parse_from(["worktree", "init", "url"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Init { url, name, warp } => {
                assert_eq!(url, Some("url".to_string()));
                assert_eq!(name, None);
                assert!(!warp);
            }
            _ => anyhow::bail!("Expected Init"),
        }

        // Test add
        let cli = Cli::try_parse_from(["worktree", "add", "feat", "branch"])
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
        let cli = Cli::try_parse_from(["worktree", "list"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        assert!(matches!(
            cli.command
                .ok_or_else(|| anyhow::anyhow!("Missing command"))?,
            Commands::List
        ));
        Ok(())
    }

    #[test]
    fn test_cli_parsing_clean() -> Result<()> {
        // Test clean with dry-run
        let cli = Cli::try_parse_from(["worktree", "clean", "--dry-run"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Clean {
                dry_run,
                artifacts: _,
            } => {
                assert!(dry_run);
            }
            _ => anyhow::bail!("Expected Clean"),
        }

        // Test clean without dry-run
        let cli = Cli::try_parse_from(["worktree", "clean"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Clean {
                dry_run,
                artifacts: _,
            } => {
                assert!(!dry_run);
            }
            _ => anyhow::bail!("Expected Clean"),
        }

        // Test clean with artifacts
        let cli = Cli::try_parse_from(["worktree", "clean", "--artifacts"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Clean { dry_run, artifacts } => {
                assert!(!dry_run);
                assert!(artifacts);
            }
            _ => anyhow::bail!("Expected Clean"),
        }

        Ok(())
    }

    #[test]
    fn test_cli_parsing_switch() -> Result<()> {
        let cli = Cli::try_parse_from(["worktree", "switch", "dev"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Switch { name, copy } => {
                assert_eq!(name, Some("dev".to_string()));
                assert!(!copy);
            }
            _ => anyhow::bail!("Expected Switch"),
        }

        let cli = Cli::try_parse_from(["worktree", "switch", "dev", "--copy"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Switch { name, copy } => {
                assert_eq!(name, Some("dev".to_string()));
                assert!(copy);
            }
            _ => anyhow::bail!("Expected Switch"),
        }
        Ok(())
    }

    #[test]
    fn test_cli_parsing_checkout() -> Result<()> {
        let cli = Cli::try_parse_from(["worktree", "checkout", "feat-a", "main"])
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        match cli
            .command
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?
        {
            Commands::Checkout { intent, branch } => {
                assert_eq!(intent, "feat-a");
                assert_eq!(branch, "main");
            }
            _ => anyhow::bail!("Expected Checkout"),
        }
        Ok(())
    }
}
