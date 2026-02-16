# Worktree Manager

A professional CLI tool for managing Git worktrees using a bare repository architecture.

## Commands

- `init <url>`: Initialize a new bare repository.
- `setup`: Setup default worktrees (`main` and `dev`).
- `add <name> [branch]`: Create a new worktree.
- `remove <name>`: Remove a worktree.
- `list`: View and manage worktrees in an interactive interface.

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
cp target/release/worktrees /usr/local/bin/wt
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
