use crate::app::intent::Intent;
use crate::app::model::{AppState, EditorConfig, PromptType};
use crate::domain::repository::{ProjectRepository, Worktree};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::CrosstermBackend, widgets::TableState};
use std::io;
use std::process::Command;
use std::time::Duration;

use super::helpers::move_selection;

#[allow(clippy::too_many_arguments)]
pub fn handle_listing_events<R: ProjectRepository>(
    key_code: KeyCode,
    repo: &R,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    worktrees: &[Worktree],
    table_state: &mut TableState,
    current_state: &AppState,
    spinner_tick: &mut usize,
) -> Result<Option<AppState>> {
    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(Some(AppState::Exiting(None))),
        KeyCode::Down | KeyCode::Char('j') => {
            move_selection(table_state, worktrees.len(), 1);
            return Ok(Some(current_state.clone()));
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_selection(table_state, worktrees.len(), -1);
            return Ok(Some(current_state.clone()));
        }
        KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') => {
            if let AppState::ListingWorktrees {
                worktrees,
                table_state,
                refresh_needed: _,
                selection_mode,
                dashboard,
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
                    refresh_needed: true,
                    selection_mode: *selection_mode,
                    dashboard: crate::app::model::DashboardState {
                        active_tab,
                        cached_status: dashboard.cached_status.clone(),
                        cached_history: dashboard.cached_history.clone(),
                    },
                }));
            }
        }
        KeyCode::Char('a') => {
            return Ok(Some(AppState::Prompting {
                prompt_type: PromptType::AddIntent,
                input: String::new(),
                prev_state: Box::new(current_state.clone()),
            }));
        }
        KeyCode::Char('d') | KeyCode::Char('x') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    return Ok(Some(AppState::Confirming {
                        title: " REMOVE WORKTREE ".into(),
                        message: format!(
                            "Are you sure you want to remove worktree '{}'?",
                            wt.branch
                        ),
                        action: Box::new(Intent::RemoveWorktree {
                            intent: wt.branch.clone(),
                        }),
                        prev_state: Box::new(current_state.clone()),
                    }));
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    let branch = wt.branch.clone();
                    let path = wt.path.clone();
                    let prev = Box::new(current_state.clone());
                    let mut syncing_state = AppState::Syncing {
                        branch: branch.clone(),
                        prev_state: prev.clone(),
                    };
                    terminal.draw(|f| {
                        super::super::view::View::draw(f, repo, &mut syncing_state, *spinner_tick)
                    })?;
                    let _ = repo.sync_configs(&path);
                    let mut complete_state = AppState::SyncComplete {
                        branch,
                        prev_state: prev,
                    };
                    terminal.draw(|f| {
                        super::super::view::View::draw(f, repo, &mut complete_state, *spinner_tick)
                    })?;
                    std::thread::sleep(Duration::from_millis(800));
                    return Ok(Some(complete_state.prev_state_boxed().clone()));
                }
            }
        }
        KeyCode::Char('p') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    let branch = wt.branch.clone();
                    let path = wt.path.clone();
                    let prev = Box::new(current_state.clone());
                    let mut pushing_state = AppState::Pushing {
                        branch: branch.clone(),
                        prev_state: prev.clone(),
                    };
                    terminal.draw(|f| {
                        super::super::view::View::draw(f, repo, &mut pushing_state, *spinner_tick)
                    })?;
                    if let Err(e) = repo.push(&path) {
                        return Ok(Some(AppState::Error(
                            format!("Failed to push: {}", e),
                            prev,
                        )));
                    }
                    let mut complete_state = AppState::PushComplete {
                        branch,
                        prev_state: prev,
                    };
                    terminal.draw(|f| {
                        super::super::view::View::draw(f, repo, &mut complete_state, *spinner_tick)
                    })?;
                    std::thread::sleep(Duration::from_millis(800));
                    return Ok(Some(complete_state.prev_state_boxed().clone()));
                }
            }
        }
        KeyCode::Char('o') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    let branch = wt.branch.clone();
                    let path = wt.path.clone();
                    let prev = Box::new(current_state.clone());

                    if let Ok(Some(editor)) = repo.get_preferred_editor() {
                        let mut opening_state = AppState::OpeningEditor {
                            branch,
                            editor: editor.clone(),
                            prev_state: prev.clone(),
                        };
                        terminal.draw(|f| {
                            super::super::view::View::draw(
                                f,
                                repo,
                                &mut opening_state,
                                *spinner_tick,
                            )
                        })?;
                        let _ = Command::new(&editor).arg(&path).spawn();
                        std::thread::sleep(Duration::from_millis(800));
                        return Ok(Some(*prev));
                    } else {
                        let options = vec![
                            EditorConfig {
                                name: "VS Code".into(),
                                command: "code".into(),
                            },
                            EditorConfig {
                                name: "Cursor".into(),
                                command: "cursor".into(),
                            },
                            EditorConfig {
                                name: "Zed".into(),
                                command: "zed".into(),
                            },
                            EditorConfig {
                                name: "Android Studio".into(),
                                command: "studio".into(),
                            },
                            EditorConfig {
                                name: "IntelliJ IDEA".into(),
                                command: "idea".into(),
                            },
                            EditorConfig {
                                name: "Vim".into(),
                                command: "vim".into(),
                            },
                            EditorConfig {
                                name: "Neovim".into(),
                                command: "nvim".into(),
                            },
                            EditorConfig {
                                name: "Antigravity".into(),
                                command: "antigravity".into(),
                            },
                        ];
                        return Ok(Some(AppState::SelectingEditor {
                            branch,
                            options,
                            selected: 0,
                            prev_state: prev,
                        }));
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            let mut setup_state = AppState::SettingUpDefaults;
            terminal.draw(|f| {
                super::super::view::View::draw(f, repo, &mut setup_state, *spinner_tick)
            })?;

            // Silent setup for TUI
            let _ = repo.add_worktree("main", "main");
            let _ = repo.add_new_worktree("dev", "dev", "main");

            let mut complete_state = AppState::SetupComplete;
            terminal.draw(|f| {
                super::super::view::View::draw(f, repo, &mut complete_state, *spinner_tick)
            })?;
            std::thread::sleep(Duration::from_millis(1200));
            return Ok(Some(current_state.clone()));
        }
        KeyCode::Char('g') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    if let Ok(status) = repo.get_status(&wt.path) {
                        return Ok(Some(AppState::ViewingStatus {
                            path: wt.path.clone(),
                            branch: wt.branch.clone(),
                            status: crate::app::model::StatusViewState {
                                staged: status.staged,
                                unstaged: status.unstaged,
                                untracked: status.untracked,
                                selected_index: 0,
                            },
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                }
            }
        }
        KeyCode::Char('l') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    if let Ok(commits) = repo.get_history(&wt.path, 50) {
                        return Ok(Some(AppState::ViewingHistory {
                            branch: wt.branch.clone(),
                            commits,
                            selected_index: 0,
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                }
            }
        }
        KeyCode::Char('b') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    if let Ok(branches) = repo.list_branches() {
                        return Ok(Some(AppState::SwitchingBranch {
                            path: wt.path.clone(),
                            branches,
                            selected_index: 0,
                            prev_state: Box::new(current_state.clone()),
                        }));
                    }
                }
            }
        }
        KeyCode::Char('f') => {
            if let Some(i) = table_state.selected() {
                if let Some(wt) = worktrees.get(i).filter(|wt| !wt.is_bare) {
                    let branch = wt.branch.clone();
                    let path = wt.path.clone();
                    let prev = Box::new(current_state.clone());
                    let mut fetching_state = AppState::Fetching {
                        branch,
                        prev_state: prev.clone(),
                    };
                    terminal.draw(|f| {
                        super::super::view::View::draw(f, repo, &mut fetching_state, *spinner_tick)
                    })?;
                    let _ = repo.fetch(&path);
                    return Ok(Some(*prev));
                }
            }
        }
        KeyCode::Enter => {
            if let AppState::ListingWorktrees {
                worktrees,
                table_state,
                selection_mode: true,
                ..
            } = current_state
            {
                if let Some(i) = table_state.selected() {
                    if let Some(wt) = worktrees.get(i) {
                        return Ok(Some(AppState::Exiting(Some(wt.path.clone()))));
                    }
                }
            }
        }
        KeyCode::Char('?') | KeyCode::Char('h') => {
            return Ok(Some(AppState::Help {
                prev_state: Box::new(current_state.clone()),
            }));
        }
        _ => {}
    }
    Ok(None)
}
