use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn generate_warp_workflows(project_path: &Path) -> Result<()> {
    let warp_dir = project_path.join(".warp").join("workflows");
    fs::create_dir_all(&warp_dir)
        .with_context(|| format!("Failed to create Warp directory: {warp_dir:?}"))?;

    let workflow_content = r"---
name: Worktree Setup
command: worktree setup
description: Setup canonical environment (main and dev worktrees)
author: Worktree Hub
---
name: Worktree Add
command: worktree add {intent}
description: Add a new worktree for a specific feature/intent
author: Worktree Hub
arguments:
  - name: intent
    description: The name/intent of the worktree (e.g., feature-xyz)
---
name: Worktree List
command: worktree list
description: List all active worktrees and their status
author: Worktree Hub
---
name: Worktree Push
command: worktree push {intent}
description: Push changes to the remote repository
author: Worktree Hub
arguments:
  - name: intent
    description: The name of the worktree to push
---
name: Worktree Switch
command: worktree switch {name}
description: Switch to a worktree by name (optimized for Warp)
author: Worktree Hub
arguments:
  - name: name
    description: The name of the worktree to switch to
";

    let workflow_path = warp_dir.join("worktrees.yaml");
    fs::write(&workflow_path, workflow_content)
        .with_context(|| format!("Failed to write Warp workflow file: {workflow_path:?}"))?;

    Ok(())
}
