use crate::app::model::{AppState, EditorConfig};
use crate::domain::repository::ProjectRepository;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::process::Command;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn handle_editor_events<R: ProjectRepository>(
    key_code: KeyCode,
    repo: &R,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    branch: &str,
    options: &[EditorConfig],
    selected: &mut usize,
    prev_state: &AppState,
    spinner_tick: &mut usize,
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
                let mut opening_state = AppState::OpeningEditor {
                    branch: branch.to_string(),
                    editor,
                    prev_state: Box::new(prev_state.clone()),
                };
                terminal.draw(|f| {
                    super::super::view::View::draw(f, repo, &mut opening_state, *spinner_tick)
                })?;
                let _ = Command::new(&options[*selected].command).arg(&p).spawn();
                std::thread::sleep(Duration::from_millis(800));
                return Ok(Some(prev_state.clone()));
            }
        }
        KeyCode::Esc => {
            return Ok(Some(prev_state.clone()));
        }
        _ => {}
    }
    Ok(None)
}
