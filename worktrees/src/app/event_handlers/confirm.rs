use crate::app::intent::Intent;
use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_confirm_events<R: ProjectRepository>(
    key_code: KeyCode,
    repo: &R,
    action: &Intent,
    prev_state: &AppState,
) -> Result<Option<AppState>> {
    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Enter | KeyCode::Char('y') => {
            // Execute action
            if let Intent::RemoveWorktree { intent } = action {
                let _ = repo.remove_worktree(intent, false);
            }
            Ok(Some(prev_state.clone()))
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => Ok(Some(prev_state.clone())),
        _ => Ok(None),
    }
}
