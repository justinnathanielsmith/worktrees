# Worktree Manager

A professional CLI tool for managing Git worktrees using a bare repository architecture.

## Quick Start

Get up and running with a high-performance bare repository workflow in minutes.

### 1. Initialize a Project

You can initialize a new project from an existing remote repository or start fresh.

**From a Remote URL:**
```bash
# Creates a new directory 'repo' with the bare repository
worktree init https://github.com/username/repo.git
cd repo
```

**Start Fresh:**
```bash
# Creates a new directory 'my-project'
worktree init --name my-project
cd my-project
```

### 2. Setup Default Worktrees

Create your standard `main` and `dev` worktrees automatically:

```bash
worktree setup
```

This will create:
- `./main`: Your stable production branch.
- `./dev`: Your integration branch (created from main if it doesn't exist).

### 3. Create a Feature Worktree

Start working on a new feature in isolation without switching contexts:

```bash
worktree add feature-login
```

This creates a new directory `./feature-login` tracking the `feature-login` branch.

### 4. Switch Contexts Efficiently

To jump between worktrees quickly, you can use the `switch` command.

> **Pro Tip:** Add this function to your `.zshrc` or `.bashrc` for instant navigation:

```bash
function worktree-switch() {
  cd "$(worktree switch "$1")"
}
```

Now you can run:
```bash
worktree-switch dev
worktree-switch feature-login
```

## Commands

- `init [url] [--name <name>]`: Initialize a new bare repository (clones if provided).
- `setup`: Automatically create `main` and `dev` worktrees.
- `add <name> [branch]`: Create a new worktree for a feature or intent.
- `remove <name> [--force]`: Delete a worktree directory and unregister it.
- `switch <name>`: Quick jump between workspaces.
- `checkout <intent> <branch>`: Reassign a worktree to a different branch.
- `list`: Enter the high-performance interactive TUI.
- `run <name> <command>`: execute a command in an isolated temporary worktree.
- `sync [name]`: Push configuration files to all or specific worktrees.
- `push [name]`: Push changes from a workspace to origin.
- `clean [--artifacts] [--dry-run]`: Reclaim disk space by purging build artifacts.
- `config set-key <key>`: Store your Gemini API key in the system keyring.
- `config get-key`: View your current Gemini API key.
- `convert [--name <name>] [--branch <branch>]`: Migrate a standard repo to Bare Hub.
- `completions <shell>`: Generate command-line completion scripts.

## Shell Completions

Optimized for **Warp**, Zsh, and Bash. Shell completions provide instant flag descriptions and branch name suggestions.

### Zsh
Add the following to your `~/.zshrc`:
```bash
source <(worktree completions zsh)
```

### Bash
Add the following to your `~/.bashrc`:
```bash
source <(worktree completions bash)
```

### Fish
```bash
worktree completions fish > ~/.config/fish/completions/worktree.fish
```

### Interactive TUI Features

The `list` command (and running without arguments) opens a high-performance interactive interface:

- **Git Status**: Press `G` to view a detailed status pane.
- **Staging**: In Status view, use `Space` to toggle files or `A` to stage all.
- **Commit Menu**: Press `C` in Status view to open the commit menu.
    - **Manual**: Enter your own commit message.
    - **AI Generation**: Automatically generate a conventional commit message using Gemini 1.5 Flash based on your staged changes.
- **Commit Log**: Press `L` to view the worktree's commit history.
- **Branch Switching**: Press `B` to switch branches.
- **Remote Fetch**: Press `F` to fetch updates from origin.
- **API Key**: Press `P` in the main list to securely set your Gemini API key.
- **Configuration Sync**: Press `S` to sync configuration files.
- **Editor Integration**: Press `O` to open the worktree in your preferred editor.

## Architecture

Built with Rust for performance and reliability.
- **Bare Repository**: Keeps your project organized by separating the Git history from your active work.
- **Worktrees**: Allows multiple branches to be checked out simultaneously in different directories.

## Development

To run the manager during development:

```bash
cargo run -- <command> [args]
```

Example:
```bash
cargo run -- --help
cargo run -- list
```

## Installation

```bash
cargo build --release
cp target/release/worktree /usr/local/bin/worktree
```

## Quality Control

The project uses several tools to ensure high-performance code quality:

### Linting & Formatting
- **rustfmt**: Ensures consistent code style.
  ```bash
  make fmt
  ```
- **clippy**: Catch common mistakes and improve your Rust code.
  ```bash
  make clippy
  ```

### Testing
- **Unit Tests**:
  ```bash
  make test
  ```
- **Code Coverage**: Uses `cargo-tarpaulin`.
  ```bash
  make coverage
  ```

### Continuous Integration
A GitHub Action is configured to run these checks on every push and pull request.
