use crate::domain::repository::{ProjectContext, Worktree};
use crate::ui::widgets::{details::DetailsWidget, worktree_list::WorktreeListWidget};
use ratatui::{Frame, layout::Rect, widgets::TableState};

pub fn render_listing(
    f: &mut Frame,
    worktrees: &[Worktree],
    table_state: &mut TableState,
    context: ProjectContext,
    chunks: std::rc::Rc<[Rect]>,
) {
    let table = WorktreeListWidget::new(worktrees);
    f.render_stateful_widget(table, chunks[1], table_state);

    let selected_worktree = table_state.selected().and_then(|i| worktrees.get(i));

    f.render_widget(DetailsWidget::new(selected_worktree, context), chunks[2]);
}
