# Worktree Manager

![Worktree Hero](public/worktrees_hero_logo_final.png)

> **The Stash Killer**: A high-performance CLI and TUI for managing Git worktrees using the Bare Hub Architecture.

[![Release](https://img.shields.io/github/v/release/justinnathanielsmith/worktrees)](https://github.com/justinnathanielsmith/worktrees/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Most developers rely on 'git stash' to temporarily store work, but stashing is destructive and leads to 'stash-amnesia.' **Worktree Manager** eliminates this by separating your Git metadata from your working directories, providing persistent, named environments for your code.

## 🚀 Key Features

- **Bare Hub Architecture**: Automatically manages '.bare' git dir and '.git' file pointer for a clean root.
- **Reactive TUI**: A high-performance interactive dashboard with real-time file system watching.
- **AI-Powered Commits**: Generate semantic commit messages using Gemini 1.5 Flash.
- **Warp-Native Integration**: 
    - **Warp Workflows**: Native command discovery via 'Ctrl+Shift+W'.
    - **Path Copying**: Instant context jumps with 'wt switch --copy'.
- **Smart Cleanup**: Reclaim gigabytes of disk space by purging build artifacts from inactive environments.
- **Context Teleportation**: Move uncommitted changes between worktrees instantly with 'worktree teleport <target>'.
- **Zero-Friction Migration**: Convert existing standard repositories to Bare Hubs in-place or as clones.
- **Cross-Platform**: Native binaries for macOS, Linux, and Windows.

---

## 📦 Installation

### macOS / Linux
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/justinnathanielsmith/worktrees/releases/latest/download/worktree-installer.sh | sh
```

### Windows (PowerShell)
```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/justinnathanielsmith/worktrees/releases/latest/download/worktree-installer.ps1 | iex"
```

### From Source
```bash
cargo install --git https://github.com/justinnathanielsmith/worktrees
```

---

## 🛠 Quick Start

### 1. Initialize a New Project
```bash
worktree init https://github.com/user/repo.git --name my-project --warp
cd my-project
```

### 2. The 'Stash Killer' Workflow
Need to fix a bug on 'main' while working on a feature? Don't stash. Just add a worktree:
```bash
worktree add hotfix-auth main
# Now cd into hotfix-auth, fix the bug, and your feature work stays untouched in its own folder.
```

### 3. Interactive TUI
Launch the dashboard to manage your entire project at a glance:
```bash
worktree
```

---

## 🏗 Bare Hub Architecture: The Flat Peer Structure

Unlike standard Git setups where worktrees are nested inside a main repo, **Worktree Manager** enforces a **Flat Peer Structure**. This keeps your root directory clean and prevents "nested repo" confusion.

```text
my-project/              <-- Project Root
├── .bare/               <-- The Git Engine (Metadata & Objects)
├── main/                <-- Primary Worktree (Active Branch)
├── develop/             <-- Secondary Worktree
└── feature-auth/        <-- Ephemeral Feature Branch
```

### Why It Works
1.  **Zero Clutter**: No `.git` folder in your worktrees (just a `.git` file pointer).
2.  **Instant Context Switching**: Jump between branches without stashing or rewriting files.
3.  **Parallel Workflows**: Run tests on `develop` while coding on `feature-auth`.

---

## ⌨️ CLI Command Reference

| Command             | Description                                                                     |
| :------------------ | :------------------------------------------------------------------------------ |
| `init <url>`        | Initialize a new bare repository (clones if provided).                          |
| `setup`             | Automatically create `main` and `dev` worktrees.                                |
| `add <name>`        | Create a new worktree tracking a branch or commit.                              |
| `switch <name>`     | Quick jump to a worktree (prints path for shell).                               |
| `list`              | Enter the interactive TUI (default command).                                    |
| `clean`             | Purge build artifacts (`node_modules`, `target`, etc.) from inactive worktrees. |
| `migrate`           | **In-place** conversion of a standard repo to Bare Hub.                         |
| `open`              | Generate Warp Launch Configurations for the project.                            |
| `run <name>`        | Execute a command in an isolated temporary sandbox.                             |
| `teleport <target>` | Move uncommitted changes to another worktree.                                   |
| `config`            | Securely store your Gemini API key (mode 0o600).                                |

---

## 🎨 Interactive TUI: Vibe Engineering

The TUI uses a **Modal System** to keep the interface clean and powerful.

### Normal Mode (Cyan)
* Default navigation and viewing mode.
- **`j` / `k`**: Navigate the list.
- **`Enter`**: Open selected worktree in editor.
- **`v`**: View detailed Git Status & Diff.
- **`l`**: View Commit History.
- **`m`**: Enter **Manage Mode**.
- **`g`**: Enter **Git Mode**.
- **`/`**: Enter **Filter Mode**.
- **`q`**: Quit.

### Manage Mode (Magenta)
* Operations on worktrees themselves.
- **`a`**: Add a new worktree.
- **`d`**: Delete selected worktree.
- **`D`**: Force delete selected worktree.
- **`c`**: Clean stale worktrees.
- **`C`**: Clean build artifacts from inactive worktrees.
- **`Esc`**: Return to Normal Mode.

### Git Mode (Green)
* Git operations on the selected worktree.
- **`p`**: Pull changes from remote.
- **`P`**: Push changes to remote.
- **`s`**: Sync configuration files.
- **`f`**: Fetch from remote.
- **`R`**: Rebase onto upstream.
- **`Esc`**: Return to Normal Mode.

### Filter Mode (Yellow)
* Rapidly find worktrees.
- **Type**: Filter by branch name or path.
- **`Enter`**: Select and return to Normal Mode.
- **`Esc`**: Clear filter and return to Normal Mode.

---

## 🏗 Contributing

We love contributions! The project is built with **Rust** (CLI/TUI) and **Astro** (Docs).

1. Fork the repo.
2. Create your feature branch ('git checkout -b feature/amazing-feature').
3. Commit your changes ('git commit -m "Add amazing feature"').
4. Push to the branch ('git push origin feature/amazing-feature').
5. Open a Pull Request.

---

## 📄 License

Distributed under the MIT License. See 'LICENSE' for more information.

---
Built with 🦀 by [Justin Smith](https://github.com/justinnathanielsmith)
