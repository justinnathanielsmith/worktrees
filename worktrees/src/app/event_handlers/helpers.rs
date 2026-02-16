use ratatui::widgets::TableState;

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
