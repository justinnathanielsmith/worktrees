# CI Debugger Skill

## Description
Specialized skill for diagnosing and resolving CI/CD pipeline failures, specifically for Rust projects using GitHub Actions.

## Capabilities
- **Workflow Analysis**: Validates `.github/workflows` configuration, ensuring correct paths and triggers.
- **Environment Matching**: Replicates CI environments locally (e.g., matching `cargo clippy` flags).
- **Log Analysis**: Interprets CI failure logs to identify root causes (e.g., "exit code 128", "unused import").

## Procedural Rules

### 1. Locate Workflows
Always check for workflow files in the repository root: `<repo_root>/.github/workflows/`.
*Note: In "Bare Hub" or workspace structures, this might be the parent of the current working directory.*

### 2. Verify Working Directory
Ensure `working-directory` in YAML matches the actual project path.
- If code is in `worktree/`, `working-directory` must be `worktree`.
- If code is in root, it can be omitted or `.`.

### 3. Replicate CI Checks
When debugging lint/test failures, run the EXACT commands used in CI.
- **Clippy**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Format**: `cargo fmt --all -- --check`
- **Tests**: `cargo test`

### 4. Analyze "Exit Code 128"
This git error usually means the runner cannot find the git directory or lacks permissions.
- Check `actions/checkout` configuration.
- Verify `working-directory`.
- Ensure `.git` is accessible.

### 5. Check for Hidden Issues
If local passes but CI fails:
- Run `cargo clean` then `cargo clippy`.
- Check for unused imports (CI often treats warnings as errors).
- Verify Rust version compatibility.
