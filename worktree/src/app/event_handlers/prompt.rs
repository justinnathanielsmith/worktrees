use crate::app::model::{AppState, PromptType, RefreshType};
use crate::domain::repository::ProjectRepository;
use ratatui::widgets::TableState;

pub fn handle_prompt_events<R: ProjectRepository>(
    event: &crossterm::event::Event,
    repo: &R,
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    prompt_type: &PromptType,
    input: &mut String,
    prev_state: &AppState,
    spinner_tick: &usize,
) -> anyhow::Result<Option<AppState>> {
    use crossterm::event::{Event, KeyCode};
    let key_code = if let Event::Key(key) = event {
        key.code
    } else {
        return Ok(None);
    };

    match key_code {
        KeyCode::Enter => {
            let val = input.trim().to_string();
            match prompt_type {
                PromptType::NameNewWorktree { base_ref } => {
                    if !val.is_empty() {
                        let mut adding_state = AppState::AddingWorktree {
                            intent: val.clone(),
                            branch: if val == *base_ref {
                                base_ref.clone()
                            } else {
                                val.clone()
                            },
                        };
                        terminal.draw(|f| {
                            super::super::view::View::draw(
                                f,
                                repo,
                                &mut adding_state,
                                *spinner_tick,
                            );
                        })?;

                        let res = if val == *base_ref {
                            repo.add_worktree(&val, base_ref)
                        } else {
                            repo.add_new_worktree(&val, &val, base_ref)
                        };

                        if let Err(e) = res {
                            return Ok(Some(AppState::Error(
                                e.to_string(),
                                Box::new(AppState::ListingWorktrees {
                                    worktrees: Vec::new(),
                                    table_state: TableState::default(),
                                    refresh_needed: RefreshType::Full,
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
                                }),
                            )));
                        }

                        return Ok(Some(AppState::ListingWorktrees {
                            worktrees: Vec::new(),
                            table_state: TableState::default(),
                            refresh_needed: RefreshType::Full,
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
                        }));
                    }
                }
                PromptType::CommitMessage => {
                    if !val.is_empty() {
                        let (path, target_state) =
                            if let AppState::ViewingStatus { path, .. } = prev_state {
                                (Some(path.clone()), prev_state.clone())
                            } else if let AppState::Committing {
                                path, prev_state, ..
                            } = prev_state
                            {
                                (Some(path.clone()), (**prev_state).clone())
                            } else {
                                (None, prev_state.clone())
                            };

                        if let Some(p) = path {
                            let _ = repo.commit(&p, &val);
                            if let Ok(status) = repo.get_status(&p) {
                                let mut new_state = target_state;
                                if let AppState::ViewingStatus { status: s, .. } = &mut new_state {
                                    s.staged = status.staged;
                                    s.unstaged = status.unstaged;
                                    s.untracked = status.untracked;
                                }
                                return Ok(Some(new_state));
                            }
                        }
                    }
                }
                PromptType::ApiKey => {
                    if !val.is_empty() {
                        let _ = repo.set_api_key(&val);
                    }
                }
                PromptType::StashMessage => {
                    let msg = if val.is_empty() { None } else { Some(val) };
                    if let AppState::ViewingStashes { path, .. } = prev_state {
                        let _ = repo.stash_save(path, msg.as_deref());
                        if let Ok(stashes) = repo.list_stashes(path) {
                            let mut next_state = prev_state.clone();
                            if let AppState::ViewingStashes { stashes: s, .. } = &mut next_state {
                                *s = stashes;
                            }
                            return Ok(Some(next_state));
                        }
                    }
                }
            }
            return Ok(Some(prev_state.clone()));
        }
        KeyCode::Esc => {
            return Ok(Some(prev_state.clone()));
        }
        KeyCode::Char(c) => input.push(c),
        KeyCode::Backspace => {
            input.pop();
        }
        _ => {}
    }
    Ok(None)
}
