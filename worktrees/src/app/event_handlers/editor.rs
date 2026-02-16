use crate::app::model::{AppState, EditorConfig};
use crate::domain::repository::ProjectRepository;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::process::Command;

#[allow(clippy::too_many_arguments)]
pub fn handle_editor_events<R: ProjectRepository>(
    key_code: KeyCode,
    repo: &R,
    _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    branch: &str,
    options: &[EditorConfig],
    selected: &mut usize,
    prev_state: &AppState,
    _spinner_tick: &mut usize,
) -> Result<Option<AppState>> {
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
                return Ok(Some(AppState::Timed {
                    inner_state: Box::new(opening_state),
                    target_state: Box::new(prev_clone),
                    start_time: std::time::Instant::now(),
                    duration: std::time::Duration::from_millis(800),
                }));
            }
        }
        KeyCode::Esc => {
            return Ok(Some(prev_state.clone()));
        }
        _ => {}
    }
    Ok(None)
}
