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

## 📖 Documentation & Architecture

For a deep dive into the **Bare Hub Architecture** and how it transforms your development speed, visit our interactive guide:

👉 **[Live Documentation Guide](https://justinnathanielsmith.github.io/worktrees/)**

The documentation site explains:
- **The 'Engine' (.bare)**: How the bare repository centralizes history.
- **The 'Hub' (.git)**: How the pointer system keeps tools compatible.
- **Worktree Lifecycles**: Best practices for ephemeral vs. persistent environments.

---

## ⌨️ CLI Command Reference

| Command | Description |
| :--- | :--- |
| 'init <url>' | Initialize a new bare repository (clones if provided). |
| 'setup' | Automatically create 'main' and 'dev' worktrees. |
| 'add <name>' | Create a new worktree tracking a branch or commit. |
| 'switch <name>' | Quick jump to a worktree (prints path for shell). |
| 'list' | Enter the interactive TUI (default command). |
| 'clean' | Purge build artifacts ('node_modules', 'target', etc.) from inactive worktrees. |
| 'migrate' | **In-place** conversion of a standard repo to Bare Hub. |
| 'open' | Generate Warp Launch Configurations for the project. |
| 'run <name>' | Execute a command in an isolated temporary sandbox. |

---

## 🎨 Interactive TUI Hotkeys

While in the 'worktree' list view:
- **'G'**: Open the Git Status pane.
- **'Space'**: Toggle file staging.
- **'C'**: Open the Commit Menu (Manual or **AI Generated**).
- **'L'**: View Git history/log.
- **'B'**: Switch branches.
- **'O'**: Open current worktree in your default editor.
- **'Shift+C'**: Clean build artifacts from all inactive worktrees.

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
