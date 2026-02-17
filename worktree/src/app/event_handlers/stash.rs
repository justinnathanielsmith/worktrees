use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use crossterm::event::{Event, KeyCode, KeyEvent};

#[allow(clippy::too_many_arguments)]
pub fn handle_stash_events<R: ProjectRepository>(
    event: &Event,
    repo: &R,
    path: &str,
    branch: &str,
    stashes: &[crate::domain::repository::StashEntry],
    selected_index: &usize,
    prev_state: &AppState,
    _current_state: &AppState,
) -> Option<AppState> {
    if let Event::Key(KeyEvent { code, .. }) = event {
        match code {
            KeyCode::Esc => return Some(prev_state.clone()),
            KeyCode::Up | KeyCode::Char('k') => {
                let new_index = if *selected_index > 0 {
                    selected_index - 1
                } else {
                    stashes.len().saturating_sub(1)
                };
                return Some(AppState::ViewingStashes {
                    path: path.to_string(),
                    branch: branch.to_string(),
                    stashes: stashes.to_vec(),
                    selected_index: new_index,
                    prev_state: Box::new(prev_state.clone()),
                });
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let new_index = if !stashes.is_empty() {
                    (selected_index + 1) % stashes.len()
                } else {
                    0
                };
                return Some(AppState::ViewingStashes {
                    path: path.to_string(),
                    branch: branch.to_string(),
                    stashes: stashes.to_vec(),
                    selected_index: new_index,
                    prev_state: Box::new(prev_state.clone()),
                });
            }
            KeyCode::Char('a') => {
                if let Some(stash) = stashes.get(*selected_index) {
                    if let Err(e) = repo.apply_stash(path, stash.index) {
                        return Some(AppState::Error(
                            format!("Failed to apply stash: {e}"),
                            Box::new(_current_state.clone()),
                        ));
                    }
                    return Some(prev_state.clone());
                }
            }
            KeyCode::Char('p') => {
                if let Some(stash) = stashes.get(*selected_index) {
                    if let Err(e) = repo.pop_stash(path, stash.index) {
                        return Some(AppState::Error(
                            format!("Failed to pop stash: {e}"),
                            Box::new(_current_state.clone()),
                        ));
                    }
                    return Some(prev_state.clone());
                }
            }
            KeyCode::Char('d') => {
                if let Some(stash) = stashes.get(*selected_index) {
                    if let Err(e) = repo.drop_stash(path, stash.index) {
                        return Some(AppState::Error(
                            format!("Failed to drop stash: {e}"),
                            Box::new(_current_state.clone()),
                        ));
                    }
                    if let Ok(new_stashes) = repo.list_stashes(path) {
                        let new_index = *selected_index.min(&new_stashes.len().saturating_sub(1));
                        return Some(AppState::ViewingStashes {
                            path: path.to_string(),
                            branch: branch.to_string(),
                            stashes: new_stashes,
                            selected_index: new_index,
                            prev_state: Box::new(prev_state.clone()),
                        });
                    }
                }
            }
            KeyCode::Char('n') => {
                return Some(AppState::Prompting {
                    prompt_type: crate::app::model::PromptType::StashMessage,
                    input: String::new(),
                    prev_state: Box::new(_current_state.clone()),
                });
            }
            _ => {}
        }
    }
    None
}
