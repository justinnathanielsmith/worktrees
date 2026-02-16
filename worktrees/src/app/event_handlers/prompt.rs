use crate::app::model::{AppState, PromptType};
use crate::domain::repository::ProjectRepository;
use anyhow::Result;
use ratatui::{Terminal, backend::CrosstermBackend, widgets::TableState};
use std::io;

pub fn handle_prompt_events<R: ProjectRepository>(
    event: crossterm::event::Event,
    repo: &R,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    prompt_type: &PromptType,
    input: &mut String,
    prev_state: &AppState,
    spinner_tick: &mut usize,
) -> Result<Option<AppState>> {
    use crossterm::event::{Event, KeyCode};
    let key_code = if let Event::Key(key) = event {
        key.code
    } else {
        return Ok(None);
    };

    match key_code {
        KeyCode::Enter => {
            let val = input.trim().to_string();
            if !val.is_empty() {
                match prompt_type {
                    PromptType::AddIntent => {
                        let mut adding_state = AppState::AddingWorktree {
                            intent: val.clone(),
                            branch: val.clone(),
                        };
                        terminal.draw(|f| {
                            super::super::view::View::draw(
                                f,
                                repo,
                                &mut adding_state,
                                *spinner_tick,
                            )
                        })?;
                        let _ = repo.add_worktree(&val, &val);
                        return Ok(Some(AppState::ListingWorktrees {
                            worktrees: Vec::new(),
                            table_state: TableState::default(),
                            refresh_needed: true,
                            selection_mode: false,
                            dashboard: crate::app::model::DashboardState {
                                active_tab: crate::app::model::DashboardTab::Info,
                                cached_status: None,
                                cached_history: None,
                            },
                        }));
                    }
                    PromptType::CommitMessage => {
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
                        return Ok(Some(prev_state.clone()));
                    }
                    PromptType::ApiKey => {
                        let _ = repo.set_api_key(&val);
                        return Ok(Some(prev_state.clone()));
                    }
                }
            } else {
                return Ok(Some(prev_state.clone()));
            }
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
