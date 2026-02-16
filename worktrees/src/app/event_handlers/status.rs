use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_status_events<R: ProjectRepository>(
    key_code: KeyCode,
    repo: &R,
    path: &str,
    branch: &str,
    status: &mut crate::app::model::StatusViewState,
    prev_state: &AppState,
    current_state: &AppState,
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
            let total = status.total();
            if total > 0 {
                status.selected_index = (status.selected_index + 1) % total;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let total = status.total();
            if total > 0 {
                status.selected_index = (status.selected_index + total - 1) % total;
            }
        }
        KeyCode::Char(' ') => {
            let idx = status.selected_index;
            if idx < status.staged.len() {
                let _ = repo.unstage_file(path, &status.staged[idx]);
            } else if idx < status.staged.len() + status.unstaged.len() {
                let _ = repo.stage_file(path, &status.unstaged[idx - status.staged.len()]);
            } else if idx < status.total() {
                let _ = repo.stage_file(
                    path,
                    &status.untracked[idx - status.staged.len() - status.unstaged.len()],
                );
            }
            if let Ok(new_status) = repo.get_status(path) {
                status.staged = new_status.staged;
                status.unstaged = new_status.unstaged;
                status.untracked = new_status.untracked;
                let new_total = status.total();
                if new_total > 0 && status.selected_index >= new_total {
                    status.selected_index = new_total - 1;
                }
            }
        }
        KeyCode::Char('c') => {
            return Ok(Some(AppState::Committing {
                path: path.to_string(),
                branch: branch.to_string(),
                selected_index: 0,
                prev_state: Box::new(current_state.clone()),
            }));
        }
        KeyCode::Char('a') => {
            let _ = repo.stage_all(path);
            if let Ok(new_status) = repo.get_status(path) {
                status.staged = new_status.staged;
                status.unstaged = new_status.unstaged;
                status.untracked = new_status.untracked;
                let new_total = status.total();
                if new_total > 0 && status.selected_index >= new_total {
                    status.selected_index = new_total - 1;
                }
            }
        }
        KeyCode::Char('u') => {
            let _ = repo.unstage_all(path);
            if let Ok(new_status) = repo.get_status(path) {
                status.staged = new_status.staged;
                status.unstaged = new_status.unstaged;
                status.untracked = new_status.untracked;
                let new_total = status.total();
                if new_total > 0 && status.selected_index >= new_total {
                    status.selected_index = new_total - 1;
                }
            }
        }
        KeyCode::Char('d') => {
            // Toggle diff preview
            status.show_diff = !status.show_diff;

            // Load diff if showing and we have a selected file
            if status.show_diff && status.selected_file().is_some() {
                status.diff_preview = repo.get_diff(path).ok();
            }
        }
        KeyCode::Char('r') => {
            // Refresh status
            if let Ok(new_status) = repo.get_status(path) {
                status.staged = new_status.staged;
                status.unstaged = new_status.unstaged;
                status.untracked = new_status.untracked;
                let new_total = status.total();
                if new_total > 0 && status.selected_index >= new_total {
                    status.selected_index = new_total - 1;
                }

                // Refresh diff if showing
                if status.show_diff && status.selected_file().is_some() {
                    status.diff_preview = repo.get_diff(path).ok();
                }
            }
        }
        _ => {}
    }

    // Update diff preview when selection changes
    if status.show_diff && status.selected_file().is_some() {
        status.diff_preview = repo.get_diff(path).ok();
    }

    Ok(None)
}
