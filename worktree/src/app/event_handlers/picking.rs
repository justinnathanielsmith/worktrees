use crate::app::model::{AppState, PromptType};
use crossterm::event::{Event, KeyCode};

pub fn handle_picking_ref_events(
    event: &Event,
    branches: &[String],
    selected_index: &mut usize,
    prev_state: &AppState,
) -> Option<AppState> {
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
            // Cancel and return to previous state
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
                // Transition to Prompting state
                return Some(AppState::Prompting {
                    prompt_type: PromptType::NameNewWorktree {
                        base_ref: branch.clone(),
                    },
                    input: String::new(),
                    prev_state: Box::new(prev_state.clone()),
                });
            }
        }
        _ => {}
    }
    None
}
