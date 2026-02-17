use worktree::infrastructure::warp_integration::is_warp_terminal;

#[test]
fn test_is_warp_terminal_positive() {
    // We need to run this in a separate process or ensure no other tests are running
    // but for now, we'll just set it and unset it carefully.
    // NOTE: This test might be flaky if run in parallel with other tests that rely on TERM_PROGRAM.
    // Ideally, we run `cargo test -- --test-threads=1` or use `serial_test`.
    
    // Set env var
    unsafe { std::env::set_var("TERM_PROGRAM", "WarpTerminal"); }
    assert!(is_warp_terminal());
    
    // Clean up
    unsafe { std::env::remove_var("TERM_PROGRAM"); }
}

#[test]
fn test_is_warp_terminal_negative() {
    unsafe { std::env::set_var("TERM_PROGRAM", "iTerm.app"); }
    assert!(!is_warp_terminal());
    unsafe { std::env::remove_var("TERM_PROGRAM"); }
}

#[test]
fn test_is_warp_terminal_missing() {
    unsafe { std::env::remove_var("TERM_PROGRAM"); }
    assert!(!is_warp_terminal());
}
