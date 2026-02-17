# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-02-16

### Added
- **TUI Overhaul**: Introduced distinct interaction modes (Listing, History, Stash) with tab-based navigation.
- **Fuzzy Matching**: Implemented fuzzy search for worktree filtering using `fuzzy-matcher`.
- **Stash Management**: Full support for viewing, applying, popping, dropping, and saving stashes within the TUI.
- **Teleport Command**: New feature to move uncommitted changes between worktrees via automated stashing.
- **Warp Integration**: Conditional generation of Warp workflows for enhanced terminal integration.
- **Git Commit Graph**: Added a visual commit graph to the history view.
- **Async Operations**: Implemented async task handling for git operations to keep the TUI responsive.
- **Integration Tests**: Added comprehensive integration tests for Git operations and Warp integration.

### Changed
- **Architecture**: Refactored the application into a library crate for better testability and reuse.
- **UI Enhancements**: Improved worktree listing with dimming, spinners, and context-aware shortcuts.
- **README**: Overhauled documentation to focus on the Worktree Manager CLI/TUI features.

### Fixed
- **Git History Parsing**: Improved robustness of history parsing using null-byte delimiters.
- **Test Stability**: Fixed race conditions in integration tests using `serial_test`.

## [0.3.3] - 2025-XX-XX
- Initial release with basic worktree management.

[0.4.0]: https://github.com/justinnathanielsmith/worktrees/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/justinnathanielsmith/worktrees/releases/tag/v0.3.3
