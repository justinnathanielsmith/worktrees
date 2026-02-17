# Documentation Implementation Plan

## Outdated Truths in Current Documentation

1.  **Architecture Description**: Reference to "Bare Hub Architecture" exists but lacks the explicit "Flat Peer Structure" emphasis and directory layout visualization (Parent -> .bare, main, develop).
2.  **TUI Navigation**: The current documentation describes a flat keybinding structure (e.g., 'G' for status, 'C' for commit) which is now obsolete due to the introduction of the **Modal System** (`AppMode`).
    -   Old: 'G' -> Status Pane. New: 'g' -> Enters Git Mode.
    -   Old: 'C' -> Commit Menu. New: 'c' in Manage Mode -> Clean Stale Worktrees.
    -   Old: 'B' -> Switch Branch. New: Removed/Changed? (Not seen in `listing.rs` Normal mode).
    -   Old: 'O' -> Open Editor. New: 'Enter' opens editor.
3.  **Missing Features**:
    -   **Vibe Engineering**: Visual improvements and modes are not mentioned.
    -   **Warp Integration details**: Specifics about deep linking and silent mode might be missing.
4.  **Security**: The 0o600 permission requirement for API keys is not explicitly stated in the public docs.

## File Modification Plan

### 1. Root `README.md`
-   **Section: "Bare Hub Architecture"**: Add the "Flat Peer Structure" visualization.
-   **Section: "Interactive TUI Hotkeys"**: Rewrite completely to explain the **Modal System** (Normal, Manage, Git, Filter).
-   **Section: "Quick Start"**: Ensure strict `0o600` mention for API key storage or at least "Securely stored".

### 2. `AGENTS.md`
-   **Identity**: Update to reflect "Vibe Engineering" standards.
-   **Protocols**: Add "Mode-Aware UI" constraints.

### 3. `.gemini/SKILLS.md`
-   **Verification**: Ensure `Release Manager` is correctly linked and described.

### 4. `src/components/CliDocs.astro`
-   **TUI Section**: Rewrite to feature the Modal Architecture.
    -   Group hotkeys by Mode (Normal, Manage, Git).
    -   Update incompatible legacy keys.

## Verification Strategy
-   **Logic Check**: Compare documented keys against `worktree/src/app/event_handlers/listing.rs`.
-   **Constraint Check**: Ensure "Flat Peer Structure" is described using the exact ASCII art from the `MEMORY[user_global]`.
