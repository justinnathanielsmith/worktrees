use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_branch_events<R: ProjectRepository>(
    key_code: KeyCode,
    repo: &R,
    path: &str,
    branches: &[String],
    selected_index: &mut usize,
    prev_state: &AppState,
) -> Result<Option<AppState>> {
    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Esc | KeyCode::Char('q') => {
            return Ok(Some(prev_state.clone()));
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
                let _ = repo.switch_branch(path, branch);
            }
            return Ok(Some(prev_state.clone()));
        }
        _ => {}
    }
    Ok(None)
}
