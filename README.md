# 🗿 Giga Chad's Bare Worktree Guide

Welcome to the **Bare Hub Architecture**. This repository contains a high-performance, low-latency guide to mastering Git worktrees. Stop coding like a mid-tier dev and start decoupling your engine from your muscle.

## ⚡ The Philosophy

Standard Git setups bundle history with your workspace, creating a bottleneck. The Bare Hub separates these concerns:
- **The Engine (`.bare/`)**: Your Git database and history.
- **The Brain (`.git`)**: A pointer file directing the root to the engine.
- **The Muscle (`folders/`)**: Dedicated worktrees for your branches.

## 🚀 Execution Protocol

To initialize your environment with this architecture:

1. **Create The Engine**
   ```bash
   mkdir project && cd project
   git clone --bare <url> .bare
   ```

2. **Connect The Brain**
   ```bash
   echo "gitdir: ./.bare" > .git
   ```

3. **Spawn Muscle**
   ```bash
   git worktree add main
   git worktree add -b feature-scaling scaling
   ```

## 🧹 Gym Hygiene (Maintenance)

- **Remove Worktree**: `git worktree remove <name>`
- **Prune Ghosts**: `git fetch --prune`
- **Global Fetch**: `git fetch --all`

## 🎨 Visual Guide
Open `index.html` in your browser for a full neon-cyberpunk interactive breakdown of these metrics and structures.

---
*Stay Hydrated. Keep Grinding.*
