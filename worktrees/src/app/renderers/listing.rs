use crate::app::model::DashboardTab;
use crate::app::renderers::dashboard::render_dashboard;
use crate::domain::repository::{GitCommit, GitStatus, ProjectContext, Worktree};
use crate::ui::widgets::worktree_list::WorktreeListWidget;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::TableState,
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
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let table = WorktreeListWidget::new(worktrees);
    f.render_stateful_widget(table, chunks[0], table_state);

    let selected_worktree = table_state.selected().and_then(|i| worktrees.get(i));

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
