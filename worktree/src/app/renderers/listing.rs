use crate::app::model::DashboardTab;
use crate::app::renderers::dashboard::render_dashboard;
use crate::domain::repository::{GitCommit, GitStatus, ProjectContext, Worktree};
use crate::ui::widgets::worktree_list::WorktreeListWidget;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, TableState},
};

#[allow(clippy::too_many_arguments)]
pub fn render_listing(
    f: &mut Frame,
    worktrees: &[Worktree],
    filtered_indices: &[usize],
    table_state: &mut TableState,
    context: ProjectContext,
    area: Rect,
    active_tab: DashboardTab,
    status: Option<&GitStatus>,
    history: Option<&[GitCommit]>,
    filter_query: &str,
    is_filtering: bool,
    mode: crate::app::model::AppMode,
    spinner_tick: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let (list_area, search_area) = if is_filtering || !filter_query.is_empty() {
        let sub_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(chunks[0]);
        (sub_chunks[0], Some(sub_chunks[1]))
    } else {
        (chunks[0], None)
    };

    // Dim the list if we are searching/filtering
    let is_dimmed = is_filtering;

    let table = WorktreeListWidget::new(worktrees, Some(filtered_indices))
        .dimmed(is_dimmed)
        .tick(spinner_tick)
        .with_filter(if !filter_query.is_empty() {
            Some(filter_query)
        } else {
            None
        })
        .with_mode(mode);

    f.render_stateful_widget(table, list_area, table_state);

    if let Some(area) = search_area {
        let border_style = if is_filtering {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let text = if filter_query.is_empty() && is_filtering {
            "Type to filter..."
        } else {
            filter_query
        };

        let search = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Filter ")
                    .border_style(border_style),
            )
            .style(if filter_query.is_empty() && is_filtering {
                Style::default().add_modifier(Modifier::DIM)
            } else {
                Style::default()
            });
        f.render_widget(search, area);

        if is_filtering {
            let width = Line::from(filter_query).width() as u16;
            // Cursor position relative to the area
            // x: area.x + 1 (border) + width of text
            // y: area.y + 1 (border)
            let max_width = area.width.saturating_sub(2);
            let cursor_x = area.x + 1 + width.min(max_width);
            let cursor_y = area.y + 1;
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    let selected_worktree = table_state
        .selected()
        .and_then(|i| filtered_indices.get(i))
        .and_then(|&idx| worktrees.get(idx));

    // Auto-switch tabs based on mode
    let effective_tab = match mode {
        crate::app::model::AppMode::Git => DashboardTab::Status,
        crate::app::model::AppMode::Manage => DashboardTab::Info,
        _ => active_tab,
    };

    render_dashboard(
        f,
        selected_worktree,
        worktrees,
        context,
        effective_tab,
        status,
        history,
        chunks[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::Backend, backend::TestBackend};

    #[test]
    fn test_render_listing_sets_cursor_when_filtering() {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut table_state = TableState::default();
        let worktrees = vec![];
        let filtered_indices = vec![];
        let context = crate::domain::repository::ProjectContext::Standard;
        let active_tab = DashboardTab::Info;
        let mode = crate::app::model::AppMode::Filter;

        terminal
            .draw(|f| {
                let area = f.area();
                render_listing(
                    f,
                    &worktrees,
                    &filtered_indices,
                    &mut table_state,
                    context,
                    area,
                    active_tab,
                    None,
                    None,
                    "foo", // filter_query
                    true,  // is_filtering
                    mode,
                    0,
                );
            })
            .unwrap();

        // Verify cursor position
        let pos = terminal.backend_mut().get_cursor_position().unwrap();
        // Default cursor is (0, 0)
        assert_eq!(pos.x, 4, "Cursor X position mismatch");
        assert_eq!(pos.y, 18, "Cursor Y position mismatch");
    }
}
