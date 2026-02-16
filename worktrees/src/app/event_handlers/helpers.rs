use crate::app::model::AppState;
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};

/// Helper function to create a Timed state transition.
pub fn create_timed_state(inner: AppState, target: AppState, duration_ms: u64) -> AppState {
    AppState::Timed {
        inner_state: Box::new(inner),
        target_state: Box::new(target),
        start_time: Instant::now(),
        duration: Duration::from_millis(duration_ms),
    }
}

/// Helper function to move table selection up or down with wrapping.
pub fn move_selection(state: &mut TableState, len: usize, delta: isize) {
    if len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) => {
            let next = i as isize + delta;
            if next < 0 {
                len - 1
            } else if next >= len as isize {
                0
            } else {
                next as usize
            }
        }
        None => 0,
    };
    state.select(Some(i));
}
