use serial_test::serial;
use std::fs;
use tempfile::tempdir;
use worktree::infrastructure::warp_integration::{generate_warp_workflows, is_warp_terminal};

#[test]
fn test_generate_warp_workflows() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    generate_warp_workflows(project_path).expect("Failed to generate warp workflows");

    let workflow_dir = project_path.join(".warp").join("workflows");
    assert!(workflow_dir.exists(), "Warp workflows directory not created");

    let workflow_file = workflow_dir.join("worktrees.yaml");
    assert!(workflow_file.exists(), "Warp workflow file not created");

    let content = fs::read_to_string(workflow_file).expect("Failed to read workflow file");
    assert!(content.contains("name: Worktree Setup"));
    assert!(content.contains("command: worktree setup"));
    assert!(content.contains("name: Worktree Switch"));
}

#[test]
#[serial]
fn test_is_warp_terminal_positive() {
    // We need to run this in a separate process or ensure no other tests are running
    // but for now, we'll just set it and unset it carefully.
    // NOTE: This test might be flaky if run in parallel with other tests that rely on TERM_PROGRAM.
    // Ideally, we run `cargo test -- --test-threads=1` or use `serial_test`.

    // Set env var
    unsafe {
        std::env::set_var("TERM_PROGRAM", "WarpTerminal");
    }
    assert!(is_warp_terminal());

    // Clean up
    unsafe {
        std::env::remove_var("TERM_PROGRAM");
    }
}

#[test]
#[serial]
fn test_is_warp_terminal_negative() {
    unsafe {
        std::env::set_var("TERM_PROGRAM", "iTerm.app");
    }
    assert!(!is_warp_terminal());
    unsafe {
        std::env::remove_var("TERM_PROGRAM");
    }
}

#[test]
#[serial]
fn test_is_warp_terminal_missing() {
    unsafe {
        std::env::remove_var("TERM_PROGRAM");
    }
    assert!(!is_warp_terminal());
}
