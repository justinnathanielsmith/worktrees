mod common;
use common::GitFixture;
use serial_test::serial;

use worktree::domain::repository::ProjectRepository;

#[test]
#[serial]
fn test_fixture_initialization() {
    let fixture = GitFixture::new();
    let root = &fixture.root_path;

    assert!(root.join(".bare").exists());
    assert!(root.join(".git").exists());
}

#[test]
#[serial]
fn test_create_worktree_via_repository() {
    // Arrange
    let fixture = GitFixture::new();
    let root = &fixture.root_path;
    let repo = &fixture.repo;

    // We must trick the repository into thinking it's inside the project root.
    // Since `GitProjectRepository` often derives paths relative to CWD or expects CWD to be root/worktree.
    // However, `GitProjectRepository::add_worktree` uses `get_project_root` which uses `git rev-parse`.
    // So as long as we run the test logic such that `add_worktree` can find the root, we are good.
    //
    // BUT: `add_worktree` takes a relative path `path` and joins it to `get_project_root()`.
    // `get_project_root()` runs `git rev-parse ...`.
    // If the test process CWD is not in the fixture, `git rev-parse` might fail or find the WRONG repo (the actual project repo!).
    //
    // CRITICAL: We must change directory to the fixture root for the duration of the test logic that relies on `GitProjectRepository`.
    // valid approach:
    std::env::set_current_dir(root).expect("Failed to set CWD to fixture root");

    // Act
    let branch_name = "feature-xyz";
    let wt_path = "feature-xyz";

    // Create a new branch from main
    repo.add_new_worktree(wt_path, branch_name, "main")
        .expect("Failed to add new worktree");

    // Assert
    let wt_dir = root.join(wt_path);
    assert!(wt_dir.exists(), "Worktree directory not created");
    assert!(
        wt_dir.join(".git").exists(),
        "Worktree .git file not created"
    );

    let output = GitFixture::run_git(&wt_dir, &["symbolic-ref", "--short", "HEAD"]);
    assert_eq!(output, branch_name);
}

#[test]
#[serial]
fn test_list_worktrees() {
    let fixture = GitFixture::new();
    let root = &fixture.root_path;
    let repo = &fixture.repo;
    std::env::set_current_dir(root).expect("Failed to set CWD");

    // "main" is created implicitly by our fixture setup (we commit to it),
    // but effectively we are in a "bare" state or "detached" state depending on how we set up.
    // Wait, fixture setup creates a commit on main but removes the worktree used to do it.
    // So currently 0 worktrees exist (only bare repo).
    // Let's create one.

    repo.add_worktree("main", "main")
        .expect("Failed to checkout main worktree");

    let worktrees = repo.list_worktrees().expect("Failed to list worktrees");
    
    // We expect 2 worktrees: the bare repo itself and the 'main' worktree we just added.
    // Use println! to debug if needed, use --nocapture to see it.
    println!("Worktrees found: {:?}", worktrees);
    
    assert_eq!(worktrees.len(), 2, "Expected bare repo + 1 worktree");
    
    let main_wt = worktrees.iter().find(|wt| wt.branch == "main");
    assert!(main_wt.is_some(), "Main worktree not found");
    assert!(main_wt.unwrap().path.ends_with("main"));
}
