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
- **Convert Existing Repos**: Turn standard git repositories into Bare Hubs with `worktree convert`.
- **Smart Branching**: Create worktrees from any branch or commit.
- **AI Integration**: Generate commit messages with Gemini 1.5 Flash.
- **Smart Cleanup**: reclaim disk space by purging build artifacts from inactive environments.
- **Cross-Platform**: Works on macOS, Linux, and Windows.

## Quick Start

### 1. Initialize a New Project

```bash
worktree init https://github.com/user/repo.git --name my-project
cd my-project
```

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

### 4. CLI Commands

- `worktree add <name> [branch]`: Create a new worktree.
- `worktree remove <name>`: Remove a worktree.
- `worktree switch <name>`: Switch to a worktree (outputs path).
- `worktree checkout <name> <branch>`: Switch a worktree to a different branch.
- `worktree config set-key <key>`: Set Gemini API key for AI features.
- `worktree clean [--artifacts] [--dry-run]`: Cleanup stale worktrees or purge build artifacts.
    - `--artifacts`: Removes `node_modules`, `target`, `build`, `dist`, `.gradle`, etc. from **inactive** worktrees.
    - Safety: It will never touch artifacts in your current working directory.

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

