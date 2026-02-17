# ARCHITECTURE.md - Technical Constraints & Ecosystem

## 1. The Bare Hub Architecture
In a **Bare Hub** workflow, the "Bare" repository is the central source of truth, with individual worktrees acting as functional directories.

### Flat Peer Structure
```text
my-project/              <-- Parent Directory
├── .bare/               <-- The actual Git repository (created with --bare)
├── main/                <-- Primary worktree (the "main" branch)
├── develop/             <-- Secondary worktree (the "dev" branch)
└── feature-xyz/         <-- Temporary worktree for a specific task
```

### Implementation Rules
1. **Absolute Paths**: Always use absolute paths derived from the project root.
2. **No Nesting**: Never nest worktrees inside other worktrees.
3. **.bare Directory**: Always use `.bare` for the git dir to avoid confusion with `.git`.

## 2. Technology Stack
- **Rust**: Powering the `worktree` CLI for Git automation.
- **Astro 4.x+**: Modern static site generator for the guide.
- **Tailwind CSS**: Utility-first CSS for styling.
- **TypeScript**: ensuring type safety.
- **Git**: The core technology.

## 3. Rust Coding Standards (Safety & Style)
- **Collapsible If**: Prefer `&&` (let-chains) to collapse nested `if let` blocks.
- **Pattern Matching**: Use `.is_some()` / `.is_none()` instead of `if let Some(_)` where possible.
- **Boxing**: Avoid `&Box<T>`, use `&T` instead.
- **Functions**: Occasional `#[allow(clippy::too_many_arguments)]` is acceptable for large UI/Event functions.

## 4. Verification Protocols
- **Filesystem**: Operations must be atomic where possible.
- **Validation**:
  - Run `cargo clippy --all-targets --all-features -- -D warnings` after changes.
  - Run `cargo fmt --all`.
  - Run `cargo test` for logic changes.
