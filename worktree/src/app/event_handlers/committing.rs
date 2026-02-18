use crate::app::model::{AppState, PromptType};
use crate::domain::repository::ProjectRepository;

#[allow(clippy::too_many_arguments)]
pub fn handle_committing_events<R: ProjectRepository + Clone + Send + Sync + 'static>(
    event: &crossterm::event::Event,
    repo: &R,
    path: &str,
    branch: &str,
    selected_index: &mut usize,
    prev_state: &AppState,
    current_state: &AppState,
    async_tx: &tokio::sync::mpsc::UnboundedSender<crate::app::async_tasks::AsyncResult>,
) -> Option<AppState> {
    use crate::app::async_tasks::AsyncResult;
    use crossterm::event::{Event, KeyCode};
    let key_code = if let Event::Key(key) = event {
        key.code
    } else {
        return None;
    };

    let options = ["Manual Commit", "AI Commit", "Set API Key"];
    let normalized_code = match key_code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        _ => key_code,
    };

    match normalized_code {
        KeyCode::Esc | KeyCode::Char('q') => {
            return Some(prev_state.clone());
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
                return Some(AppState::Prompting {
                    prompt_type: PromptType::CommitMessage,
                    input: String::new(),
                    prev_state: Box::new(current_state.clone()),
                });
            }
            1 => {
                // AI
                let repo_clone = repo.clone();
                let path_clone = path.to_string();
                let branch_clone = branch.to_string();
                let tx = async_tx.clone();

                tokio::task::spawn_blocking(move || {
                    let result = (|| -> anyhow::Result<String> {
                        let diff = repo_clone.get_diff(&path_clone)?;
                        if diff.trim().is_empty() {
                            return Err(anyhow::anyhow!("No changes detected."));
                        }
                        repo_clone.generate_commit_message(&diff, &branch_clone)
                    })();

                    let _ = tx.send(AsyncResult::CommitMessageGenerated { result });
                });

                return Some(AppState::GeneratingCommitMessage {
                    prev_state: Box::new(current_state.clone()),
                });
            }
            2 => {
                // Set API Key
                return Some(AppState::Prompting {
                    prompt_type: PromptType::ApiKey,
                    input: String::new(),
                    prev_state: Box::new(current_state.clone()),
                });
            }
            _ => {}
        },
        _ => {}
    }
    None
}
