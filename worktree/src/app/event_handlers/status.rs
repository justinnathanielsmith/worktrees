use crate::app::model::AppState;
use crate::domain::repository::ProjectRepository;

#[allow(clippy::too_many_arguments)]
pub fn handle_status_events<R: ProjectRepository>(
    event: &crossterm::event::Event,
    repo: &R,
    path: &str,
    branch: &str,
    status: &mut crate::app::model::StatusViewState,
    prev_state: &AppState,
    current_state: &AppState,
) -> Option<AppState> {
    use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
    use ratatui::layout::{Constraint, Direction, Layout, Rect};

    // Calculate layout for mouse hit testing
    // Note: We need the terminal size. Since we don't have it passed here easily (unlike listing),
    // we might need to rely on Key events or just accept that this handler needs terminal size.
    // However, looking at signature, we don't have terminal here.
    // `listing` has terminal passed in.
    // `status` does NOT.
    // I should update signature to include terminal if I want accurate mouse support,
    // OR I can use `crossterm::terminal::size()` to get it directly.

    // For now, let's update signature to include terminal in valid `mod.rs` in next step if needed.
    // But `listing.rs` has it. `status.rs` does not.
    // `view.rs` calls it. `view.rs` has `terminal` avail in `run_loop`.
    // So I will update signature to take terminal, or just use `crossterm::terminal::size()`.
    // Using `crossterm::terminal::size()` is easier for now to avoid changing every caller in view.rs if not needed,
    // but correct way is passing it.
    // But wait, `view.rs` calls `handle_status_events`. I will update `view.rs` anyway.

    match event {
        Event::Key(key) => {
            let key_code = key.code;
            let normalized_code = match key_code {
                KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                _ => key_code,
            };

            match normalized_code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    return Some(prev_state.clone());
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
                        let _ = repo.unstage_file(path, &status.staged[idx].0);
                    } else if idx < status.staged.len() + status.unstaged.len() {
                        let _ =
                            repo.stage_file(path, &status.unstaged[idx - status.staged.len()].0);
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
                    return Some(AppState::Committing {
                        path: path.to_string(),
                        branch: branch.to_string(),
                        selected_index: 0,
                        prev_state: Box::new(current_state.clone()),
                    });
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
                KeyCode::Char('s') => {
                    if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                        if let Ok(stashes) = repo.list_stashes(path) {
                            return Some(AppState::ViewingStashes {
                                path: path.to_string(),
                                branch: branch.to_string(),
                                stashes,
                                selected_index: 0,
                                prev_state: Box::new(current_state.clone()),
                            });
                        }
                    }
                }
                _ => {}
            }

            // Update diff preview when selection changes
            if status.show_diff && status.selected_file().is_some() {
                status.diff_preview = repo.get_diff(path).ok();
            }
        }
        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // Start of mouse click handling
                    if let Ok((w, h)) = crossterm::terminal::size() {
                        let area = Rect::new(0, 0, w, h);

                        // Replicate layout logic from render_status
                        let body_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                            .split(area);

                        let main_area = body_chunks[1];
                        let main_chunks = if status.show_diff {
                            Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([
                                    Constraint::Percentage(60),
                                    Constraint::Percentage(40),
                                ])
                                .split(main_area)
                        } else {
                            Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([Constraint::Percentage(100)])
                                .split(main_area)
                        };

                        let status_area = main_chunks[0];
                        let inner_area = Rect::new(
                            status_area.x + 1,
                            status_area.y + 1,
                            status_area.width.saturating_sub(2),
                            status_area.height.saturating_sub(2),
                        );

                        let status_layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Length(3), Constraint::Min(0)])
                            .split(inner_area);

                        let file_area = status_layout[1];
                        let file_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .split(file_area);

                        let col = mouse.column;
                        let row = mouse.row;

                        // Check Staged Column
                        let staged_rect = file_chunks[0];
                        if col >= staged_rect.x
                            && col < staged_rect.x + staged_rect.width
                            && row >= staged_rect.y
                            && row < staged_rect.y + staged_rect.height
                        {
                            let relative_y = row.saturating_sub(staged_rect.y);
                            if (relative_y as usize) < status.staged.len() {
                                status.selected_index = relative_y as usize;
                                if status.show_diff {
                                    status.diff_preview = repo.get_diff(path).ok();
                                }
                            }
                        }
                        // Check Unstaged/Untracked Column
                        else if col >= file_chunks[1].x
                            && col < file_chunks[1].x + file_chunks[1].width
                            && row >= file_chunks[1].y
                            && row < file_chunks[1].y + file_chunks[1].height
                        {
                            let relative_y = row.saturating_sub(file_chunks[1].y);
                            let unstaged_len = status.unstaged.len();
                            let untracked_len = status.untracked.len();
                            if (relative_y as usize) < unstaged_len + untracked_len {
                                status.selected_index = status.staged.len() + relative_y as usize;
                                if status.show_diff {
                                    status.diff_preview = repo.get_diff(path).ok();
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => {
                    let total = status.total();
                    if total > 0 {
                        status.selected_index = (status.selected_index + 1) % total;
                        if status.show_diff {
                            status.diff_preview = repo.get_diff(path).ok();
                        }
                    }
                }
                MouseEventKind::ScrollUp => {
                    let total = status.total();
                    if total > 0 {
                        status.selected_index = (status.selected_index + total - 1) % total;
                        if status.show_diff {
                            status.diff_preview = repo.get_diff(path).ok();
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    None
}
