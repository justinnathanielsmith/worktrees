#[cfg(test)]
pub mod scaffolding {
    use super::super::model::AppState;
    use super::super::ports::ViewPort;
    use super::super::reducer::Reducer;
    use crate::domain::repository::{
        GitCommit, GitStatus, ProjectContext, ProjectRepository, RepoStatus, RepositoryEvent,
        StashEntry, Worktree,
    };
    use crossbeam_channel::Receiver;
    use miette::Result;
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    // --- 1. The Spy View ---
    // Captures everything sent to the UI
    #[derive(Clone, Default)]
    pub struct TestSpyView {
        pub states: Arc<Mutex<Vec<AppState>>>,
        pub jsons: Arc<Mutex<Vec<Value>>>,
        pub banners: Arc<Mutex<usize>>,
        pub listings: Arc<Mutex<Vec<Vec<Worktree>>>>,
        pub prompts: Arc<Mutex<usize>>,
    }

    impl ViewPort for TestSpyView {
        fn render(&self, state: AppState) {
            self.states.lock().unwrap().push(state);
        }

        fn render_json<T: serde::Serialize>(&self, data: &T) -> Result<()> {
            use miette::IntoDiagnostic;
            let json = serde_json::to_value(data).into_diagnostic()?;
            self.jsons.lock().unwrap().push(json);
            Ok(())
        }

        fn render_banner(&self) {
            *self.banners.lock().unwrap() += 1;
        }

        fn render_listing_table(&self, worktrees: &[Worktree]) {
            self.listings.lock().unwrap().push(worktrees.to_vec());
        }

        fn render_feedback_prompt(&self) {
            *self.prompts.lock().unwrap() += 1;
        }
    }

    // --- 2. The Mock Repo Builder ---
    // A fluent builder to easily script repo behavior
    #[derive(Clone, Default)]
    pub struct MockRepoBuilder {
        worktrees: Vec<Worktree>,
    }

    impl MockRepoBuilder {
        pub fn with_worktrees(mut self, worktrees: Vec<Worktree>) -> Self {
            self.worktrees = worktrees;
            self
        }

        pub fn build(self) -> MockRepo {
            MockRepo {
                worktrees: self.worktrees,
            }
        }
    }

    #[derive(Clone)]
    pub struct MockRepo {
        worktrees: Vec<Worktree>,
    }

    impl ProjectRepository for MockRepo {
        fn init_bare_repo(&self, _url: Option<&str>, _project_name: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn add_worktree(&self, _path: &str, _branch: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn add_new_worktree(&self, _path: &str, _branch: &str, _base: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove_worktree(&self, _path: &str, _force: bool) -> anyhow::Result<()> {
            Ok(())
        }
        fn list_worktrees(&self) -> anyhow::Result<Vec<Worktree>> {
            Ok(self.worktrees.clone())
        }
        fn sync_configs(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn detect_context(&self, _base_path: &Path) -> ProjectContext {
            ProjectContext::Standard
        }
        fn get_preferred_editor(&self) -> anyhow::Result<Option<String>> {
            Ok(None)
        }
        fn set_preferred_editor(&self, _editor: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn fetch(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn pull(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn push(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_status(&self, _path: &str) -> anyhow::Result<GitStatus> {
            Ok(GitStatus {
                staged: vec![],
                unstaged: vec![],
                untracked: vec![],
            })
        }
        fn stage_all(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn unstage_all(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn stage_file(&self, _path: &str, _file: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn unstage_file(&self, _path: &str, _file: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn commit(&self, _path: &str, _message: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_diff(&self, _path: &str) -> anyhow::Result<String> {
            Ok(String::new())
        }
        fn generate_commit_message(&self, _diff: &str, _branch: &str) -> anyhow::Result<String> {
            Ok("mock commit".into())
        }
        fn get_history(&self, _path: &str, _limit: usize) -> anyhow::Result<Vec<GitCommit>> {
            Ok(vec![])
        }
        fn list_branches(&self) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }
        fn switch_branch(&self, _path: &str, _branch: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn rebase(&self, _path: &str, _upstream: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_conflict_diff(&self, _path: &str) -> anyhow::Result<String> {
            Ok(String::new())
        }
        fn explain_rebase_conflict(&self, _diff: &str) -> anyhow::Result<String> {
            Ok("mock explanation".into())
        }
        fn get_api_key(&self) -> anyhow::Result<Option<String>> {
            Ok(None)
        }
        fn set_api_key(&self, _key: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn clean_worktrees(&self, _dry_run: bool, _artifacts: bool) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }
        fn get_project_root(&self) -> anyhow::Result<PathBuf> {
            Ok(PathBuf::from("/mock/root"))
        }
        fn convert_to_bare(
            &self,
            _name: Option<&str>,
            _branch: Option<&str>,
        ) -> anyhow::Result<PathBuf> {
            Ok(PathBuf::from("/mock/hub"))
        }
        fn migrate_to_bare(&self, _force: bool, _dry_run: bool) -> anyhow::Result<PathBuf> {
            Ok(PathBuf::from("/mock/migrated_hub"))
        }
        fn check_status(&self, _path: &Path) -> RepoStatus {
            RepoStatus::BareHub
        }
        fn watch(&self) -> anyhow::Result<Receiver<RepositoryEvent>> {
            let (_, rx) = crossbeam_channel::unbounded();
            Ok(rx)
        }
        fn list_stashes(&self, _path: &str) -> anyhow::Result<Vec<StashEntry>> {
            Ok(vec![])
        }
        fn apply_stash(&self, _path: &str, _index: usize) -> anyhow::Result<()> {
            Ok(())
        }
        fn pop_stash(&self, _path: &str, _index: usize) -> anyhow::Result<()> {
            Ok(())
        }
        fn drop_stash(&self, _path: &str, _index: usize) -> anyhow::Result<()> {
            Ok(())
        }
        fn stash_save(&self, _path: &str, _message: Option<&str>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    // --- 3. The Test Context ---
    // This holds the pieces together
    pub struct ReducerTestKit {
        pub reducer: Reducer<MockRepo, TestSpyView>,
        pub view: TestSpyView,
        pub repo: MockRepo,
    }

    impl Default for ReducerTestKit {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ReducerTestKit {
        pub fn new() -> Self {
            let repo = MockRepoBuilder::default().build();
            let view = TestSpyView::default();
            let reducer = Reducer::new_with_view(repo.clone(), view.clone(), false, false);

            Self {
                reducer,
                view,
                repo,
            }
        }

        pub fn with_repo(repo: MockRepo) -> Self {
            let view = TestSpyView::default();
            let reducer = Reducer::new_with_view(repo.clone(), view.clone(), false, false);
            Self {
                reducer,
                view,
                repo,
            }
        }

        /// Helper to assert the last state emitted matches a variant
        pub fn assert_last_state<F>(&self, matcher: F)
        where
            F: Fn(&AppState) -> bool,
        {
            let states = self.view.states.lock().unwrap();
            let last = states.last().expect("No states were emitted!");
            assert!(matcher(last), "Last state did not match expected pattern");
        }

        /// Check the sequence of states emitted
        pub fn assert_state_count(&self, count: usize) {
            assert_eq!(
                self.view.states.lock().unwrap().len(),
                count,
                "Unexpected number of states emitted"
            );
        }
    }
}
