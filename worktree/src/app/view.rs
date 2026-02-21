use crate::app::async_tasks::AsyncResult;
use crate::app::cli_renderer::CliRenderer;
use crate::app::event_handlers::helpers::create_timed_state;
use crate::app::event_handlers::{
    handle_branch_events, handle_committing_events, handle_confirm_events, handle_editor_events,
    handle_history_events, handle_listing_events, handle_picking_ref_events, handle_prompt_events,
    handle_stash_events, handle_status_events,
};
use crate::app::model::{AppState, RefreshType};
use crate::app::renderers::{
    render_branch_selection, render_commit_menu, render_editor_selection, render_history,
    render_listing, render_modals, render_prompt, render_status,
};
use crate::domain::repository::{ProjectRepository, RepositoryEvent, Worktree};
use crate::ui::widgets::{footer::FooterWidget, header::HeaderWidget, stash_list::StashListWidget};
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
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub struct RenderContext {
    pub project_name: String,
    pub context: crate::domain::repository::ProjectContext,
}

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
        spinner_tick: usize,
    ) {
        match state {
            AppState::ListingWorktrees {
                worktrees,
                filtered_worktrees,
                table_state,
                dashboard,
                filter_query,
                is_filtering,
                mode,
                ..
            } => {
                render_listing(
                    f,
                    worktrees.as_slice(),
                    filtered_worktrees.as_slice(),
                    &mut table_state.clone(),
                    context,
                    area,
                    dashboard.active_tab,
                    dashboard.cached_status.as_ref(),
                    dashboard.cached_history.as_deref(),
                    filter_query,
                    *is_filtering,
                    *mode,
                    spinner_tick,
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
            AppState::LoadingStatus { prev_state, .. }
            | AppState::LoadingHistory { prev_state, .. }
            | AppState::LoadingBranches { prev_state, .. }
            | AppState::Cleaning { prev_state, .. }
            | AppState::Staging { prev_state, .. }
            | AppState::Unstaging { prev_state, .. }
            | AppState::SwitchingBranchTask { prev_state, .. }
            | AppState::GeneratingCommitMessage { prev_state, .. }
            | AppState::LoadingDiff { prev_state, .. } => {
                Self::render_background(f, prev_state, context, area, spinner_tick);
            }
            _ => {}
        }
    }

    pub fn render_tui<R: ProjectRepository + Clone + Send + Sync + 'static>(
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
        let (async_tx, async_rx) = unbounded_channel();

        let res = Self::run_loop(
            &mut terminal,
            repo,
            &mut state,
            &mut spinner_tick,
            rx,
            async_tx,
            async_rx,
        );

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::collapsible_if)]
    fn run_loop<R: ProjectRepository + Clone + Send + Sync + 'static>(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        repo: &R,
        state: &mut AppState,
        spinner_tick: &mut usize,
        rx: Option<Receiver<RepositoryEvent>>,
        async_tx: UnboundedSender<AsyncResult>,
        mut async_rx: UnboundedReceiver<AsyncResult>,
    ) -> Result<Option<String>> {
        let current_dir = std::env::current_dir().map_err(|e| anyhow::anyhow!(e))?;
        let project_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        let context = repo.detect_context(std::path::Path::new("."));
        let render_context = RenderContext {
            project_name,
            context,
        };

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

            // Handle async results
            while let Ok(result) = async_rx.try_recv() {
                match result {
                    AsyncResult::StatusFetched { path, result } => {
                        if let AppState::ListingWorktrees {
                            dashboard,
                            table_state,
                            worktrees,
                            ..
                        } = state
                            && let Some(selected_idx) = table_state.selected()
                            && let Some(wt) = worktrees.get(selected_idx)
                            && wt.path == path
                        {
                            dashboard.loading = false;
                            dashboard.cached_status = result.ok();
                        } else if let AppState::LoadingStatus {
                            path: load_path,
                            branch,
                            prev_state,
                            ..
                        } = state
                            && *load_path == path
                        {
                            match result {
                                Ok(status) => {
                                    *state = AppState::ViewingStatus {
                                        path,
                                        branch: branch.clone(),
                                        status: crate::app::model::StatusViewState {
                                            staged: status.staged,
                                            unstaged: status.unstaged,
                                            untracked: status.untracked,
                                            selected_index: 0,
                                            diff_preview: None,
                                            show_diff: false,
                                        },
                                        prev_state: prev_state.clone(),
                                    };
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("Failed to fetch status: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                        }
                    }
                    AsyncResult::HistoryFetched { path, result } => {
                        if let AppState::ListingWorktrees {
                            dashboard,
                            table_state,
                            worktrees,
                            ..
                        } = state
                            && let Some(selected_idx) = table_state.selected()
                            && let Some(wt) = worktrees.get(selected_idx)
                            && wt.path == path
                        {
                            dashboard.loading = false;
                            dashboard.cached_history = result.ok();
                        } else if let AppState::LoadingHistory {
                            branch, prev_state, ..
                        } = state
                        {
                            match result {
                                Ok(commits) => {
                                    *state = AppState::ViewingHistory {
                                        branch: branch.clone(),
                                        commits,
                                        selected_index: 0,
                                        prev_state: prev_state.clone(),
                                    };
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("Failed to fetch history: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                        }
                    }
                    AsyncResult::BranchesFetched { result } => {
                        if let AppState::LoadingBranches { prev_state, .. } = state {
                            match result {
                                Ok(branches) => {
                                    *state = AppState::PickingBaseRef {
                                        branches,
                                        selected_index: 0,
                                        prev_state: prev_state.clone(),
                                    };
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("Failed to list branches: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                        }
                    }
                    AsyncResult::CleanCompleted { result } => {
                        if let AppState::Cleaning { prev_state, .. } = state {
                            match result {
                                Ok(_) => {
                                    *state = create_timed_state(
                                        AppState::WorktreeRemoved,
                                        *prev_state.clone(),
                                        1200,
                                    );
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("Clean failed: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                            state.request_refresh();
                        }
                    }
                    AsyncResult::FetchCompleted {
                        branch: _branch,
                        result,
                    } => {
                        if let AppState::Fetching { prev_state, .. } = state {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Fetch failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = *prev_state.clone();
                            }
                            state.request_refresh();
                        }
                    }
                    AsyncResult::PullCompleted { branch, result } => {
                        if let AppState::Pulling { prev_state, .. } = state {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Pull failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = create_timed_state(
                                    AppState::PullComplete {
                                        branch: branch.clone(),
                                        prev_state: prev_state.clone(),
                                    },
                                    *prev_state.clone(),
                                    800,
                                );
                            }
                            state.request_refresh();
                        }
                    }
                    AsyncResult::PushCompleted { branch, result } => {
                        if let AppState::Pushing { prev_state, .. } = state {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Push failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = create_timed_state(
                                    AppState::PushComplete {
                                        branch: branch.clone(),
                                        prev_state: prev_state.clone(),
                                    },
                                    *prev_state.clone(),
                                    800,
                                );
                            }
                            state.request_refresh();
                        }
                    }
                    AsyncResult::SyncCompleted { branch, result } => {
                        if let AppState::Syncing { prev_state, .. } = state {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Sync failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = create_timed_state(
                                    AppState::SyncComplete {
                                        branch: branch.clone(),
                                        prev_state: prev_state.clone(),
                                    },
                                    *prev_state.clone(),
                                    800,
                                );
                            }
                            state.request_refresh();
                        }
                    }
                    AsyncResult::StagedFile { path: _, result }
                    | AsyncResult::UnstagedFile { path: _, result }
                    | AsyncResult::StagedAll { path: _, result }
                    | AsyncResult::UnstagedAll { path: _, result } => {
                        if let AppState::Staging { prev_state, .. }
                        | AppState::Unstaging { prev_state, .. } = state
                        {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Operation failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = *prev_state.clone();
                                // We need to refresh the status in the view
                                if let AppState::ViewingStatus {
                                    path,
                                    branch,
                                    prev_state: inner_prev,
                                    ..
                                } = state
                                {
                                    let path_clone = path.clone();
                                    let branch_clone = branch.clone();
                                    let tx = async_tx.clone();
                                    let repo_clone = repo.clone();
                                    tokio::task::spawn_blocking(move || {
                                        let res = repo_clone.get_status(&path_clone);
                                        let _ = tx.send(AsyncResult::StatusFetched {
                                            path: path_clone,
                                            result: res,
                                        });
                                    });
                                    *state = AppState::LoadingStatus {
                                        path: path.clone(),
                                        branch: branch_clone,
                                        prev_state: inner_prev.clone(),
                                    };
                                }
                            }
                        }
                    }
                    AsyncResult::DiffFetched { path: _, result } => {
                        if let AppState::LoadingDiff { prev_state, .. } = state {
                            match result {
                                Ok(diff) => {
                                    *state = *prev_state.clone();
                                    if let AppState::ViewingStatus { status, .. } = state {
                                        status.diff_preview = Some(diff);
                                    }
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("Failed to fetch diff: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                        }
                    }
                    AsyncResult::BranchSwitched { path: _, result } => {
                        if let AppState::SwitchingBranchTask { prev_state, .. } = state {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Branch switch failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = *prev_state.clone();
                                state.request_refresh();
                            }
                        }
                    }
                    AsyncResult::CommitMessageGenerated { result } => {
                        if let AppState::GeneratingCommitMessage { prev_state, .. } = state {
                            match result {
                                Ok(msg) => {
                                    *state = AppState::Prompting {
                                        prompt_type: crate::app::model::PromptType::CommitMessage,
                                        input: msg,
                                        prev_state: prev_state.clone(),
                                    };
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("AI generation failed: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                        }
                    }
                    AsyncResult::StashApplied { result }
                    | AsyncResult::StashPopped { result }
                    | AsyncResult::StashDropped { result }
                    | AsyncResult::StashSaved { result } => {
                        if let AppState::StashAction { prev_state, .. } = state {
                            if let Err(e) = result {
                                *state = AppState::Error(
                                    format!("Stash operation failed: {e}"),
                                    prev_state.clone(),
                                );
                            } else {
                                *state = *prev_state.clone();
                                // If we were in ViewingStashes, we need to refresh the stash list
                                if let AppState::ViewingStashes {
                                    path,
                                    branch,
                                    selected_index,
                                    prev_state: inner_prev,
                                    ..
                                } = state
                                {
                                    let path_clone = path.clone();
                                    let branch_clone = branch.clone();
                                    let tx = async_tx.clone();
                                    let repo_clone = repo.clone();
                                    let path_clone_for_task = path_clone.clone();
                                    let current_index = *selected_index;
                                    tokio::task::spawn_blocking(move || {
                                        let res = repo_clone.list_stashes(&path_clone_for_task);
                                        let _ = tx.send(AsyncResult::StashesFetched {
                                            path: path_clone_for_task,
                                            result: res,
                                        });
                                    });
                                    *state = AppState::LoadingStashes {
                                        path: path_clone.clone(),
                                        branch: branch_clone,
                                        selected_index: current_index,
                                        prev_state: inner_prev.clone(),
                                    };
                                }
                            }
                        }
                    }
                    AsyncResult::StashesFetched { path: _, result } => {
                        if let AppState::LoadingStashes {
                            path,
                            branch,
                            selected_index,
                            prev_state,
                        } = state
                        {
                            match result {
                                Ok(stashes) => {
                                    let new_selected_index = if stashes.is_empty() {
                                        0
                                    } else {
                                        (*selected_index).min(stashes.len() - 1)
                                    };
                                    *state = AppState::ViewingStashes {
                                        path: path.clone(),
                                        branch: branch.clone(),
                                        stashes,
                                        selected_index: new_selected_index,
                                        prev_state: prev_state.clone(),
                                    };
                                }
                                Err(e) => {
                                    *state = AppState::Error(
                                        format!("Failed to list stashes: {e}"),
                                        prev_state.clone(),
                                    );
                                }
                            }
                        }
                    }
                    AsyncResult::WorktreesListed { result } => {
                        if let AppState::ListingWorktrees {
                            worktrees,
                            filtered_worktrees,
                            table_state,
                            dashboard,
                            filter_query,
                            ..
                        } = state
                        {
                            if let Ok(new_worktrees) = result {
                                *worktrees = new_worktrees;
                                *filtered_worktrees =
                                    crate::app::model::filter_worktrees(worktrees, filter_query);
                                if table_state.selected().is_none() && !worktrees.is_empty() {
                                    table_state.select(Some(0));
                                }
                                // Trigger dashboard refresh for new selection
                                dashboard.cached_status = None;
                                dashboard.cached_history = None;
                            }
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
                selection_mode: _,
                dashboard: _,
                table_state: _,
                worktrees: _,
                filtered_worktrees: _,
                filter_query: _,
                is_filtering: _,
                mode: _,
                last_selection_change: _,
                ..
            } = state
            {
                if *refresh_needed == RefreshType::Full {
                    let repo_clone = repo.clone();
                    let tx = async_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let res = repo_clone.list_worktrees();
                        let _ = tx.send(AsyncResult::WorktreesListed { result: res });
                    });
                    *refresh_needed = RefreshType::None;
                } else if *refresh_needed == RefreshType::Dashboard {
                    if let AppState::ListingWorktrees {
                        refresh_needed,
                        dashboard,
                        ..
                    } = state
                    {
                        dashboard.cached_status = None;
                        dashboard.cached_history = None;
                        dashboard.loading = false;
                        *refresh_needed = RefreshType::None;
                    }
                }
            }

            // Core debouncing logic - needs careful borrowing
            if let AppState::ListingWorktrees {
                dashboard,
                filtered_worktrees,
                table_state,
                last_selection_change,
                ..
            } = state
            {
                let debounce_duration = Duration::from_millis(200);
                if last_selection_change.elapsed() >= debounce_duration
                    && !dashboard.loading
                    && (dashboard.cached_status.is_none() || dashboard.cached_history.is_none())
                {
                    Self::fetch_dashboard_data(
                        repo,
                        filtered_worktrees,
                        table_state.selected(),
                        dashboard,
                        &async_tx,
                    );
                }
            }

            terminal.draw(|f| Self::draw(f, repo, state, *spinner_tick, &render_context))?;
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
                            &async_tx,
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
                            &async_tx,
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
                            &async_tx,
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
                            &async_tx,
                        );
                    }
                    AppState::ViewingStashes {
                        path,
                        branch,
                        stashes,
                        selected_index,
                        prev_state,
                    } => {
                        new_state = handle_stash_events(
                            &event,
                            repo,
                            path,
                            branch,
                            stashes,
                            selected_index,
                            prev_state,
                            &current_state_clone,
                            &async_tx,
                        );
                    }
                    AppState::LoadingStatus { .. }
                    | AppState::LoadingHistory { .. }
                    | AppState::LoadingBranches { .. }
                    | AppState::Cleaning { .. }
                    | AppState::Staging { .. }
                    | AppState::Unstaging { .. }
                    | AppState::SwitchingBranchTask { .. }
                    | AppState::GeneratingCommitMessage { .. }
                    | AppState::LoadingDiff { .. } => {
                        // Background loading states don't have secondary event handlers
                        // But can still be exited via global q/Esc handled below
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
                }
            }
        }
    }

    fn fetch_dashboard_data<R: ProjectRepository + Clone + Send + Sync + 'static>(
        repo: &R,
        worktrees: &[Worktree],
        selected_index: Option<usize>,
        dashboard: &mut crate::app::model::DashboardState,
        async_tx: &UnboundedSender<AsyncResult>,
    ) {
        if let Some(i) = selected_index
            && let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare)
        {
            // If we already have data or are loading, don't fetch again
            // But if we switched tabs, we might need different data
            match dashboard.active_tab {
                crate::app::model::DashboardTab::Status => {
                    if dashboard.cached_status.is_none() && !dashboard.loading {
                        dashboard.loading = true;
                        let repo_clone = repo.clone();
                        let path = wt.path.clone();
                        let tx = async_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = repo_clone.get_status(&path);
                            let _ = tx.send(AsyncResult::StatusFetched { path, result });
                        });
                    }
                }
                crate::app::model::DashboardTab::Log => {
                    if dashboard.cached_history.is_none() && !dashboard.loading {
                        dashboard.loading = true;
                        let repo_clone = repo.clone();
                        let path = wt.path.clone();
                        let tx = async_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = repo_clone.get_history(&path, 10);
                            let _ = tx.send(AsyncResult::HistoryFetched { path, result });
                        });
                    }
                }
                _ => {}
            }
        }
    }

    pub fn draw<R: ProjectRepository>(
        f: &mut Frame,
        repo: &R,
        state: &mut AppState,
        spinner_tick: usize,
        render_context: &RenderContext,
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

        let context = render_context.context;
        f.render_widget(
            HeaderWidget {
                context,
                project_name: &render_context.project_name,
                state: display_state,
                spinner_tick,
            },
            chunks[0],
        );

        match display_state {
            AppState::ListingWorktrees {
                worktrees,
                filtered_worktrees,
                table_state,
                dashboard,
                filter_query,
                is_filtering,
                mode,
                ..
            } => {
                render_listing(
                    f,
                    worktrees.as_slice(),
                    filtered_worktrees.as_slice(),
                    table_state,
                    context,
                    chunks[1],
                    dashboard.active_tab,
                    dashboard.cached_status.as_ref(),
                    dashboard.cached_history.as_deref(),
                    filter_query,
                    *is_filtering,
                    *mode,
                    spinner_tick,
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
                Self::render_background(f, prev_state, context, chunks[1], spinner_tick);
                render_history(f, branch, commits, *selected_index);
            }
            AppState::SwitchingBranch {
                branches,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1], spinner_tick);
                render_branch_selection(f, branches, *selected_index, None);
            }
            AppState::PickingBaseRef {
                branches,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1], spinner_tick);
                render_branch_selection(f, branches, *selected_index, Some("SELECT BASE BRANCH"));
            }
            AppState::SelectingEditor {
                branch,
                options,
                selected,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1], spinner_tick);
                render_editor_selection(f, branch, options, *selected);
            }
            AppState::Prompting {
                prompt_type,
                input,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1], spinner_tick);
                render_prompt(f, prompt_type, input);
            }
            AppState::Committing {
                branch,
                selected_index,
                prev_state,
                ..
            } => {
                Self::render_background(f, prev_state, context, chunks[1], spinner_tick);
                render_commit_menu(f, branch, *selected_index);
            }
            AppState::ViewingStashes { .. } => {
                StashListWidget::render(f, chunks[1], display_state);
            }
            AppState::LoadingStatus { .. }
            | AppState::LoadingHistory { .. }
            | AppState::LoadingBranches { .. }
            | AppState::Cleaning { .. }
            | AppState::Staging { .. }
            | AppState::Unstaging { .. }
            | AppState::SwitchingBranchTask { .. }
            | AppState::GeneratingCommitMessage { .. }
            | AppState::LoadingDiff { .. }
            | AppState::LoadingStashes { .. }
            | AppState::StashAction { .. } => {
                render_modals(f, repo, display_state, spinner_tick);
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

        fn rebase(&self, _path: &str, _upstream: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn get_conflict_diff(&self, _path: &str) -> anyhow::Result<String> {
            Ok(String::new())
        }

        fn explain_rebase_conflict(&self, _diff: &str) -> anyhow::Result<String> {
            Ok("Mock conflict explanation".to_string())
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

        fn migrate_to_bare(
            &self,
            _force: bool,
            _dry_run: bool,
        ) -> anyhow::Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from("/mock/migrated_hub"))
        }

        fn check_status(&self, _path: &std::path::Path) -> crate::domain::repository::RepoStatus {
            crate::domain::repository::RepoStatus::BareHub
        }

        fn watch(&self) -> anyhow::Result<Receiver<RepositoryEvent>> {
            let (_, rx) = crossbeam_channel::unbounded();
            Ok(rx)
        }

        fn list_stashes(
            &self,
            _path: &str,
        ) -> anyhow::Result<Vec<crate::domain::repository::StashEntry>> {
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

    #[test]
    fn test_draw_welcome_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let repo = MockRepository;
        let mut state = AppState::Welcome;
        let render_context = RenderContext {
            project_name: "test-project".to_string(),
            context: ProjectContext::Standard,
        };

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0, &render_context);
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
            filtered_worktrees: worktrees.clone(),
            worktrees,
            table_state,
            refresh_needed: RefreshType::None,
            selection_mode: false,
            dashboard: crate::app::model::DashboardState {
                active_tab: crate::app::model::DashboardTab::Info,
                cached_status: None,
                cached_history: None,
                loading: false,
            },
            filter_query: String::new(),
            is_filtering: false,
            mode: crate::app::model::AppMode::Normal,
            last_selection_change: std::time::Instant::now(),
        };
        let render_context = RenderContext {
            project_name: "test-project".to_string(),
            context: ProjectContext::Standard,
        };

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0, &render_context);
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
        let render_context = RenderContext {
            project_name: "test-project".to_string(),
            context: ProjectContext::Standard,
        };

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0, &render_context);
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
        let render_context = RenderContext {
            project_name: "test-project".to_string(),
            context: ProjectContext::Standard,
        };

        terminal
            .draw(|f| {
                View::draw(f, &repo, &mut state, 0, &render_context);
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
