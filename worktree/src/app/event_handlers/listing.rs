use crate::app::intent::Intent;
use crate::app::model::{AppState, EditorConfig, RefreshType, filter_worktrees, AppMode, DashboardTab, DashboardState};
use crate::domain::repository::{ProjectRepository, Worktree};
use anyhow::Result;
use ratatui::{Terminal, backend::CrosstermBackend, widgets::TableState};
use std::io;
use std::process::Command;

use super::helpers::{create_timed_state, move_selection};

#[allow(clippy::too_many_arguments)]
pub fn handle_listing_events<R: ProjectRepository>(
    event: &crossterm::event::Event,
    repo: &R,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    worktrees: &[Worktree],
    table_state: &mut TableState,
    current_state: &AppState,
    spinner_tick: &usize,
) -> Result<Option<AppState>> {
    use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};

    let (filter_query, is_filtering, mode) = if let AppState::ListingWorktrees {
        filter_query,
        is_filtering,
        mode,
        ..
    } = current_state
    {
        (filter_query.clone(), *is_filtering, *mode)
    } else {
        (String::new(), false, AppMode::Normal)
    };

    let filtered_worktrees = filter_worktrees(worktrees, &filter_query);

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
                        move_selection(table_state, filtered_worktrees.len(), 1);
                        selection_changed = true;
                    }
                    KeyCode::Up => {
                        move_selection(table_state, filtered_worktrees.len(), -1);
                        selection_changed = true;
                    }
                    _ => {}
                }

                if changed || selection_changed {
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        filter_query,
                        is_filtering,
                        refresh_needed,
                        mode: m,
                        table_state: ts,
                        ..
                    } = &mut new_state
                    {
                        if changed {
                            *filter_query = new_query;
                            *is_filtering = !stop_filtering;
                            if stop_filtering {
                                *m = AppMode::Normal;
                            }
                        }
                        if changed || selection_changed {
                            *refresh_needed = RefreshType::Dashboard;
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
                    KeyCode::Char('/') => {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { is_filtering, mode: m, .. } = &mut new_state {
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
                        move_selection(table_state, filtered_worktrees.len(), 1);
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { table_state: ts, refresh_needed, .. } = &mut new_state {
                            *ts = table_state.clone();
                            *refresh_needed = RefreshType::Dashboard;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        move_selection(table_state, filtered_worktrees.len(), -1);
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees { table_state: ts, refresh_needed, .. } = &mut new_state {
                            *ts = table_state.clone();
                            *refresh_needed = RefreshType::Dashboard;
                        }
                        return Ok(Some(new_state));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i) {
                            let branch = if wt.is_bare { "HUB".to_string() } else { wt.branch.clone() };
                            let path = if wt.is_bare { repo.get_project_root()?.to_string_lossy().to_string() } else { wt.path.clone() };
                            let prev = Box::new(current_state.clone());
                            if let Ok(Some(editor)) = repo.get_preferred_editor() {
                                let _ = Command::new(&editor).arg(&path).spawn();
                                return Ok(Some(create_timed_state(AppState::OpeningEditor { branch, editor, prev_state: prev.clone() }, *prev, 800)));
                            }
                            return Ok(Some(AppState::SelectingEditor { branch, options: EditorConfig::defaults(), selected: 0, prev_state: prev }));
                        }
                    }
                    KeyCode::Char('v') => {
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare) {
                            if let Ok(status) = repo.get_status(&wt.path) {
                                return Ok(Some(AppState::ViewingStatus {
                                    path: wt.path.clone(),
                                    branch: wt.branch.clone(),
                                    status: crate::app::model::StatusViewState {
                                        staged: status.staged,
                                        unstaged: status.unstaged,
                                        untracked: status.untracked,
                                        selected_index: 0,
                                        diff_preview: None,
                                        show_diff: false,
                                    },
                                    prev_state: Box::new(current_state.clone()),
                                }));
                            }
                        }
                    }
                    KeyCode::Char('l') => {
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare) {
                            if let Ok(commits) = repo.get_history(&wt.path, 50) {
                                return Ok(Some(AppState::ViewingHistory { branch: wt.branch.clone(), commits, selected_index: 0, prev_state: Box::new(current_state.clone()) }));
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Some(AppState::Exiting(None))),
                    KeyCode::Char('1' | '2' | '3') => {
                         if let AppState::ListingWorktrees { worktrees, table_state, selection_mode, dashboard, filter_query, is_filtering, mode: m, .. } = current_state {
                            let active_tab = match key_code {
                                KeyCode::Char('1') => DashboardTab::Info,
                                KeyCode::Char('2') => DashboardTab::Status,
                                KeyCode::Char('3') => DashboardTab::Log,
                                _ => dashboard.active_tab,
                            };
                            return Ok(Some(AppState::ListingWorktrees {
                                worktrees: worktrees.to_vec(),
                                table_state: table_state.clone(),
                                refresh_needed: RefreshType::Dashboard,
                                selection_mode: *selection_mode,
                                dashboard: DashboardState {
                                    active_tab,
                                    cached_status: dashboard.cached_status.clone(),
                                    cached_history: dashboard.cached_history.clone(),
                                },
                                filter_query: filter_query.clone(),
                                is_filtering: *is_filtering,
                                mode: *m,
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
                        let branches = repo.list_branches().map_err(|e| anyhow::anyhow!("Failed to list branches: {e}"))?;
                        return Ok(Some(AppState::PickingBaseRef { branches, selected_index: 0, prev_state: Box::new(current_state.clone()) }));
                    }
                    KeyCode::Char('d' | 'x' | 'D') => {
                        let is_force = matches!(key_code, KeyCode::Char('D'));
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare) {
                            return Ok(Some(AppState::Confirming {
                                title: if is_force { " FORCE REMOVE ".into() } else { " REMOVE ".into() },
                                message: format!("Are you sure you want to {}remove worktree '{}'?", if is_force { "FORCE " } else { "" }, wt.branch),
                                action: Box::new(Intent::RemoveWorktree { intent: wt.path.clone(), force: is_force }),
                                prev_state: Box::new(current_state.clone()),
                            }));
                        }
                    }
                    KeyCode::Char('c') => {
                        let prev = Box::new(current_state.clone());
                        match repo.clean_worktrees(false, false) {
                            Ok(_) => return Ok(Some(create_timed_state(AppState::WorktreeRemoved, *prev, 1200))),
                            Err(e) => return Ok(Some(AppState::Error(format!("Failed to clean: {e}"), prev))),
                        }
                    }
                    KeyCode::Char('C') => {
                        return Ok(Some(AppState::Confirming {
                            title: " CLEAN ARTIFACTS ".into(),
                            message: "Remove all build artifacts from INACTIVE worktrees?".into(),
                            action: Box::new(Intent::CleanWorktrees { dry_run: false, artifacts: true }),
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
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare) {
                            let branch = wt.branch.clone();
                            let path = wt.path.clone();
                            let prev = Box::new(current_state.clone());
                            let _ = repo.sync_configs(&path);
                            return Ok(Some(create_timed_state(AppState::SyncComplete { branch, prev_state: prev.clone() }, *prev, 800)));
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare) {
                            let branch = wt.branch.clone();
                            let path = wt.path.clone();
                            let prev = Box::new(current_state.clone());
                            if let Err(e) = repo.push(&path) {
                                return Ok(Some(AppState::Error(format!("Push failed: {e}"), prev)));
                            }
                            return Ok(Some(create_timed_state(AppState::PushComplete { branch, prev_state: prev.clone() }, *prev, 800)));
                        }
                    }
                    KeyCode::Char('P') => {
                         if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare) {
                            let branch = wt.branch.clone();
                            let path = wt.path.clone();
                            let prev = Box::new(current_state.clone());
                            if let Err(e) = repo.pull(&path) {
                                return Ok(Some(AppState::Error(format!("Pull failed: {e}"), prev)));
                            }
                            return Ok(Some(create_timed_state(AppState::PullComplete { branch, prev_state: prev.clone() }, *prev, 800)));
                        }
                    }
                    KeyCode::Char('f') => {
                        if let Some(i) = table_state.selected() && let Some(wt) = filtered_worktrees.get(i) {
                            let _ = repo.fetch(&wt.path);
                            return Ok(Some(current_state.clone()));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Ok(size) = terminal.size() {
                        let term_width = size.width;
                        let list_width = ((f32::from(term_width) - 2.0) * 0.4) as u16;
                        let x = mouse.column;
                        let y = mouse.row;

                        if x >= 1 && x <= 1 + list_width && y >= 5 {
                            let row_index = (y as i16 - 5) as usize; 
                            if row_index < filtered_worktrees.len() {
                                table_state.select(Some(row_index));
                                if let AppState::ListingWorktrees { selection_mode, dashboard, filter_query, is_filtering, mode: m, .. } = current_state {
                                    return Ok(Some(AppState::ListingWorktrees {
                                        worktrees: worktrees.to_vec(),
                                        table_state: table_state.clone(),
                                        refresh_needed: RefreshType::Dashboard,
                                        selection_mode: *selection_mode,
                                        dashboard: dashboard.clone(),
                                        filter_query: filter_query.clone(),
                                        is_filtering: *is_filtering,
                                        mode: *m,
                                    }));
                                }
                            }
                        } else if x > 1 + list_width && y >= 4 {
                            if let AppState::ListingWorktrees { selection_mode, dashboard, filter_query, is_filtering, mode: m, .. } = current_state {
                                let dash_x = x - (1 + list_width);
                                let active_tab = if dash_x < 12 { Some(DashboardTab::Info) } else if dash_x < 24 { Some(DashboardTab::Status) } else if dash_x < 36 { Some(DashboardTab::Log) } else { None };
                                if let Some(tab) = active_tab {
                                    return Ok(Some(AppState::ListingWorktrees {
                                        worktrees: worktrees.to_vec(),
                                        table_state: table_state.clone(),
                                        refresh_needed: RefreshType::Dashboard,
                                        selection_mode: *selection_mode,
                                        dashboard: DashboardState {
                                            active_tab: tab,
                                            cached_status: dashboard.cached_status.clone(),
                                            cached_history: dashboard.cached_history.clone(),
                                        },
                                        filter_query: filter_query.clone(),
                                        is_filtering: *is_filtering,
                                        mode: *m,
                                    }));
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => {
                    move_selection(table_state, filtered_worktrees.len(), 1);
                    if let AppState::ListingWorktrees { worktrees, selection_mode, dashboard, filter_query, is_filtering, mode: m, .. } = current_state {
                        return Ok(Some(AppState::ListingWorktrees {
                            worktrees: worktrees.to_vec(),
                            table_state: table_state.clone(),
                            refresh_needed: RefreshType::Dashboard,
                            selection_mode: *selection_mode,
                            dashboard: dashboard.clone(),
                            filter_query: filter_query.clone(),
                            is_filtering: *is_filtering,
                            mode: *m,
                        }));
                    }
                }
                MouseEventKind::ScrollUp => {
                    move_selection(table_state, filtered_worktrees.len(), -1);
                    if let AppState::ListingWorktrees { worktrees, selection_mode, dashboard, filter_query, is_filtering, mode: m, .. } = current_state {
                        return Ok(Some(AppState::ListingWorktrees {
                            worktrees: worktrees.to_vec(),
                            table_state: table_state.clone(),
                            refresh_needed: RefreshType::Dashboard,
                            selection_mode: *selection_mode,
                            dashboard: dashboard.clone(),
                            filter_query: filter_query.clone(),
                            is_filtering: *is_filtering,
                            mode: *m,
                        }));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(None)
}
