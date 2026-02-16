use crate::app::model::AppState;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_history_events(
    key_code: KeyCode,
    commits: &[crate::domain::repository::GitCommit],
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
            if !commits.is_empty() {
                *selected_index = (*selected_index + 1) % commits.len();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !commits.is_empty() {
                *selected_index = (*selected_index + commits.len() - 1) % commits.len();
            }
        }
        _ => {}
    }
    Ok(None)
}
