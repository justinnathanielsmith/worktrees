use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "worktrees")]
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
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new bare repository from a remote URL
    ///
    /// Example: worktrees init https://github.com/user/repo.git
    Init {
        /// URL of the remote repository (HTTPS or SSH)
        url: String,
        /// Directory name for the project (defaults to repo name)
        name: Option<String>,
    },
    /// Add a new worktree for a specific intent/feature
    ///
    /// Example: worktrees add feature-login
    Add {
        /// The name/intent of the worktree (e.g., dev, main, feat-x)
        intent: String,
        /// The branch to track (defaults to intent name)
        branch: Option<String>,
    },
    /// Remove an existing worktree and its associated files
    ///
    /// Example: worktrees remove feature-login
    Remove {
        /// The name of the worktree to remove
        intent: String,
    },
    /// List all active worktrees and their status
    List,
    /// Run a command in a temporary worktree and remove it afterward
    ///
    /// Example: worktrees run temp-check "cargo test"
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
    /// Example: worktrees sync feature-login
    Sync {
        /// The name of the worktree to sync (omit to sync all)
        intent: Option<String>,
    },
    /// Push changes to the remote repository
    ///
    /// Example: worktrees push feature-login
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
