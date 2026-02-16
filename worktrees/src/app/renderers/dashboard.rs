use crate::app::model::DashboardTab;
use crate::domain::repository::{GitCommit, GitStatus, ProjectContext, Worktree};
use crate::ui::widgets::dashboard::DashboardWidget;
use ratatui::{Frame, layout::Rect};

pub fn render_dashboard(
    f: &mut Frame,
    worktree: Option<&Worktree>,
    context: ProjectContext,
    active_tab: DashboardTab,
    status: &Option<GitStatus>,
    history: &Option<Vec<GitCommit>>,
    area: Rect,
) {
    let widget = DashboardWidget::new(worktree, context, active_tab, status, history);
    f.render_widget(widget, area);
}
