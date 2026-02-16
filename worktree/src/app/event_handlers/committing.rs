use crate::app::model::{AppState, PromptType};
use crate::domain::repository::ProjectRepository;
use anyhow::Result;

pub fn handle_committing_events<R: ProjectRepository>(
    event: crossterm::event::Event,
    repo: &R,
    path: &str,
    branch: &str,
    selected_index: &mut usize,
    prev_state: &AppState,
    current_state: &AppState,
) -> Result<Option<AppState>> {
    use crossterm::event::{Event, KeyCode};
    let key_code = if let Event::Key(key) = event {
        key.code
    } else {
        return Ok(None);
    };

    let options = ["Manual Commit", "AI Commit", "Set API Key"];
    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Esc | KeyCode::Char('q') => {
            return Ok(Some(prev_state.clone()));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *selected_index = (*selected_index + 1) % options.len();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            *selected_index = (*selected_index + options.len() - 1) % options.len();
        }
        KeyCode::Enter => match *selected_index {
            0 => {
                // Manual
                return Ok(Some(AppState::Prompting {
                    prompt_type: PromptType::CommitMessage,
                    input: String::new(),
                    prev_state: Box::new(current_state.clone()),
                }));
            }
            1 => {
                // AI
                match repo.get_diff(path) {
                    Ok(diff) => {
                        if diff.trim().is_empty() {
                            return Ok(Some(AppState::Error(
                                "No changes detected to generate a message for.".into(),
                                Box::new(current_state.clone()),
                            )));
                        }
                        match repo.generate_commit_message(&diff, branch) {
                            Ok(msg) => {
                                return Ok(Some(AppState::Prompting {
                                    prompt_type: PromptType::CommitMessage,
                                    input: msg,
                                    prev_state: Box::new(current_state.clone()),
                                }));
                            }
                            Err(e) => {
                                return Ok(Some(AppState::Error(
                                    format!("AI Generation failed: {}", e),
                                    Box::new(current_state.clone()),
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        return Ok(Some(AppState::Error(
                            format!("Failed to get diff: {}", e),
                            Box::new(current_state.clone()),
                        )));
                    }
                }
            }
            2 => {
                // Set API Key
                return Ok(Some(AppState::Prompting {
                    prompt_type: PromptType::ApiKey,
                    input: String::new(),
                    prev_state: Box::new(current_state.clone()),
                }));
            }
            _ => {}
        },
        _ => {}
    }
    Ok(None)
}
