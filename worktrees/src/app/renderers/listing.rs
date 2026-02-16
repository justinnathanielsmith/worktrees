use crate::app::model::{filter_worktrees, DashboardTab};
use crate::app::renderers::dashboard::render_dashboard;
use crate::domain::repository::{GitCommit, GitStatus, ProjectContext, Worktree};
use crate::ui::widgets::worktree_list::WorktreeListWidget;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, TableState},
    Frame,
};

#[allow(clippy::too_many_arguments)]
pub fn render_listing(
    f: &mut Frame,
    worktrees: &[Worktree],
    table_state: &mut TableState,
    context: ProjectContext,
    area: Rect,
    active_tab: DashboardTab,
    status: &Option<GitStatus>,
    history: &Option<Vec<GitCommit>>,
    filter_query: &str,
    is_filtering: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let filtered_worktrees = filter_worktrees(worktrees, filter_query);

    let (list_area, search_area) = if is_filtering || !filter_query.is_empty() {
        let sub_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(chunks[0]);
        (sub_chunks[0], Some(sub_chunks[1]))
    } else {
        (chunks[0], None)
    };

    let table = WorktreeListWidget::new(&filtered_worktrees);
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
    }

    let selected_worktree = table_state
        .selected()
        .and_then(|i| filtered_worktrees.get(i));

    render_dashboard(
        f,
        selected_worktree,
        context,
        active_tab,
        status,
        history,
        chunks[1],
    );
}
