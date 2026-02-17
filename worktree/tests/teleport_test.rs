mod common;
use common::GitFixture;
use serial_test::serial;
use std::fs;
use worktree::app::intent::Intent;
use worktree::app::reducer::Reducer;
use worktree::domain::repository::ProjectRepository;
use worktree::infrastructure::git_repo::GitProjectRepository;

#[tokio::test]
#[serial]
async fn test_teleport_changes_between_worktrees() {
    // 1. Arrange
    let fixture = GitFixture::new();
    let root = &fixture.root_path;
    let repo = GitProjectRepository;
    std::env::set_current_dir(root).expect("Failed to set CWD");

    // Create target worktree 'feat-a'
    repo.add_new_worktree("feat-a", "feat-a", "main").expect("Failed to create feat-a");
    
    // Create current worktree 'dev'
    repo.add_new_worktree("dev", "dev", "main").expect("Failed to create dev");
    
    // Switch to 'dev' worktree
    let dev_path = root.join("dev");
    std::env::set_current_dir(&dev_path).expect("Failed to enter dev worktree");

    // Make some uncommitted changes in 'dev'
    let test_file = dev_path.join("teleport_test.txt");
    fs::write(&test_file, "original content").expect("Failed to write test file");
    
    // Stage the file (optional, but let's test with staged changes too)
    repo.stage_all(&dev_path.to_string_lossy()).expect("Failed to stage changes");
    
    // Verify changes are in 'dev'
    let status_dev = repo.get_status(&dev_path.to_string_lossy()).expect("Failed to get status");
    assert!(!status_dev.staged.is_empty());

    // 2. Act
    let reducer = Reducer::new(repo.clone(), false, false);
    reducer.handle(Intent::Teleport { target: "feat-a".to_string() })
        .await
        .expect("Teleport failed");

    // 3. Assert
    // Verify 'dev' is clean (or at least the teleported file is gone from status)
    let status_dev_after = repo.get_status(&dev_path.to_string_lossy()).expect("Failed to get status");
    assert!(status_dev_after.staged.is_empty());
    assert!(status_dev_after.unstaged.is_empty());
    assert!(status_dev_after.untracked.is_empty());

    // Verify 'feat-a' has the changes
    let feat_a_path = root.join("feat-a");
    let status_feat_a = repo.get_status(&feat_a_path.to_string_lossy()).expect("Failed to get status");
    
    // Note: git stash apply may restore changes to either staged or unstaged depending on source state and git version.
    let found_staged = status_feat_a.staged.iter().any(|(f, _)| f == "teleport_test.txt");
    let found_unstaged = status_feat_a.unstaged.iter().any(|(f, _)| f == "teleport_test.txt");
    assert!(found_staged || found_unstaged, "File 'teleport_test.txt' not found in target worktree status. Status: {:?}", status_feat_a);
    
    let content = fs::read_to_string(feat_a_path.join("teleport_test.txt")).expect("Failed to read file in target");
    assert_eq!(content, "original content");
}
