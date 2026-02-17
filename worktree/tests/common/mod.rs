use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use worktree::infrastructure::git_repo::GitProjectRepository;

pub struct GitFixture {
    _temp_dir: TempDir,
    pub root_path: PathBuf,
    pub repo: GitProjectRepository,
}

impl GitFixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let root_path = temp_dir.path().to_path_buf();

        // 1. Initialize Bare Repo
        let bare_path = root_path.join(".bare");
        let output = Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(&bare_path)
            .current_dir(&root_path)
            .output()
            .expect("Failed to init bare repo");
        assert!(output.status.success(), "git init --bare failed");

        // 2. Create .git file redirect
        std::fs::write(root_path.join(".git"), "gitdir: ./.bare\n")
            .expect("Failed to write .git file");

        // 3. Set meaningful defaults for the test repo
        // This is crucial for environments where global git config might be missing (e.g., CI, clean containers)
        Self::run_git(&bare_path, &["config", "user.email", "test@example.com"]);
        Self::run_git(&bare_path, &["config", "user.name", "Test User"]);
        Self::run_git(&bare_path, &["symbolic-ref", "HEAD", "refs/heads/main"]);

        // 4. Create initial commit (empty) so 'main' branch exists
        // We need a temporary worktree to commit from because you can't commit directly to a bare repo easily without low-level plumbing
        let initial_worktree_path = root_path.join("initial_setup");
        Self::run_git(
            &bare_path,
            &[
                "worktree",
                "add",
                "-b",
                "main",
                initial_worktree_path.to_str().unwrap(),
            ],
        );

        Self::run_git(
            &initial_worktree_path,
            &["commit", "--allow-empty", "-m", "Initial commit"],
        );

        // Clean up the setup worktree
        Self::run_git(
            &bare_path,
            &[
                "worktree",
                "remove",
                "--force",
                initial_worktree_path.to_str().unwrap(),
            ],
        );

        Self {
            _temp_dir: temp_dir,
            root_path,
            repo: GitProjectRepository,
        }
    }

    pub fn run_git(path: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .expect("Failed to execute git command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Git command failed: {:?} \nError: {}", args, stderr);
        }

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Helper to create a new worktree *safely* within the fixture
    #[allow(dead_code)]
    pub fn create_worktree(&self, name: &str, branch: &str) {
        // We use the CLI's logic ideally, but for fixture setup we might want to just force it raw
        // for "given" state. However, let's use the repo trait to verify THAT logic primarily.
        // This method is for "arrange" phase if needed.
        let path = self.root_path.join(name);
        Self::run_git(
            &self.root_path,
            &[
                "worktree",
                "add",
                "-b",
                branch,
                path.to_str().unwrap(),
                "main",
            ],
        );
    }
}
