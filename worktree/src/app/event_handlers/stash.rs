use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use crossterm::event::{Event, KeyCode, KeyEvent};

#[allow(clippy::too_many_arguments)]
pub fn handle_stash_events<R: ProjectRepository + Clone + Send + Sync + 'static>(
    event: &Event,
    repo: &R,
    path: &str,
    branch: &str,
    stashes: &[crate::domain::repository::StashEntry],
    selected_index: &usize,
    prev_state: &AppState,
    _current_state: &AppState,
    async_tx: &tokio::sync::mpsc::UnboundedSender<crate::app::async_tasks::AsyncResult>,
) -> Option<AppState> {
    use crate::app::async_tasks::AsyncResult;
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
                    let repo_clone = repo.clone();
                    let path_clone = path.to_string();
                    let idx = stash.index;
                    let tx = async_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = repo_clone.apply_stash(&path_clone, idx);
                        let _ = tx.send(AsyncResult::StashApplied { result });
                    });
                    return Some(AppState::StashAction {
                        message: "Applying stash...".into(),
                        prev_state: Box::new(_current_state.clone()),
                    });
                }
            }
            KeyCode::Char('p') => {
                if let Some(stash) = stashes.get(*selected_index) {
                    let repo_clone = repo.clone();
                    let path_clone = path.to_string();
                    let idx = stash.index;
                    let tx = async_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = repo_clone.pop_stash(&path_clone, idx);
                        let _ = tx.send(AsyncResult::StashPopped { result });
                    });
                    return Some(AppState::StashAction {
                        message: "Popping stash...".into(),
                        prev_state: Box::new(_current_state.clone()),
                    });
                }
            }
            KeyCode::Char('d') => {
                if let Some(stash) = stashes.get(*selected_index) {
                    let repo_clone = repo.clone();
                    let path_clone = path.to_string();
                    let idx = stash.index;
                    let tx = async_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = repo_clone.drop_stash(&path_clone, idx);
                        let _ = tx.send(AsyncResult::StashDropped { result });
                    });
                    return Some(AppState::StashAction {
                        message: "Dropping stash...".into(),
                        prev_state: Box::new(_current_state.clone()),
                    });
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
