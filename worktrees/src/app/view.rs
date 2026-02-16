use crate::app::cli_renderer::CliRenderer;
use crate::app::event_handlers::*;
use crate::app::model::{AppState, PromptType};
use crate::app::renderers::*;
use crate::domain::repository::{ProjectRepository, Worktree};
use crate::ui::widgets::{footer::FooterWidget, header::HeaderWidget};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::TableState,
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

pub struct View;

impl View {
    pub fn render_banner() {
        CliRenderer::render_banner();
    }

    pub fn render_json<T: serde::Serialize>(data: &T) -> Result<()> {
        CliRenderer::render_json(data)
    }

    pub fn render_listing_table(worktrees: &[Worktree]) {
        CliRenderer::render_listing_table(worktrees);
    }

    pub fn render_feedback_prompt() {
        CliRenderer::render_feedback_prompt();
    }

    pub fn render_tui<R: ProjectRepository>(repo: &R, mut state: AppState) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut spinner_tick: usize = 0;
        let res = Self::run_loop(&mut terminal, repo, &mut state, &mut spinner_tick);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        res
    }

    fn run_loop<R: ProjectRepository>(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        repo: &R,
        state: &mut AppState,
        spinner_tick: &mut usize,
    ) -> Result<()> {
        loop {
            if let AppState::ListingWorktrees {
                refresh_needed: true,
                ..
            } = state
            {
                if let Ok(worktrees) = repo.list_worktrees() {
                    let mut table_state = TableState::default();
                    if !worktrees.is_empty() {
                        table_state.select(Some(0));
                    }
                    *state = AppState::ListingWorktrees {
                        worktrees,
                        table_state,
                        refresh_needed: false,
                    };
                }
            }

            terminal.draw(|f| Self::draw(f, repo, state, *spinner_tick))?;
            *spinner_tick = spinner_tick.wrapping_add(1);

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    let mut new_state = None;
                    let current_state_clone = state.clone();
                    match state {
                        AppState::ListingWorktrees {
                            worktrees,
                            table_state,
                            ..
                        } => {
                            new_state = handle_listing_events(
                                key.code,
                                repo,
                                terminal,
                                worktrees,
                                table_state,
                                &current_state_clone,
                                spinner_tick,
                            )?;
                        }
                        AppState::ViewingStatus {
                            path,
                            branch,
                            status,
                            prev_state,
                            ..
                        } => {
                            new_state = handle_status_events(
                                key.code,
                                repo,
                                path,
                                branch,
                                status,
                                prev_state,
                                &current_state_clone,
                            )?;
                        }
                        AppState::ViewingHistory {
                            commits,
                            selected_index,
                            prev_state,
                            ..
                        } => {
                            new_state =
                                handle_history_events(key.code, commits, selected_index, prev_state)?;
                        }
                        AppState::SwitchingBranch {
                            path,
                            branches,
                            selected_index,
                            prev_state,
                        } => {
                            new_state = handle_branch_events(
                                key.code,
                                repo,
                                path,
                                branches,
                                selected_index,
                                prev_state,
                            )?;
                        }
                        AppState::Confirming {
                            action,
                            prev_state,
                            ..
                        } => {
                            new_state =
                                handle_confirm_events(key.code, repo, action, prev_state)?;
                        }
                        AppState::Committing {
                            path,
                            branch,
                            selected_index,
                            prev_state,
                        } => {
                            new_state = handle_committing_events(
                                key.code,
                                repo,
                                path,
                                branch,
                                selected_index,
                                prev_state,
                                &current_state_clone,
                            )?;
                        }
                        AppState::SelectingEditor {
                            branch,
                            options,
                            selected,
                            prev_state,
                        } => {
                            new_state = handle_editor_events(
                                key.code,
                                repo,
                                terminal,
                                branch,
                                options,
                                selected,
                                prev_state,
                                spinner_tick,
                            )?;
                        }
                        AppState::Syncing { prev_state, .. }
                        | AppState::SyncComplete { prev_state, .. }
                        | AppState::Help { prev_state }
                        | AppState::Fetching { prev_state, .. }
                        | AppState::Pushing { prev_state, .. }
                        | AppState::PushComplete { prev_state, .. }
                        | AppState::OpeningEditor { prev_state, .. }
                        | AppState::Error(_, prev_state) => {
                            if let KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter = key.code {
                                new_state = Some(*prev_state.clone());
                            }
                        }
                        AppState::Prompting {
                            prompt_type,
                            input,
                            prev_state,
                        } => {
                            new_state = handle_prompt_events(
                                key.code,
                                repo,
                                terminal,
                                prompt_type,
                                input,
                                prev_state,
                                spinner_tick,
                            )?;
                        }
                        AppState::Welcome => {
                            if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                                return Ok(());
                            }
                            if let KeyCode::Char('i') = key.code {
                                new_state = Some(AppState::Prompting {
                                    prompt_type: PromptType::InitUrl,
                                    input: String::new(),
                                    prev_state: Box::new(AppState::Welcome),
                                });
                            }
                        }
                        _ => {
                            if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                                return Ok(());
                            }
                        }
                    }
                    if let Some(ns) = new_state {
                        if matches!(ns, AppState::Exiting) {
                            return Ok(());
                        }
                        *state = ns;
                        state.request_refresh();
                    }
                }
            }
        }
    }


    pub fn draw<R: ProjectRepository>(f: &mut Frame, repo: &R, state: &mut AppState, spinner_tick: usize) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(5),    // Table
                    Constraint::Length(6), // Details
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(f.area());

        let context = repo.detect_context();
        f.render_widget(HeaderWidget { context }, chunks[0]);

        // Wrap chunks in Rc for sharing with renderers
        let shared_chunks = chunks.clone();

        match state {
            AppState::ListingWorktrees {
                worktrees,
                table_state,
                ..
            } => {
                render_listing(f, worktrees.as_slice(), table_state, context, shared_chunks.clone());
            }
            AppState::ViewingStatus {
                branch,
                status,
                prev_state,
                ..
            } => {
                render_status(f, branch, status, prev_state, shared_chunks.clone());
            }
            AppState::ViewingHistory {
                branch,
                commits,
                selected_index,
                prev_state,
                ..
            } => {
                // Background
                if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    ..
                } = &**prev_state
                {
                    render_listing(f, worktrees.as_slice(), &mut table_state.clone(), context, shared_chunks.clone());
                }
                render_history(f, branch, commits, *selected_index);
            }
            AppState::SwitchingBranch {
                branches,
                selected_index,
                prev_state,
                ..
            } => {
                // Background
                if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    ..
                } = &**prev_state
                {
                     render_listing(f, worktrees.as_slice(), &mut table_state.clone(), context, shared_chunks.clone());
                } else if let AppState::ViewingStatus {
                    branch,
                    status,
                    prev_state: p_prev,
                    ..
                } = &**prev_state
                {
                     render_status(f, branch, status, p_prev, shared_chunks.clone());
                }
                render_branch_selection(f, branches, *selected_index);
            }
            AppState::SelectingEditor {
                branch,
                options,
                selected,
                prev_state,
                ..
            } => {
               // Background
                if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    ..
                } = &**prev_state
                {
                     render_listing(f, worktrees.as_slice(), &mut table_state.clone(), context, shared_chunks.clone());
                }
                render_editor_selection(f, branch, options, *selected);
            }
            AppState::Prompting {
                prompt_type,
                input,
                prev_state,
                ..
            } => {
               // Background
                if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    ..
                } = &**prev_state
                {
                     render_listing(f, worktrees.as_slice(), &mut table_state.clone(), context, shared_chunks.clone());
                }
                render_prompt(f, prompt_type, input);
            }
            AppState::Committing {
                branch,
                selected_index,
                prev_state,
                ..
            } => {
                // Background
                 if let AppState::ViewingStatus {
                    branch: b,
                    status,
                    prev_state: p_prev,
                    ..
                } = &**prev_state
                {
                     render_status(f, b, status, p_prev, shared_chunks.clone());
                }
                render_commit_menu(f, branch, *selected_index);
            }
            _ => {
                // Handle modals and everything else
                render_modals(f, repo, state, spinner_tick);
            }
        }

        f.render_widget(FooterWidget, chunks[3]);
    }



    pub fn render(state: AppState) {
        CliRenderer::render(state);
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::{GitCommit, GitStatus, ProjectContext};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Minimal mock repository for testing view rendering
    struct MockRepository;

    impl ProjectRepository for MockRepository {
        fn init_bare_repo(&self, _url: &str, _project_name: &str) -> anyhow::Result<()> {
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
            Ok(vec![])
        }

        fn sync_configs(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn detect_context(&self) -> ProjectContext {
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
            Ok("feat: test commit".to_string())
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

        fn get_api_key(&self) -> anyhow::Result<Option<String>> {
            Ok(None)
        }

        fn set_api_key(&self, _key: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn clean_worktrees(&self, _dry_run: bool) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_draw_welcome_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let repo = MockRepository;
        let mut state = AppState::Welcome;

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0);
            })
            .unwrap();

        // Verify the terminal buffer contains expected text
        let buffer = terminal.backend().buffer();
        let content = buffer.content();
        
        // Check for key welcome screen text
        let content_str = content.iter().map(|c| c.symbol()).collect::<String>();
        
        // Check for key welcome screen text
        let has_welcome_text = content_str.contains("NO REPOSITORY DETECTED") || 
            content_str.contains("ONBOARDING");
        
        assert!(
            has_welcome_text,
            "Welcome state should render welcome screen text"
        );
    }

    #[test]
    fn test_draw_listing_worktrees_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let repo = MockRepository;
        
        let worktrees = vec![
            Worktree {
                path: "/test/main".to_string(),
                commit: "abc123".to_string(),
                branch: "main".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: Some("clean".to_string()),
            },
            Worktree {
                path: "/test/dev".to_string(),
                commit: "def456".to_string(),
                branch: "dev".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: Some("+2 ~1".to_string()),
            },
        ];

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut state = AppState::ListingWorktrees {
            worktrees,
            table_state,
            refresh_needed: false,
        };

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0);
            })
            .unwrap();

        // Verify no panic and the buffer is populated
        let buffer = terminal.backend().buffer();
        assert!(!buffer.content().is_empty());
        
        // Check that worktree information appears
        let content_str = buffer.content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        
        assert!(
            content_str.contains("main") || content_str.contains("dev"),
            "Listing state should render worktree names"
        );
    }

    #[test]
    fn test_draw_error_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let repo = MockRepository;
        
        let prev_state = Box::new(AppState::Welcome);
        let mut state = AppState::Error(
            "Test error message".to_string(),
            prev_state,
        );

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0);
            })
            .unwrap();

        // Verify the terminal buffer contains error-related text
        let buffer = terminal.backend().buffer();
        let content_str = buffer.content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        
        assert!(
            content_str.contains("ERROR") || content_str.contains("Test error"),
            "Error state should render error message"
        );
    }
}
