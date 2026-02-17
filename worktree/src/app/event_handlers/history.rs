use crate::app::model::AppState;

#[allow(clippy::too_many_arguments)]
pub fn handle_history_events(
    event: &crossterm::event::Event,
    commits: &[crate::domain::repository::GitCommit],
    selected_index: &mut usize,
    prev_state: &AppState,
) -> Option<AppState> {
    use crate::app::renderers::helpers::centered_rect;
    use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
    use ratatui::layout::Rect;

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
                    if !commits.is_empty() {
                        let mut next = (*selected_index + 1) % commits.len();
                        // Skip graph-only lines
                        while next != *selected_index && commits[next].hash.is_empty() {
                            next = (next + 1) % commits.len();
                        }
                        *selected_index = next;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !commits.is_empty() {
                        let mut prev = (*selected_index + commits.len() - 1) % commits.len();
                        // Skip graph-only lines
                        while prev != *selected_index && commits[prev].hash.is_empty() {
                            prev = (prev + commits.len() - 1) % commits.len();
                        }
                        *selected_index = prev;
                    }
                }
                _ => {}
            }
        }
        Event::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Ok((w, h)) = crossterm::terminal::size() {
                        let area = Rect::new(0, 0, w, h);
                        let popup_area = centered_rect(85, 80, area);

                        // Inner area calculation (borders)
                        let inner_x = popup_area.x + 1;
                        let inner_y = popup_area.y + 1;
                        let inner_w = popup_area.width.saturating_sub(2);
                        let inner_h = popup_area.height.saturating_sub(2);

                        let col = mouse.column;
                        let row = mouse.row;

                        if col >= inner_x
                            && col < inner_x + inner_w
                            && row >= inner_y
                            && row < inner_y + inner_h
                        {
                            let relative_y = row.saturating_sub(inner_y) as usize;
                            if relative_y < commits.len() && !commits[relative_y].hash.is_empty() {
                                *selected_index = relative_y;
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => {
                    if !commits.is_empty() {
                        *selected_index = (*selected_index + 1) % commits.len();
                    }
                }
                MouseEventKind::ScrollUp => {
                    if !commits.is_empty() {
                        *selected_index = (*selected_index + commits.len() - 1) % commits.len();
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}
