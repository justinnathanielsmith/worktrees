use crate::app::cli_renderer::CliRenderer;
use crate::app::event_handlers::*;
use crate::app::model::AppState;
use crate::app::renderers::*;
use crate::domain::repository::{ProjectRepository, Worktree};
use crate::ui::widgets::{footer::FooterWidget, header::HeaderWidget};
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
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
        let res = Self::run_loop(&mut terminal, repo, &mut state, &mut spinner_tick);

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
    ) -> Result<Option<String>> {
        loop {
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
                refresh_needed: true,
                selection_mode,
                dashboard,
                table_state,
                ..
            } = state
                && let Ok(worktrees) = repo.list_worktrees()
            {
                let mut ts = table_state.clone();
                if ts.selected().is_none() && !worktrees.is_empty() {
                    ts.select(Some(0));
                }

                let (status, history) =
                    Self::fetch_dashboard_data(repo, &worktrees, ts.selected(), dashboard);

                *state = AppState::ListingWorktrees {
                    worktrees,
                    table_state: ts,
                    refresh_needed: false,
                    selection_mode: *selection_mode,
                    dashboard: crate::app::model::DashboardState {
                        active_tab: dashboard.active_tab,
                        cached_status: status,
                        cached_history: history,
                    },
                };
            }

            terminal.draw(|f| Self::draw(f, repo, state, *spinner_tick))?;
            *spinner_tick = spinner_tick.wrapping_add(1);

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    let mut new_state = None;
                    let current_state_clone = state.clone();
                    // ... key handling ...
                    match state {
                        // ... existing key matching ...
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
                            branch: _,
                            commits,
                            selected_index,
                            prev_state,
                            ..
                        } => {
                            new_state = handle_history_events(
                                key.code,
                                commits,
                                selected_index,
                                prev_state,
                            )?;
                        }
                        AppState::SwitchingBranch {
                            path,
                            branches,
                            selected_index,
                            prev_state,
                            ..
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
                        AppState::SelectingEditor {
                            branch,
                            options,
                            selected,
                            prev_state,
                            ..
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
                        AppState::Prompting {
                            prompt_type,
                            input,
                            prev_state,
                            ..
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
                        AppState::Confirming {
                            action, prev_state, ..
                        } => {
                            new_state = handle_confirm_events(key.code, repo, action, prev_state)?;
                        }
                        AppState::Committing {
                            path,
                            branch,
                            selected_index,
                            prev_state,
                            ..
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
                        // ... other states ...
                        _ => {
                            if let KeyCode::Char('q') | KeyCode::Esc = key.code {
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
                } else if let Event::Mouse(mouse) = event::read()?
                    && let MouseEventKind::Down(MouseButton::Left) = mouse.kind
                    && let AppState::ListingWorktrees {
                        worktrees,
                        table_state,
                        ..
                    } = state
                {
                    // Hardcoded layout assumption:
                    // Margin: 1
                    // Header: 3
                    // Table starts at y = 1 + 3 = 4
                    // Table header is 1 row
                    // Data starts at y = 5
                    let header_height = 4; // Margin + Header widget
                    let table_header = 1;
                    let data_start_y = header_height + table_header;

                    let row_index = mouse.row as i16 - data_start_y as i16;

                    if row_index >= 0 && (row_index as usize) < worktrees.len() {
                        table_state.select(Some(row_index as usize));
                        state.request_refresh();
                    }
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

        let context = repo.detect_context();
        f.render_widget(HeaderWidget { context }, chunks[0]);

        match display_state {
            AppState::ListingWorktrees {
                worktrees,
                table_state,
                dashboard,
                ..
            } => {
                render_listing(
                    f,
                    worktrees.as_slice(),
                    table_state,
                    context,
                    chunks[1],
                    dashboard.active_tab,
                    &dashboard.cached_status,
                    &dashboard.cached_history,
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
                // Background
                if let AppState::ListingWorktrees {
                    worktrees,
                    table_state,
                    dashboard,
                    ..
                } = &**prev_state
                {
                    render_listing(
                        f,
                        worktrees.as_slice(),
                        &mut table_state.clone(),
                        context,
                        chunks[1],
                        dashboard.active_tab,
                        &dashboard.cached_status,
                        &dashboard.cached_history,
                    );
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
                    dashboard,
                    ..
                } = &**prev_state
                {
                    render_listing(
                        f,
                        worktrees.as_slice(),
                        &mut table_state.clone(),
                        context,
                        chunks[1],
                        dashboard.active_tab,
                        &dashboard.cached_status,
                        &dashboard.cached_history,
                    );
                } else if let AppState::ViewingStatus {
                    branch,
                    status,
                    prev_state: p_prev,
                    ..
                } = &**prev_state
                {
                    render_status(f, branch, status, p_prev, chunks[1]);
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
                    dashboard,
                    ..
                } = &**prev_state
                {
                    render_listing(
                        f,
                        worktrees.as_slice(),
                        &mut table_state.clone(),
                        context,
                        chunks[1],
                        dashboard.active_tab,
                        &dashboard.cached_status,
                        &dashboard.cached_history,
                    );
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
                    dashboard,
                    ..
                } = &**prev_state
                {
                    render_listing(
                        f,
                        worktrees.as_slice(),
                        &mut table_state.clone(),
                        context,
                        chunks[1],
                        dashboard.active_tab,
                        &dashboard.cached_status,
                        &dashboard.cached_history,
                    );
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
                    render_status(f, b, status, p_prev, chunks[1]);
                }
                render_commit_menu(f, branch, *selected_index);
            }
            _ => {
                // Handle modals and everything else
                render_modals(f, repo, display_state, spinner_tick);
            }
        }

        f.render_widget(FooterWidget, chunks[2]);
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
            selection_mode: false,
            dashboard: crate::app::model::DashboardState {
                active_tab: crate::app::model::DashboardTab::Info,
                cached_status: None,
                cached_history: None,
            },
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
            .map(|c| c.symbol())
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
            .map(|c| c.symbol())
            .collect::<String>();

        // Check that SyncComplete text appears (rendered via Timed inner_state)
        assert!(
            content_str.contains("SYNC COMPLETE") || content_str.contains("test-branch"),
            "Timed state should render its inner state (SyncComplete)"
        );
    }
}
