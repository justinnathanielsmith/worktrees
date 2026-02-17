use crate::app::model::{AppState, EditorConfig};
use crate::domain::repository::ProjectRepository;
use std::process::Command;

use super::helpers::create_timed_state;

pub fn handle_editor_events<R: ProjectRepository>(
    event: &crossterm::event::Event,
    repo: &R,
    branch: &str,
    options: &[EditorConfig],
    selected: &mut usize,
    prev_state: &AppState,
) -> Option<AppState> {
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
        KeyCode::Up | KeyCode::Char('k') => {
            *selected = if *selected > 0 {
                *selected - 1
            } else {
                options.len() - 1
            };
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *selected = if *selected < options.len() - 1 {
                *selected + 1
            } else {
                0
            };
        }
        KeyCode::Enter => {
            let editor = options[*selected].command.clone();
            let _ = repo.set_preferred_editor(&editor);
            let path = if let AppState::ListingWorktrees {
                worktrees,
                table_state,
                ..
            } = prev_state
            {
                table_state
                    .selected()
                    .and_then(|i| worktrees.get(i))
                    .map(|wt| wt.path.clone())
            } else {
                None
            };
            if let Some(p) = path {
                let prev_clone = prev_state.clone();
                let opening_state = AppState::OpeningEditor {
                    branch: branch.to_string(),
                    editor,
                    prev_state: Box::new(prev_state.clone()),
                };
                let _ = Command::new(&options[*selected].command).arg(&p).spawn();
                return Some(create_timed_state(opening_state, prev_clone, 800));
            }
        }
        KeyCode::Esc => {
            return Some(prev_state.clone());
        }
        _ => {}
    }
    None
}
