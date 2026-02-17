use crate::app::intent::Intent;
use crate::app::model::{AppState, EditorConfig, RefreshType, filter_worktrees};
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

    let (filter_query, is_filtering) = if let AppState::ListingWorktrees {
        filter_query,
        is_filtering,
        ..
    } = current_state
    {
        (filter_query.clone(), *is_filtering)
    } else {
        (String::new(), false)
    };

    let filtered_worktrees = filter_worktrees(worktrees, &filter_query);

    match event {
        Event::Key(key) => {
            let key_code = key.code;

            if is_filtering {
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
                        table_state: ts,
                        ..
                    } = &mut new_state
                    {
                        if changed {
                            *filter_query = new_query;
                            *is_filtering = !stop_filtering;
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

            if key_code == KeyCode::Char('/') {
                let mut new_state = current_state.clone();
                if let AppState::ListingWorktrees { is_filtering, .. } = &mut new_state {
                    *is_filtering = true;
                }
                return Ok(Some(new_state));
            }

            // Handle Shift+P for Pull before normalization (since p is Push)
            if key_code == KeyCode::Char('P')
                && let Some(i) = table_state.selected()
                && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
            {
                let branch = wt.branch.clone();
                let path = wt.path.clone();
                let prev = Box::new(current_state.clone());
                let mut pulling_state = AppState::Pulling {
                    branch: branch.clone(),
                    prev_state: prev.clone(),
                };
                terminal.draw(|f| {
                    super::super::view::View::draw(f, repo, &mut pulling_state, *spinner_tick);
                })?;
                if let Err(e) = repo.pull(&path) {
                    return Ok(Some(AppState::Error(format!("Failed to pull: {e}"), prev)));
                }
                let prev_clone = prev.clone();
                let complete_state = AppState::PullComplete {
                    branch,
                    prev_state: prev,
                };
                return Ok(Some(create_timed_state(complete_state, *prev_clone, 800)));
            }

            // Handle navigation keys that are case-sensitive before normalization
            match key_code {
                KeyCode::Char('g') => {
                    table_state.select(Some(0));
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                    }
                    return Ok(Some(new_state));
                }
                KeyCode::Char('G') => {
                    if !filtered_worktrees.is_empty() {
                        table_state.select(Some(filtered_worktrees.len() - 1));
                    }
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                    }
                    return Ok(Some(new_state));
                }
                KeyCode::Char('d' | 'x' | 'D') => {
                    let is_force = matches!(key_code, KeyCode::Char('D'));
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
                    {
                        return Ok(Some(AppState::Confirming {
                            title: if is_force {
                                " FORCE REMOVE WORKTREE ".into()
                            } else {
                                " REMOVE WORKTREE ".into()
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
                KeyCode::Char('C') => {
                    return Ok(Some(AppState::Confirming {
                        title: " CLEAN BUILD ARTIFACTS ".into(),
                        message: "Are you sure you want to remove all build artifacts (node_modules, target, build, etc.) from INACTIVE worktrees?".into(),
                        action: Box::new(Intent::CleanWorktrees {
                            dry_run: false,
                            artifacts: true,
                        }),
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
                _ => {}
            }

            let normalized_code = match key_code {
                KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                _ => key_code,
            };

            match normalized_code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if !filter_query.is_empty() {
                        let mut new_state = current_state.clone();
                        if let AppState::ListingWorktrees {
                            filter_query,
                            refresh_needed,
                            ..
                        } = &mut new_state
                        {
                            filter_query.clear();
                            *refresh_needed = RefreshType::Dashboard;
                        }
                        return Ok(Some(new_state));
                    }
                    return Ok(Some(AppState::Exiting(None)));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    move_selection(table_state, filtered_worktrees.len(), 1);
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                    }
                    return Ok(Some(new_state));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    move_selection(table_state, filtered_worktrees.len(), -1);
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                    }
                    return Ok(Some(new_state));
                }
                KeyCode::Char('1' | '2' | '3') => {
                    if let AppState::ListingWorktrees {
                        worktrees,
                        table_state,
                        selection_mode,
                        dashboard,
                        filter_query,
                        is_filtering,
                        ..
                    } = current_state
                    {
                        let active_tab = match normalized_code {
                            KeyCode::Char('1') => crate::app::model::DashboardTab::Info,
                            KeyCode::Char('2') => crate::app::model::DashboardTab::Status,
                            KeyCode::Char('3') => crate::app::model::DashboardTab::Log,
                            _ => dashboard.active_tab,
                        };
                        return Ok(Some(AppState::ListingWorktrees {
                            worktrees: worktrees.clone(),
                            table_state: table_state.clone(),
                            refresh_needed: RefreshType::Dashboard,
                            selection_mode: *selection_mode,
                            dashboard: crate::app::model::DashboardState {
                                active_tab,
                                cached_status: dashboard.cached_status.clone(),
                                cached_history: dashboard.cached_history.clone(),
                            },
                            filter_query: filter_query.clone(),
                            is_filtering: *is_filtering,
                        }));
                    }
                }
                KeyCode::Char('a') => {
                    let branches = repo
                        .list_branches()
                        .map_err(|e| anyhow::anyhow!("Failed to list branches: {e}"))?;
                    return Ok(Some(AppState::PickingBaseRef {
                        branches,
                        selected_index: 0,
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
                KeyCode::Char('s') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
                    {
                        let branch = wt.branch.clone();
                        let path = wt.path.clone();
                        let prev = Box::new(current_state.clone());
                        let mut syncing_state = AppState::Syncing {
                            branch: branch.clone(),
                            prev_state: prev.clone(),
                        };
                        terminal.draw(|f| {
                            super::super::view::View::draw(
                                f,
                                repo,
                                &mut syncing_state,
                                *spinner_tick,
                            );
                        })?;
                        let _ = repo.sync_configs(&path);
                        let prev_clone = prev.clone();
                        let complete_state = AppState::SyncComplete {
                            branch,
                            prev_state: prev,
                        };
                        return Ok(Some(create_timed_state(complete_state, *prev_clone, 800)));
                    }
                }
                KeyCode::Char('p') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
                    {
                        let branch = wt.branch.clone();
                        let path = wt.path.clone();
                        let prev = Box::new(current_state.clone());
                        let mut pushing_state = AppState::Pushing {
                            branch: branch.clone(),
                            prev_state: prev.clone(),
                        };
                        terminal.draw(|f| {
                            super::super::view::View::draw(
                                f,
                                repo,
                                &mut pushing_state,
                                *spinner_tick,
                            );
                        })?;
                        if let Err(e) = repo.push(&path) {
                            return Ok(Some(AppState::Error(format!("Failed to push: {e}"), prev)));
                        }
                        let prev_clone = prev.clone();
                        let complete_state = AppState::PushComplete {
                            branch,
                            prev_state: prev,
                        };
                        return Ok(Some(create_timed_state(complete_state, *prev_clone, 800)));
                    }
                }
                KeyCode::Char('c') => {
                    // C for CLEAN stale metadata
                    let prev = Box::new(current_state.clone());
                    match repo.clean_worktrees(false, false) {
                        Ok(cleaned) => {
                            let _count = cleaned.len();
                            return Ok(Some(create_timed_state(
                                AppState::WorktreeRemoved, // Reuse removed state for success feedback
                                *prev,
                                1200,
                            )));
                        }
                        Err(e) => {
                            return Ok(Some(AppState::Error(
                                format!("Failed to clean worktrees: {e}"),
                                prev,
                            )));
                        }
                    }
                }
                KeyCode::Char('o') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i)
                    {
                        let branch = if wt.is_bare {
                            "HUB".to_string()
                        } else {
                            wt.branch.clone()
                        };
                        let path = if wt.is_bare {
                            repo.get_project_root()?.to_string_lossy().to_string()
                        } else {
                            wt.path.clone()
                        };
                        let prev = Box::new(current_state.clone());

                        if let Ok(Some(editor)) = repo.get_preferred_editor() {
                            let prev_clone = prev.clone();
                            let opening_state = AppState::OpeningEditor {
                                branch,
                                editor: editor.clone(),
                                prev_state: prev,
                            };
                            let _ = Command::new(&editor).arg(&path).spawn();
                            return Ok(Some(create_timed_state(opening_state, *prev_clone, 800)));
                        }
                        let options = EditorConfig::defaults();
                        return Ok(Some(AppState::SelectingEditor {
                            branch,
                            options,
                            selected: 0,
                            prev_state: prev,
                        }));
                    }
                }
                KeyCode::Char('u') => {
                    let mut setup_state = AppState::SettingUpDefaults;
                    terminal.draw(|f| {
                        super::super::view::View::draw(f, repo, &mut setup_state, *spinner_tick);
                    })?;

                    // Silent setup for TUI
                    let _ = repo.add_worktree("main", "main");
                    let _ = repo.add_new_worktree("dev", "dev", "main");

                    return Ok(Some(create_timed_state(
                        AppState::SetupComplete,
                        current_state.clone(),
                        1200,
                    )));
                }
                KeyCode::Char('v') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
                        && let Ok(status) = repo.get_status(&wt.path)
                    {
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
                KeyCode::Char('l') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
                        && let Ok(commits) = repo.get_history(&wt.path, 50)
                    {
                        return Ok(Some(AppState::ViewingHistory {
                            branch: wt.branch.clone(),
                            commits,
                            selected_index: 0,
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                }
                KeyCode::Char('b') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i).filter(|wt| !wt.is_bare)
                        && let Ok(branches) = repo.list_branches()
                    {
                        return Ok(Some(AppState::SwitchingBranch {
                            path: wt.path.clone(),
                            branches,
                            selected_index: 0,
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                }
                KeyCode::Char('f') => {
                    if let Some(i) = table_state.selected()
                        && let Some(wt) = filtered_worktrees.get(i)
                    {
                        let branch = if wt.is_bare {
                            "HUB".to_string()
                        } else {
                            wt.branch.clone()
                        };
                        let path = wt.path.clone();
                        let prev = Box::new(current_state.clone());
                        let mut fetching_state = AppState::Fetching {
                            branch,
                            prev_state: prev.clone(),
                        };
                        terminal.draw(|f| {
                            super::super::view::View::draw(
                                f,
                                repo,
                                &mut fetching_state,
                                *spinner_tick,
                            );
                        })?;
                        let _ = repo.fetch(&path);
                        return Ok(Some(*prev));
                    }
                }
                KeyCode::Enter => {
                    if let AppState::ListingWorktrees { selection_mode, .. } = current_state {
                        if *selection_mode {
                            if let Some(i) = table_state.selected()
                                && let Some(wt) = filtered_worktrees.get(i)
                            {
                                return Ok(Some(AppState::Exiting(Some(wt.path.clone()))));
                            }
                        } else if let Some(i) = table_state.selected()
                            && let Some(wt) = filtered_worktrees.get(i)
                        {
                            // Open in editor behavior (replicate 'o' key logic)
                            let branch = if wt.is_bare {
                                "HUB".to_string()
                            } else {
                                wt.branch.clone()
                            };
                            let path = if wt.is_bare {
                                repo.get_project_root()?.to_string_lossy().to_string()
                            } else {
                                wt.path.clone()
                            };
                            let prev = Box::new(current_state.clone());

                            if let Ok(Some(editor)) = repo.get_preferred_editor() {
                                let prev_clone = prev.clone();
                                let opening_state = AppState::OpeningEditor {
                                    branch,
                                    editor: editor.clone(),
                                    prev_state: prev,
                                };
                                let _ = Command::new(&editor).arg(&path).spawn();
                                return Ok(Some(create_timed_state(
                                    opening_state,
                                    *prev_clone,
                                    800,
                                )));
                            }
                            let options = EditorConfig::defaults();
                            return Ok(Some(AppState::SelectingEditor {
                                branch,
                                options,
                                selected: 0,
                                prev_state: prev,
                            }));
                        }
                    }
                }
                KeyCode::Char('?' | 'h') => {
                    return Ok(Some(AppState::Help {
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
                _ => {}
            }
        }
        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Ok(size) = terminal.size() {
                        let term_width = size.width;
                        let _ = size.height;

                        // Fixed Layout Assumptions:
                        // Margin: 1
                        // Header: 3
                        // Body Y Start: 4 (1 (margin) + 3 (header))
                        // Table Header: 1
                        // Data Y Start: 5

                        // Split Logic: Percentage(40) for Worktree List
                        // Width of list = (term_width - 2) * 0.4
                        let list_width = ((f32::from(term_width) - 2.0) * 0.4) as u16;

                        let x = mouse.column;
                        let y = mouse.row;

                        // Check if click is in List Area
                        if x >= 1 && x <= 1 + list_width && y >= 5 {
                            let row_index = (y as i16 - 5) as usize; // 5 is data_start_y
                            if row_index < filtered_worktrees.len() {
                                table_state.select(Some(row_index));

                                // Return same state to trigger refresh of dashboard
                                if let AppState::ListingWorktrees {
                                    worktrees: _,
                                    table_state: _,
                                    selection_mode,
                                    dashboard,
                                    filter_query,
                                    is_filtering,
                                    ..
                                } = current_state
                                {
                                    return Ok(Some(AppState::ListingWorktrees {
                                        worktrees: worktrees.to_vec(),
                                        table_state: table_state.clone(),
                                        refresh_needed: RefreshType::Dashboard,
                                        selection_mode: *selection_mode,
                                        dashboard: dashboard.clone(),
                                        filter_query: filter_query.clone(),
                                        is_filtering: *is_filtering,
                                    }));
                                }
                            }
                        } else if x > 1 + list_width && y >= 4 {
                            // Click In Dashboard Area
                            // Assume Tabs are at the top of the dashboard area, roughly first line
                            if let AppState::ListingWorktrees {
                                worktrees,
                                table_state,
                                selection_mode,
                                dashboard,
                                filter_query,
                                is_filtering,
                                ..
                            } = current_state
                            {
                                // Simple heuristic for tabs:
                                // "Info": First ~10 chars
                                // "Status": Next ~10 chars
                                // "Log": Next ~10 chars
                                // Relative X in dashboard
                                let dash_x = x - (1 + list_width);
                                let active_tab = if dash_x < 12 {
                                    Some(crate::app::model::DashboardTab::Info)
                                } else if dash_x < 24 {
                                    Some(crate::app::model::DashboardTab::Status)
                                } else if dash_x < 36 {
                                    Some(crate::app::model::DashboardTab::Log)
                                } else {
                                    None
                                };

                                if let Some(tab) = active_tab {
                                    return Ok(Some(AppState::ListingWorktrees {
                                        worktrees: worktrees.clone(),
                                        table_state: table_state.clone(),
                                        refresh_needed: RefreshType::Dashboard,
                                        selection_mode: *selection_mode,
                                        dashboard: crate::app::model::DashboardState {
                                            active_tab: tab,
                                            cached_status: dashboard.cached_status.clone(),
                                            cached_history: dashboard.cached_history.clone(),
                                        },
                                        filter_query: filter_query.clone(),
                                        is_filtering: *is_filtering,
                                    }));
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => {
                    move_selection(table_state, filtered_worktrees.len(), 1);
                    // Update state with new table_state
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                    }
                    return Ok(Some(new_state));
                }
                MouseEventKind::ScrollUp => {
                    move_selection(table_state, filtered_worktrees.len(), -1);
                    let mut new_state = current_state.clone();
                    if let AppState::ListingWorktrees {
                        table_state: ts,
                        refresh_needed,
                        ..
                    } = &mut new_state
                    {
                        *ts = table_state.clone();
                        *refresh_needed = RefreshType::Dashboard;
                    }
                    return Ok(Some(new_state));
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(None)
}
