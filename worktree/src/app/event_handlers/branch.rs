use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;

pub fn handle_branch_events<R: ProjectRepository + Clone + Send + Sync + 'static>(
    event: &crossterm::event::Event,
    repo: &R,
    path: &str,
    branches: &[String],
    selected_index: &mut usize,
    prev_state: &AppState,
    async_tx: &tokio::sync::mpsc::UnboundedSender<crate::app::async_tasks::AsyncResult>,
) -> Option<AppState> {
    use crate::app::async_tasks::AsyncResult;
    use crossterm::event::{Event, KeyCode};
    let key_code = if let Event::Key(key) = event {
        key.code
    } else {
        return None;
    };

    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Esc | KeyCode::Char('q') => {
            return Some(prev_state.clone());
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !branches.is_empty() {
                *selected_index = (*selected_index + 1) % branches.len();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !branches.is_empty() {
                *selected_index = (*selected_index + branches.len() - 1) % branches.len();
            }
        }
        KeyCode::Enter => {
            if let Some(branch) = branches.get(*selected_index) {
                let repo_clone = repo.clone();
                let path_clone = path.to_string();
                let branch_clone = branch.clone();
                let tx = async_tx.clone();

                tokio::task::spawn_blocking(move || {
                    let result = repo_clone.switch_branch(&path_clone, &branch_clone);
                    let _ = tx.send(AsyncResult::BranchSwitched {
                        path: path_clone,
                        result,
                    });
                });
                return Some(AppState::SwitchingBranchTask {
                    path: path.to_string(),
                    prev_state: Box::new(prev_state.clone()),
                });
            }
            return Some(prev_state.clone());
        }
        _ => {}
    }
    None
}
