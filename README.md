# Git Bare Hub Architecture Guide

> **Live Guide:** [https://justinnathanielsmith.github.io/worktrees/](https://justinnathanielsmith.github.io/worktrees/)

This repository contains the source for the **Git Bare Hub Architecture Guide**—a comprehensive resource for developers looking to optimize their Git workflow using bare repositories and worktrees.

## The Bare Hub Workflow: The Stash Killer

The Bare Hub architecture is the **Stash Killer**. Most developers rely on `git stash` to temporarily store work, but stashing is destructive and leads to "stash-amnesia." 

By separating your Git metadata from your working directories, Bare Hub provides **persistent, named environments** for your code:

1.  **Eliminate Stash Amnesia**: Your uncommitted changes stay in their own physical directory, exactly where you left them. No more "popping" errors or lost context.
2.  **Instant Context Switching**: Move between tasks as fast as a `cd` command. No more waiting for Git to rewrite files or your IDE to re-index.
3.  **Parallel Development**: Run long-running tests or builds on `main` while simultaneously coding a hotfix in another worktree.
4.  **Isolated Dependencies**: Each worktree maintains its own `node_modules` and build artifacts, preventing version conflicts when jumping between branches.
5.  **Clean Root Architecture**: Isolate the main Git engine in a hidden `.bare` directory, keeping your project root organized and clutter-free.

## Project Architecture

This interactive guide explains the core concepts:

1.  **The "Engine" (.bare)**: The bare repository holding all Git history and refs.
2.  **The "Hub" (.git)**: A pointer file connecting your root to the engine.
3.  **Worktrees**: Dedicated directories for each active branch (e.g., `main`, `feature-x`).

## Interactive Guide Source

This site is built with **Astro 4.x+** and **Tailwind CSS**.

### Local Development

To run the guide locally:

#### Astro Site
```bash
# Clone the repository
git clone https://github.com/justinnathanielsmith/worktrees.git
cd worktrees

# Install dependencies
npm install

# Start the dev server
npm run dev
```

#### Installation

The `worktree` CLI can be installed quickly on macOS, Linux, and Windows.

**macOS/Linux (Shell):**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/justinnathanielsmith/worktrees/releases/latest/download/worktree-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/justinnathanielsmith/worktrees/releases/latest/download/worktree-installer.ps1 | iex"
```

**From Source (Cargo):**
```bash
cargo install --git https://github.com/justinnathanielsmith/worktrees
```

### Key Features (v0.2.0)

- **Bare Hub Architecture**: Automatically manages `.bare` git dir and `.git` file pointer.
- **Reactive TUI**: Real-time updates when files or branches change.
- **Convert & Migrate**: Turn standard git repositories into Bare Hubs (new or in-place).
- **Smart Branching**: Create worktrees from any branch or commit.
- **AI Integration**: Generate commit messages with Gemini 1.5 Flash.
- **Smart Cleanup**: reclaim disk space by purging build artifacts from inactive environments.
- **Warp-Native Integration**: 
    - **Warp Workflows**: Generate logical command groups with `wt init --warp`.
    - **Warp Blocks**: Clean output borders and semantic spacing for perfect block containment and context-aware "Ask AI".
    - **Path Copying**: Use `wt switch <name> --copy` to instantly jump to worktrees via clipboard.
- **Shell Completions**: Rich completions for Zsh, Bash, Fish, and PowerShell (highly recommended for Warp).
- **Cross-Platform**: Works on macOS, Linux, and Windows.

## Quick Start

### 1. Initialize a New Project

```bash
worktree init https://github.com/user/repo.git --name my-project --warp
cd my-project
```
> [!TIP]
> Using the `--warp` flag at initialization creates `.warp/workflows/worktrees.yaml`, making your Bare Hub operations discoverable in Warp's `Ctrl+Shift+W` menu.

### 2. Convert an Existing Repository

```bash
cd my-existing-repo
worktree convert
```

### 3. Open the TUI

```bash
worktree list
# or just
worktree
```

### 4. CLI Commands Reference

- `worktree init <url> [--name <name>] [--warp]`: Initialize a new bare repository. Use `--warp` to generate native workflows.
- `worktree setup`: Opinionated setup; creates `main` and `dev` worktrees automatically.
- `worktree add <name> [branch]`: Create a new worktree tracking a branch.
- `worktree remove <name> [--force]`: Remove a worktree and its directory.
- `worktree switch <name>`: Quick jump to a worktree (prints path for shell integration).
- `worktree checkout <name> <branch>`: Switch an existing worktree to a different branch.
- `worktree open`: Generate and display a Warp Launch Configuration for the project.
- `worktree list`: Launch the interactive Terminal User Interface.
- `worktree run <name> <command>`: Execute a command in a temporary sandbox.
- `worktree sync [name]`: Synchronize configuration files to worktrees.
- `worktree push [name]`: Push changes from a specific worktree to remote.
- `worktree config set-key <key>`: Securely store your Gemini API key.
- `worktree config get-key`: Retrieve the stored Gemini API key.
- `worktree convert [--name <name>] [--branch <branch>]`: Create a *new* Bare Hub from an existing repo (sibling directory).
- `worktree migrate [--force] [--dry-run]`: **In-place** migration of the current repo to a Bare Hub structure.
- `worktree clean [--artifacts] [--dry-run]`: Cleanup stale worktrees or heavy build artifacts.
- `worktree completions <shell>`: Generate completion scripts for `zsh`, `bash`, `fish`, or `powershell`.

## Shell Completions

Enable rich command-line suggestions and descriptions in your terminal. This is highly recommended for Warp users.

### Zsh (Recommended)
Add this to your `~/.zshrc`:
```bash
if command -v worktree &>/dev/null; then
    source <(worktree completions zsh)
fi
```

### Bash
Add this to your `~/.bashrc`:
```bash
if command -v worktree &>/dev/null; then
    source <(worktree completions bash)
fi
```

### Fish
```bash
worktree completions fish > ~/.config/fish/completions/worktree.fish
```

### TUI Hotkeys (Worktree List)

- **`c`**: **Prune** stale metadata (cleans up legacy references in `.bare/worktrees`).
- **`Shift+C`**: **Clean** build artifacts from all worktrees except the one you are currently in. Requires confirmation.


### Local Development

Generate the static site:

```bash
npm run build
```

The output will be in the `dist/` directory.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

