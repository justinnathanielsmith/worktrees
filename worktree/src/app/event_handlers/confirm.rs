use crate::app::intent::Intent;
use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use anyhow::Result;

pub fn handle_confirm_events<R: ProjectRepository>(
    event: crossterm::event::Event,
    repo: &R,
    action: &Intent,
    prev_state: &AppState,
) -> Result<Option<AppState>> {
    use crossterm::event::{Event, KeyCode};
    let key_code = if let Event::Key(key) = event {
        key.code
    } else {
        return Ok(None);
    };

    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Enter | KeyCode::Char('y') => {
            // Execute action
            if let Intent::RemoveWorktree { intent, force } = action {
                if let Err(e) = repo.remove_worktree(intent, *force) {
                    return Ok(Some(AppState::Error(
                        format!("Failed to remove worktree: {}", e),
                        Box::new(prev_state.clone()),
                    )));
                }
            }
            Ok(Some(prev_state.clone()))
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => Ok(Some(prev_state.clone())),
        _ => Ok(None),
    }
}
