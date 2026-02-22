use crate::app::async_tasks::AsyncResult;
use crate::app::intent::Intent;
use crate::app::model::{
    AppMode, AppState, DashboardState, DashboardTab, RefreshType, filter_worktrees,
};
use crate::domain::repository::{ProjectRepository, Worktree};
use anyhow::Result;
use ratatui::{Terminal, backend::Backend, widgets::TableState};
use tokio::sync::mpsc::UnboundedSender;

use super::helpers::{create_timed_state, move_selection};
use std::borrow::Cow;

#[allow(clippy::too_many_arguments)]
#[allow(clippy::collapsible_if)]
pub fn handle_listing_events<R: ProjectRepository + Clone + Send + Sync + 'static, B: Backend>(
    event: &crossterm::event::Event,
    repo: &R,
    terminal: &mut Terminal<B>,
    worktrees: &[Worktree],
    table_state: &mut TableState,
    current_state: &AppState,
    _spinner_tick: &usize,
    async_tx: &UnboundedSender<AsyncResult>,
) -> Result<Option<AppState>> {
    use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};

    // Optimization: Early return for irrelevant mouse events (Moved/Drag) to avoid overhead
    if let Event::Mouse(mouse) = event {
        if matches!(mouse.kind, MouseEventKind::Moved | MouseEventKind::Drag(_)) {
            return Ok(None);
        }
    }

    let (filter_query, mode, filtered_indices) = if let AppState::ListingWorktrees {
        filter_query,
        mode,
        filtered_indices,
        ..
    } = current_state
    {
        (
            filter_query.clone(),
            *mode,
            Cow::Borrowed(filtered_indices.as_slice()),
        )
    } else {
        (
            String::new(),
            AppMode::Normal,
            Cow::Owned((0..worktrees.len()).collect()),
        )
    };

    match event {
        Event::Key(key) => {
            let key_code = key.code;

            // Handle Filter Mode
            if mode == AppMode::Filter {
                let mut new_query = filter_query;
                let mut stop_filtering = false;
                let mut changed = false;
                let mut selection_changed = false;

                match key_code {
                    KeyCode::Esc => {
                        new_query.clear();
                        stop_filtering = true;
                        changed = true;
                    }
                    KeyCode::Enter => {
                        stop_filtering = true;
                        changed = true;
                    }
                    KeyCode::Char('u')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        new_query.clear();
                        changed = true;
                    }
                    KeyCode::Backspace => {
                        if new_query.pop().is_some() {
                            changed = true;
                        }
                    }
                    KeyCode::Char(c) => {
                        new_query.push(c);
                        changed = true;
                    }
                    KeyCode::Down => {
                        move_selection(table_state, filtered_indices.len(), 1);
                        selection_changed = true;
                    }
                    KeyCode::Up => {
                        move_selection(table_state, filtered_indices.len(), -1);
                        selection_changed = true;
                    }
                    _ => {}
                }

                if changed || selection_changed {
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        worktrees: raw_worktrees,
                        filtered_indices: state_filtered,
                        filter_query,
                        is_filtering,
                        refresh_needed,
                        mode: m,
                        table_state: ts,
                        last_selection_change,
                        ..
                    } = &mut new_state
                    {
                        if changed {
                            *filter_query = new_query;
                            *is_filtering = !stop_filtering;
                            if stop_filtering {
                                *m = AppMode::Normal;
                            }
                            // Update the cached filtered list
                            *state_filtered = filter_worktrees(raw_worktrees, filter_query);
                        }
                        if changed || selection_changed {
                            *refresh_needed = RefreshType::Dashboard;
                            *last_selection_change = std::time::Instant::now();
                        }
                        *ts = table_state.clone();
                    }
                    return Ok(Some(new_state));
                }
                return Ok(None);
            }

            // Normal and Specialized Modes
            match mode {
                AppMode::Normal => match key_code {
                    KeyCode::Char('?') => {
                        return Ok(Some(AppState::Help {
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                    KeyCode::Char('/') => {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees {
                            is_filtering,
                            mode: m,
                            ..
                        } = &mut new_state
                        {
                            *is_filtering = true;
                            *m = AppMode::Filter;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('m') => {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { mode: m, .. } = &mut new_state {
                            *m = AppMode::Manage;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('g') => {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { mode: m, .. } = &mut new_state {
                            *m = AppMode::Git;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        move_selection(table_state, filtered_indices.len(), 1);
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees {
                            table_state: ts,
                            refresh_needed,
                            last_selection_change,
                            ..
                        } = &mut new_state
                        {
                            *ts = table_state.clone();
                            *refresh_needed = RefreshType::Dashboard;
                            *last_selection_change = std::time::Instant::now();
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        move_selection(table_state, filtered_indices.len(), -1);
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees {
                            table_state: ts,
                            refresh_needed,
                            last_selection_change,
                            ..
                        } = &mut new_state
                        {
                            *ts = table_state.clone();
                            *refresh_needed = RefreshType::Dashboard;
                            *last_selection_change = std::time::Instant::now();
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx)
                        {
                            let branch = if wt.is_bare {
                                "HUB".to_string()
                            } else {
                                wt.branch.clone()
                            };
                            let path = if wt.is_bare {
                                // Offload project root fetching
                                let repo_clone = repo.clone();
                                tokio::task::spawn_blocking(move || {
                                    let root = repo_clone.get_project_root();
                                    let editor = repo_clone.get_preferred_editor();
                                    // Send a composite result or just reuse existing mechanisms
                                    // For simplicity here, let's just trigger the editor logic
                                    if let Ok(root_path) = root {
                                        let path_str = root_path.to_string_lossy().to_string();
                                        if let Ok(Some(editor_cmd)) = editor {
                                            let _ = std::process::Command::new(&editor_cmd)
                                                .arg(&path_str)
                                                .spawn();
                                        }
                                    }
                                });
                                return Ok(Some(create_timed_state(
                                    AppState::OpeningEditor {
                                        branch,
                                        editor: "Detecting...".into(),
                                        prev_state: Box::new(current_state.clone()),
                                    },
                                    current_state.clone(),
                                    800,
                                )));
                            } else {
                                wt.path.clone()
                            };

                            let prev = Box::new(current_state.clone());
                            // Offload editor config fetch
                            let repo_clone = repo.clone();
                            let path_clone = path.clone();

                            tokio::task::spawn_blocking(move || {
                                if let Ok(Some(editor)) = repo_clone.get_preferred_editor() {
                                    let _ = std::process::Command::new(&editor)
                                        .arg(&path_clone)
                                        .spawn();
                                    // We could send an AsyncResult to confirm opening, but Timed state handles UI feedback
                                }
                            });

                            return Ok(Some(create_timed_state(
                                AppState::OpeningEditor {
                                    branch,
                                    editor: "Opening...".into(),
                                    prev_state: prev.clone(),
                                },
                                *prev,
                                800,
                            )));
                        }
                    }
                    KeyCode::Char('v') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            let repo_clone = repo.clone();
                            let path_clone = wt.path.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.get_status(&path_clone);
                                let _ = tx.send(AsyncResult::StatusFetched {
                                    path: path_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::LoadingStatus {
                                path: wt.path.clone(),
                                branch: wt.branch.clone(),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    KeyCode::Char('l') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            let repo_clone = repo.clone();
                            let path_clone = wt.path.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.get_history(&path_clone, 50);
                                let _ = tx.send(AsyncResult::HistoryFetched {
                                    path: path_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::LoadingHistory {
                                branch: wt.branch.clone(),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Some(AppState::Exiting(None))),
                    KeyCode::Char('1' | '2' | '3') => {
                        if let AppState::ListingWorktrees {
                            worktrees,
                            filtered_indices,
                            table_state,
                            selection_mode,
                            dashboard,
                            filter_query,
                            is_filtering,
                            mode: m,
                            ..
                        } = current_state
                        {
                            let active_tab = match key_code {
                                KeyCode::Char('1') => DashboardTab::Info,
                                KeyCode::Char('2') => DashboardTab::Status,
                                KeyCode::Char('3') => DashboardTab::Log,
                                _ => dashboard.active_tab,
                            };
                            return Ok(Some(AppState::ListingWorktrees {
                                worktrees: worktrees.to_vec(),
                                filtered_indices: filtered_indices.to_vec(),
                                table_state: table_state.clone(),
                                refresh_needed: RefreshType::Dashboard,
                                selection_mode: *selection_mode,
                                dashboard: DashboardState {
                                    active_tab,
                                    cached_status: dashboard.cached_status.clone(),
                                    cached_history: dashboard.cached_history.clone(),
                                    loading: false,
                                },
                                filter_query: filter_query.clone(),
                                is_filtering: *is_filtering,
                                mode: *m,
                                last_selection_change: std::time::Instant::now(),
                            }));
                        }
                    }
                    KeyCode::Char('o') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx)
                            && wt.is_bare
                        {
                            let repo_clone = repo.clone();
                            tokio::task::spawn_blocking(move || {
                                if let Ok(root) = repo_clone.get_project_root() {
                                    let path = root.to_string_lossy().to_string();
                                    #[cfg(target_os = "macos")]
                                    let _ = std::process::Command::new("open").arg(&path).spawn();
                                    #[cfg(target_os = "linux")]
                                    let _ =
                                        std::process::Command::new("xdg-open").arg(&path).spawn();
                                    #[cfg(target_os = "windows")]
                                    let _ =
                                        std::process::Command::new("explorer").arg(&path).spawn();
                                }
                            });
                        }
                    }
                    KeyCode::Char('f') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx)
                            && wt.is_bare
                        {
                            let repo_clone = repo.clone();
                            let path_clone = wt.path.clone();
                            let branch_clone = wt.branch.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.fetch(&path_clone);
                                let _ = tx.send(AsyncResult::FetchCompleted {
                                    branch: branch_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::Fetching {
                                branch: wt.branch.clone(),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    KeyCode::Char('c') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx)
                            && wt.is_bare
                        {
                            return Ok(Some(AppState::Confirming {
                                title: " PRUNE ".into(),
                                message: "Are you sure you want to prune stale worktrees?".into(),
                                action: Box::new(Intent::CleanWorktrees {
                                    dry_run: false,
                                    artifacts: false,
                                }),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    _ => {}
                },
                AppMode::Manage => match key_code {
                    KeyCode::Esc => {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { mode: m, .. } = &mut new_state {
                            *m = AppMode::Normal;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('a') => {
                        let repo_clone = repo.clone();
                        let tx = async_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = repo_clone.list_branches();
                            let _ = tx.send(AsyncResult::BranchesFetched { result });
                        });
                        return Ok(Some(AppState::LoadingBranches {
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                    KeyCode::Char('r' | 'd' | 'x' | 'D') => {
                        let is_force = matches!(key_code, KeyCode::Char('D'));
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            return Ok(Some(AppState::Confirming {
                                title: if is_force {
                                    " FORCE REMOVE ".into()
                                } else {
                                    " REMOVE ".into()
                                },
                                message: format!(
                                    "Are you sure you want to {}remove worktree '{}'?",
                                    if is_force { "FORCE " } else { "" },
                                    wt.branch
                                ),
                                action: Box::new(Intent::RemoveWorktree {
                                    intent: wt.path.clone(),
                                    force: is_force,
                                }),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    KeyCode::Char('c') => {
                        let repo_clone = repo.clone();
                        let tx = async_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = repo_clone.clean_worktrees(false, false);
                            let _ = tx.send(AsyncResult::CleanCompleted { result });
                        });
                        return Ok(Some(AppState::Cleaning {
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                    KeyCode::Char('C') => {
                        return Ok(Some(AppState::Confirming {
                            title: " CLEAN ARTIFACTS ".into(),
                            message: "Remove all build artifacts from INACTIVE worktrees?".into(),
                            action: Box::new(Intent::CleanWorktrees {
                                dry_run: false,
                                artifacts: true,
                            }),
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                    _ => {}
                },
                AppMode::Git => match key_code {
                    KeyCode::Esc => {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { mode: m, .. } = &mut new_state {
                            *m = AppMode::Normal;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('s') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            let branch = wt.branch.clone();
                            let path = wt.path.clone();
                            let prev = Box::new(current_state.clone());
                            let repo_clone = repo.clone();
                            let path_clone = path.clone();
                            let branch_clone = branch.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.sync_configs(&path_clone);
                                let _ = tx.send(AsyncResult::SyncCompleted {
                                    branch: branch_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::Syncing {
                                branch,
                                prev_state: prev,
                            }));
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            let branch = wt.branch.clone();
                            let path = wt.path.clone();
                            let prev = Box::new(current_state.clone());
                            let repo_clone = repo.clone();
                            let path_clone = path.clone();
                            let branch_clone = branch.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.pull(&path_clone);
                                let _ = tx.send(AsyncResult::PullCompleted {
                                    branch: branch_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::Pulling {
                                branch,
                                prev_state: prev,
                            }));
                        }
                    }
                    KeyCode::Char('P') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            let branch = wt.branch.clone();
                            let path = wt.path.clone();
                            let prev = Box::new(current_state.clone());
                            let repo_clone = repo.clone();
                            let path_clone = path.clone();
                            let branch_clone = branch.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.push(&path_clone);
                                let _ = tx.send(AsyncResult::PushCompleted {
                                    branch: branch_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::Pushing {
                                branch,
                                prev_state: prev,
                            }));
                        }
                    }
                    KeyCode::Char('R') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx).filter(|wt| !wt.is_bare)
                        {
                            let prev = Box::new(current_state.clone());
                            return Ok(Some(AppState::Confirming {
                                title: " REBASE ".into(),
                                message: format!("Rebase '{}' onto its upstream?", wt.branch),
                                action: Box::new(Intent::Rebase { upstream: None }),
                                prev_state: prev,
                            }));
                        }
                    }
                    KeyCode::Char('f') => {
                        if let Some(i) = table_state.selected()
                            && let Some(idx) = filtered_indices.get(i)
                            && let Some(wt) = worktrees.get(*idx)
                        {
                            let repo_clone = repo.clone();
                            let path_clone = wt.path.clone();
                            let branch_clone = wt.branch.clone();
                            let tx = async_tx.clone();

                            tokio::task::spawn_blocking(move || {
                                let result = repo_clone.fetch(&path_clone);
                                let _ = tx.send(AsyncResult::FetchCompleted {
                                    branch: branch_clone,
                                    result,
                                });
                            });

                            return Ok(Some(AppState::Fetching {
                                branch: wt.branch.clone(),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Ok(size) = terminal.size() {
                    let term_width = size.width;
                    let list_width = ((f32::from(term_width) - 2.0) * 0.4) as u16;
                    let x = mouse.column;
                    let y = mouse.row;

                    if x >= 1 && x <= 1 + list_width && y >= 5 {
                        let row_index = (y as i16 - 5) as usize;
                        if row_index < filtered_indices.len() {
                            table_state.select(Some(row_index));
                            if let AppState::ListingWorktrees {
                                filtered_indices: state_filtered,
                                selection_mode,
                                dashboard,
                                filter_query,
                                is_filtering,
                                mode: m,
                                ..
                            } = current_state
                            {
                                return Ok(Some(AppState::ListingWorktrees {
                                    worktrees: worktrees.to_vec(),
                                    filtered_indices: state_filtered.to_vec(),
                                    table_state: table_state.clone(),
                                    refresh_needed: RefreshType::Dashboard,
                                    selection_mode: *selection_mode,
                                    dashboard: dashboard.clone(),
                                    filter_query: filter_query.clone(),
                                    is_filtering: *is_filtering,
                                    mode: *m,
                                    last_selection_change: std::time::Instant::now(),
                                }));
                            }
                        }
                    } else if x > 1 + list_width
                        && y >= 4
                        && let AppState::ListingWorktrees {
                            selection_mode,
                            dashboard,
                            filter_query,
                            is_filtering,
                            mode: m,
                            ..
                        } = current_state
                    {
                        let dash_x = x - (1 + list_width);
                        let active_tab = if dash_x < 12 {
                            Some(DashboardTab::Info)
                        } else if dash_x < 24 {
                            Some(DashboardTab::Status)
                        } else if dash_x < 36 {
                            Some(DashboardTab::Log)
                        } else {
                            None
                        };
                        if let Some(tab) = active_tab {
                            return Ok(Some(AppState::ListingWorktrees {
                                worktrees: worktrees.to_vec(),
                                filtered_indices: filtered_indices.to_vec(), // Use the current function's filtered_indices
                                table_state: table_state.clone(),
                                refresh_needed: RefreshType::Dashboard,
                                selection_mode: *selection_mode,
                                dashboard: DashboardState {
                                    active_tab: tab,
                                    cached_status: dashboard.cached_status.clone(),
                                    cached_history: dashboard.cached_history.clone(),
                                    loading: false,
                                },
                                filter_query: filter_query.clone(),
                                is_filtering: *is_filtering,
                                mode: *m,
                                last_selection_change: std::time::Instant::now(),
                            }));
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                move_selection(table_state, filtered_indices.len(), 1);
                if let AppState::ListingWorktrees { .. } = current_state {
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        last_selection_change,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                        *last_selection_change = std::time::Instant::now();
                    }
                    return Ok(Some(new_state));
                }
            }
            MouseEventKind::ScrollUp => {
                move_selection(table_state, filtered_indices.len(), -1);
                if let AppState::ListingWorktrees { .. } = current_state {
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        last_selection_change,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                        *last_selection_change = std::time::Instant::now();
                    }
                    return Ok(Some(new_state));
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::{DashboardState, DashboardTab, RefreshType};
    use crate::app::test_utils::scaffolding::MockRepoBuilder;
    use crate::domain::repository::Worktree;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_handle_listing_events_normal_mode_system_actions() {
        // 1. Setup
        let hub_wt = Worktree {
            path: "/path/to/hub".into(),
            commit: "abcdef1".into(),
            branch: "main".into(),
            is_bare: true,
            is_detached: false,
            status_summary: None,
            size_bytes: 0,
            metadata: None,
        };
        let worktrees = vec![hub_wt.clone()];
        let repo = MockRepoBuilder::default()
            .with_worktrees(worktrees.clone())
            .build();

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let (async_tx, _async_rx) = mpsc::unbounded_channel();

        let current_state = AppState::ListingWorktrees {
            filtered_indices: vec![0],
            worktrees: worktrees.clone(),
            table_state: table_state.clone(),
            refresh_needed: RefreshType::None,
            selection_mode: false,
            dashboard: DashboardState {
                active_tab: DashboardTab::Info,
                cached_status: None,
                cached_history: None,
                loading: false,
            },
            filter_query: String::new(),
            is_filtering: false,
            mode: AppMode::Normal,
            last_selection_change: std::time::Instant::now(),
        };

        // 2. Test Fetch ('f')
        let fetch_event = Event::Key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()));
        let res = handle_listing_events(
            &fetch_event,
            &repo,
            &mut terminal,
            &worktrees,
            &mut table_state,
            &current_state,
            &0,
            &async_tx,
        )
        .unwrap();

        match res {
            Some(AppState::Fetching { branch, .. }) => assert_eq!(branch, "main"),
            _ => panic!("Expected Fetching state, got {:?}", res),
        }

        // 3. Test Prune ('c')
        let prune_event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
        let res = handle_listing_events(
            &prune_event,
            &repo,
            &mut terminal,
            &worktrees,
            &mut table_state,
            &current_state,
            &0,
            &async_tx,
        )
        .unwrap();

        match res {
            Some(AppState::Confirming { title, .. }) => assert_eq!(title, " PRUNE "),
            _ => panic!("Expected Confirming state for prune, got {:?}", res),
        }

        // 4. Test Open ('o') - verify it doesn't return a state (it's a side effect)
        let open_event = Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()));
        let res = handle_listing_events(
            &open_event,
            &repo,
            &mut terminal,
            &worktrees,
            &mut table_state,
            &current_state,
            &0,
            &async_tx,
        )
        .unwrap();

        assert!(res.is_none());
    }

    #[tokio::test]
    async fn test_handle_listing_events_filter_ctrl_u() {
        // 1. Setup
        let hub_wt = Worktree {
            path: "/path/to/hub".into(),
            commit: "abcdef1".into(),
            branch: "main".into(),
            is_bare: true,
            is_detached: false,
            status_summary: None,
            size_bytes: 0,
            metadata: None,
        };
        let worktrees = vec![hub_wt.clone()];
        let repo = MockRepoBuilder::default()
            .with_worktrees(worktrees.clone())
            .build();

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let (async_tx, _async_rx) = mpsc::unbounded_channel();

        let current_state = AppState::ListingWorktrees {
            filtered_indices: vec![0],
            worktrees: worktrees.clone(),
            table_state: table_state.clone(),
            refresh_needed: RefreshType::None,
            selection_mode: false,
            dashboard: DashboardState {
                active_tab: DashboardTab::Info,
                cached_status: None,
                cached_history: None,
                loading: false,
            },
            filter_query: "some filter text".to_string(),
            is_filtering: true,
            mode: AppMode::Filter,
            last_selection_change: std::time::Instant::now(),
        };

        // 2. Test Ctrl+U
        let ctrl_u_event = Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        let res = handle_listing_events(
            &ctrl_u_event,
            &repo,
            &mut terminal,
            &worktrees,
            &mut table_state,
            &current_state,
            &0,
            &async_tx,
        )
        .unwrap();

        match res {
            Some(AppState::ListingWorktrees { filter_query, .. }) => {
                assert_eq!(
                    filter_query, "",
                    "Filter query should be empty after Ctrl+U"
                );
            }
            _ => panic!("Expected ListingWorktrees state, got {:?}", res),
        }
    }
}
