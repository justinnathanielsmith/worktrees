# Release Manager Skill

## Description
Specialized skill for managing the versioning, changelog generation, and publishing of the `worktrees` CLI and Astro documentation to GitHub.

## Capabilities
- **Semantic Versioning**: Suggests the next version (Patch/Minor/Major) based on commit history.
- **Changelog Automation**: Extracts meaningful changes from Git logs to populate release notes.
- **Workflow Verification**: Ensures GitHub Action triggers in `.github/workflows/` are aligned with the new tag.

## Procedural Rules

### 1. Pre-Release Audit
Before tagging, you MUST verify the project is in a releasable state:
- Run the "Enforcer" audit: `cargo clippy` and `cargo fmt`.
- Run `cargo test` to ensure zero regressions in worktree logic.
- Check `Cargo.toml` in the `worktree/` directory to ensure the version string matches the intended release.

### 2. Versioning Protocol
- Use semantic versioning (vX.Y.Z).
- **Patch**: Bug fixes (e.g., fixing insecure file permissions).
- **Minor**: New features (e.g., adding TUI AppModes).
- **Major**: Breaking architectural changes to the "Bare Hub" structure.

### 3. Tagging & Publishing
- Do not execute Git tags directly without a confirmation artifact summarizing:
    - Current Version -> New Version.
    - Summary of "Gains" (Features) and "Fixes".
- Ensure the tag is pushed to the branch targeted by `release.yml` to trigger the automated binary build.

### 4. Post-Release Verification
- Verify the `deploy.yml` workflow triggers to update the "Git Bare Hub Architecture Guide" documentation.
