# Git Bare Hub Architecture Guide

> **Live Guide:** [https://justinnathanielsmith.github.io/worktrees/](https://justinnathanielsmith.github.io/worktrees/)

This repository contains the source for the **Git Bare Hub Architecture Guide**—a comprehensive resource for developers looking to optimize their Git workflow using bare repositories and worktrees.

## The Bare Hub Workflow

The Bare Hub architecture separates your Git metadata from your working directories, offering several benefits over standard cloning:

1.  **Instant Context Switching**: Create new branches in dedicated directories (worktrees) without stashing or rewriting your current workspace.
2.  **Parallel Development**: Run long-running tests or builds in one worktree while coding in another.
3.  **Clean Root Directory**: Keep your project root organized by isolating the main Git engine in a hidden `.bare` directory.
4.  **Isolated Dependencies**: Each worktree has its own `node_modules` or build artifacts, preventing conflicts when switching between branches with different dependency versions.

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

#### Worktrees CLI (Rust)
The repository also includes a Rust CLI tool for automating the Bare Hub setup, located in `worktrees/`.

```bash
cd worktrees
# Run the CLI
cargo run -- --help
```

### Production Build

Generate the static site:

```bash
npm run build
```

The output will be in the `dist/` directory.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

