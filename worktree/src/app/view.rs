use crate::app::cli_renderer::CliRenderer;
use crate::app::event_handlers::{
    handle_branch_events, handle_committing_events, handle_confirm_events, handle_editor_events,
    handle_history_events, handle_listing_events, handle_picking_ref_events, handle_prompt_events,
    handle_status_events,
};
use crate::app::model::{AppState, RefreshType};
use crate::app::renderers::{
    render_branch_selection, render_commit_menu, render_editor_selection, render_history,
    render_listing, render_modals, render_prompt, render_status,
};
use crate::domain::repository::{ProjectRepository, RepositoryEvent, Worktree};
use crate::ui::widgets::{footer::FooterWidget, header::HeaderWidget};
use anyhow::Result;
use crossbeam_channel::Receiver;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
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

    fn render_background(
        f: &mut Frame,
        state: &AppState,
        context: crate::domain::repository::ProjectContext,
        area: ratatui::layout::Rect,
    ) {
        match state {
            AppState::ListingWorktrees {
                worktrees,
                table_state,
                dashboard,
                filter_query,
                is_filtering,
                ..
            } => {
                render_listing(
                    f,
                    worktrees.as_slice(),
                    &mut table_state.clone(),
                    context,
                    area,
                    dashboard.active_tab,
                    dashboard.cached_status.as_ref(),
                    dashboard.cached_history.as_deref(),
                    filter_query,
                    *is_filtering,
                );
            }
            AppState::ViewingStatus {
                branch,
                status,
                prev_state,
                ..
            } => {
                render_status(f, branch, status, prev_state, area);
            }
            _ => {}
        }
    }

    pub fn render_tui<R: ProjectRepository>(
        repo: &R,
        mut state: AppState,
    ) -> Result<Option<String>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut spinner_tick: usize = 0;
        let rx = repo.watch().ok();
        let res = Self::run_loop(&mut terminal, repo, &mut state, &mut spinner_tick, rx);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    }

    fn run_loop<R: ProjectRepository>(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        repo: &R,
        state: &mut AppState,
        spinner_tick: &mut usize,
        rx: Option<Receiver<RepositoryEvent>>,
    ) -> Result<Option<String>> {
        loop {
            // Handle repository events
            if let Some(ref rx) = rx {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        RepositoryEvent::RescanRequired
                        | RepositoryEvent::WorktreeListChanged
                        | RepositoryEvent::StatusChanged(_)
                        | RepositoryEvent::HeadChanged(_) => {
                            state.request_refresh();
                        }
                    }
                }
            }
            if let AppState::Timed {
                target_state,
                start_time,
                duration,
                ..
            } = state
                && start_time.elapsed() >= *duration
            {
                *state = *target_state.clone();
                state.request_refresh();
            }

            if let AppState::ListingWorktrees {
                refresh_needed,
                selection_mode,
                dashboard,
                table_state,
                worktrees,
                filter_query,
                is_filtering,
                ..
            } = state
            {
                if *refresh_needed == RefreshType::Full
                    && let Ok(new_worktrees) = repo.list_worktrees()
                {
                    let mut ts = table_state.clone();
                    if ts.selected().is_none() && !new_worktrees.is_empty() {
                        ts.select(Some(0));
                    }

                    let (status, history) =
                        Self::fetch_dashboard_data(repo, &new_worktrees, ts.selected(), dashboard);

                    *state = AppState::ListingWorktrees {
                        worktrees: new_worktrees,
                        table_state: ts,
                        refresh_needed: RefreshType::None,
                        selection_mode: *selection_mode,
                        dashboard: crate::app::model::DashboardState {
                            active_tab: dashboard.active_tab,
                            cached_status: status,
                            cached_history: history,
                        },
                        filter_query: filter_query.clone(),
                        is_filtering: *is_filtering,
                    };
                } else if *refresh_needed == RefreshType::Dashboard {
                    let (status, history) = Self::fetch_dashboard_data(
                        repo,
                        worktrees,
                        table_state.selected(),
                        dashboard,
                    );

                    *state = AppState::ListingWorktrees {
                        worktrees: worktrees.clone(),
                        table_state: table_state.clone(),
                        refresh_needed: RefreshType::None,
                        selection_mode: *selection_mode,
                        dashboard: crate::app::model::DashboardState {
                            active_tab: dashboard.active_tab,
                            cached_status: status,
                            cached_history: history,
                        },
                        filter_query: filter_query.clone(),
                        is_filtering: *is_filtering,
                    };
                }
            }

            terminal.draw(|f| Self::draw(f, repo, state, *spinner_tick))?;
            *spinner_tick = spinner_tick.wrapping_add(1);

            if event::poll(Duration::from_millis(100))? {
                let event = event::read()?;
                let mut new_state = None;
                let current_state_clone = state.clone();

                match state {
                    AppState::ListingWorktrees {
                        worktrees,
                        table_state,
                        ..
                    } => {
                        new_state = handle_listing_events(
                            &event,
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
                            &event,
                            repo,
                            path,
                            branch,
                            status,
                            prev_state,
                            &current_state_clone,
                        );
                    }
                    AppState::ViewingHistory {
                        branch: _,
                        commits,
                        selected_index,
                        prev_state,
                        ..
                    } => {
                        new_state =
                            handle_history_events(&event, commits, selected_index, prev_state);
                    }
                    AppState::SwitchingBranch {
                        path,
                        branches,
                        selected_index,
                        prev_state,
                        ..
                    } => {
                        new_state = handle_branch_events(
                            &event,
                            repo,
                            path,
                            branches,
                            selected_index,
                            prev_state,
                        );
                    }
                    AppState::PickingBaseRef {
                        branches,
                        selected_index,
                        prev_state,
                        ..
                    } => {
                        new_state =
                            handle_picking_ref_events(&event, branches, selected_index, prev_state);
                    }
                    AppState::SelectingEditor {
                        branch,
                        options,
                        selected,
                        prev_state,
                        ..
                    } => {
                        new_state = handle_editor_events(
                            &event, repo, branch, options, selected, prev_state,
                        );
                    }
                    AppState::Prompting {
                        prompt_type,
                        input,
                        prev_state,
                        ..
                    } => {
                        new_state = handle_prompt_events(
                            &event,
                            repo,
                            terminal,
                            prompt_type,
                            input,
                            prev_state,
                            spinner_tick,
                        )?;
                    }
                    AppState::Confirming {
                        action, prev_state, ..
                    } => {
                        new_state = handle_confirm_events(&event, repo, action, prev_state);
                    }
                    AppState::Committing {
                        path,
                        branch,
                        selected_index,
                        prev_state,
                        ..
                    } => {
                        new_state = handle_committing_events(
                            &event,
                            repo,
                            path,
                            branch,
                            selected_index,
                            prev_state,
                            &current_state_clone,
                        );
                    }
                    // Handle global exit if not handled by detailed handlers (or for states without handlers)
                    _ => {
                        if let Event::Key(key) = event
                            && let KeyCode::Char('q') | KeyCode::Esc = key.code
                        {
                            return Ok(None);
                        }
                    }
                }

                if let Some(ns) = new_state {
                    if let AppState::Exiting(res) = ns {
                        return Ok(res);
                    }
                    *state = ns;
                    state.request_refresh();
                }
            }
        }
    }

    fn fetch_dashboard_data<R: ProjectRepository>(
        repo: &R,
        worktrees: &[Worktree],
        selected_index: Option<usize>,
        dashboard: &crate::app::model::DashboardState,
    ) -> (
        Option<crate::domain::repository::GitStatus>,
        Option<Vec<crate::domain::repository::GitCommit>>,
    ) {
        let mut status = None;
        let mut history = None;

        if let Some(i) = selected_index
            && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
        {
            match dashboard.active_tab {
                crate::app::model::DashboardTab::Status => {
                    status = repo.get_status(&wt.path).ok();
                }
                crate::app::model::DashboardTab::Log => {
                    history = repo.get_history(&wt.path, 10).ok();
                }
                _ => {}
            }
        }

        (status, history)
    }

    pub fn draw<R: ProjectRepository>(
        f: &mut Frame,
        repo: &R,
        state: &mut AppState,
        spinner_tick: usize,
    ) {
        let display_state = if let AppState::Timed { inner_state, .. } = state {
            &mut **inner_state
        } else {
            state
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(5),    // Body (List + Dashboard)
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(f.area());

        let current_dir = std::env::current_dir().unwrap_or_default();
        let project_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("UNKNOWN")
            .to_string();

        let context = repo.detect_context(std::path::Path::new("."));
        f.render_widget(
            HeaderWidget {
                context,
                project_name,
                state: display_state,
            },
            chunks[0],
        );

        match display_state {
            AppState::ListingWorktrees {
                worktrees,
                table_state,
                dashboard,
                filter_query,
                is_filtering,
                ..
            } => {
                render_listing(
                    f,
                    worktrees.as_slice(),
                    table_state,
                    context,
                    chunks[1],
                    dashboard.active_tab,
                    dashboard.cached_status.as_ref(),
                    dashboard.cached_history.as_deref(),
                    filter_query,
                    *is_filtering,
                );
            }
            AppState::ViewingStatus {
                branch,
                status,
                prev_state,
                ..
            } => {
                render_status(f, branch, status, prev_state, chunks[1]);
            }
            AppState::ViewingHistory {
                branch,
                commits,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1]);
                render_history(f, branch, commits, *selected_index);
            }
            AppState::SwitchingBranch {
                branches,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1]);
                render_branch_selection(f, branches, *selected_index, None);
            }
            AppState::PickingBaseRef {
                branches,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1]);
                render_branch_selection(f, branches, *selected_index, Some("SELECT BASE BRANCH"));
            }
            AppState::SelectingEditor {
                branch,
                options,
                selected,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1]);
                render_editor_selection(f, branch, options, *selected);
            }
            AppState::Prompting {
                prompt_type,
                input,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1]);
                render_prompt(f, prompt_type, input);
            }
            AppState::Committing {
                branch,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1]);
                render_commit_menu(f, branch, *selected_index);
            }
            _ => {
                // Handle modals and everything else
                render_modals(f, repo, display_state, spinner_tick);
            }
        }

        f.render_widget(
            FooterWidget {
                state: display_state,
            },
            chunks[2],
        );
    }

    pub fn render(state: AppState) {
        CliRenderer::render(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::{GitCommit, GitStatus, ProjectContext};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::TableState;

    /// Minimal mock repository for testing view rendering
    struct MockRepository;

    impl ProjectRepository for MockRepository {
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
            Ok(vec![])
        }

        fn sync_configs(&self, _path: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn detect_context(&self, _base_path: &std::path::Path) -> ProjectContext {
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

        fn clean_worktrees(&self, _dry_run: bool, _artifacts: bool) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }

        fn get_project_root(&self) -> anyhow::Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from("/mock/root"))
        }

        fn convert_to_bare(
            &self,
            _name: Option<&str>,
            _branch: Option<&str>,
        ) -> anyhow::Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from("/mock/hub"))
        }

        fn migrate_to_bare(&self, _force: bool, _dry_run: bool) -> anyhow::Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from("/mock/migrated_hub"))
        }

        fn check_status(&self, _path: &std::path::Path) -> crate::domain::repository::RepoStatus {
            crate::domain::repository::RepoStatus::BareHub
        }

        fn watch(&self) -> anyhow::Result<Receiver<RepositoryEvent>> {
            let (_, rx) = crossbeam_channel::unbounded();
            Ok(rx)
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
        let content_str = content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        // Check for key welcome screen text
        let has_welcome_text =
            content_str.contains("NO REPOSITORY DETECTED") || content_str.contains("ONBOARDING");

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
                size_bytes: 1024,
                metadata: None,
            },
            Worktree {
                path: "/test/dev".to_string(),
                commit: "def456".to_string(),
                branch: "dev".to_string(),
                is_bare: false,
                is_detached: false,
                status_summary: Some("+2 ~1".to_string()),
                size_bytes: 2048,
                metadata: None,
            },
        ];

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut state = AppState::ListingWorktrees {
            worktrees,
            table_state,
            refresh_needed: RefreshType::None,
            selection_mode: false,
            dashboard: crate::app::model::DashboardState {
                active_tab: crate::app::model::DashboardTab::Info,
                cached_status: None,
                cached_history: None,
            },
            filter_query: String::new(),
            is_filtering: false,
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
        let content_str = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
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
        let mut state = AppState::Error("Test error message".to_string(), prev_state);

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0);
            })
            .unwrap();

        // Verify the terminal buffer contains error-related text
        let buffer = terminal.backend().buffer();
        let content_str = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(
            content_str.contains("ERROR") || content_str.contains("Test error"),
            "Error state should render error message"
        );
    }

    #[test]
    fn test_draw_timed_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let repo = MockRepository;

        let prev_state = Box::new(AppState::Welcome);
        let inner_state = Box::new(AppState::SyncComplete {
            branch: "test-branch".to_string(),
            prev_state: prev_state.clone(),
        });

        let mut state = AppState::Timed {
            inner_state,
            target_state: prev_state,
            start_time: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(800),
        };

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_str = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        // Check that SyncComplete text appears (rendered via Timed inner_state)
        assert!(
            content_str.contains("SYNC COMPLETE") || content_str.contains("test-branch"),
            "Timed state should render its inner state (SyncComplete)"
        );
    }
}
