use crate::domain::repository::Worktree;
use crate::ui::theme::{CyberTheme, Icons};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, StatefulWidget, Table, TableState},
};

pub struct WorktreeListWidget<'a> {
    worktrees: &'a [Worktree],
}

impl<'a> WorktreeListWidget<'a> {
    pub fn new(worktrees: &'a [Worktree]) -> Self {
        Self { worktrees }
    }
}

impl<'a> StatefulWidget for WorktreeListWidget<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let theme = CyberTheme::default();

        let rows: Vec<Row> = self
            .worktrees
            .iter()
            .enumerate()
            .map(|(i, wt)| {
                let is_selected = Some(i) == state.selected();

                let (icon, branch_style) = if wt.is_bare {
                    (
                        Icons::BARE,
                        Style::default()
                            .fg(theme.subtle)
                            .add_modifier(Modifier::ITALIC),
                    )
                } else if wt.is_detached {
                    (Icons::DETACHED, Style::default().fg(theme.error))
                } else {
                    (Icons::WORKTREE, Style::default().fg(theme.success))
                };

                let branch_str = if wt.is_bare {
                    "MAIN".to_string()
                } else if wt.is_detached {
                    format!(
                        "DETACHED @ {}",
                        &wt.commit[..std::cmp::min(wt.commit.len(), 7)]
                    )
                } else {
                    wt.branch.clone()
                };

                let row_style = if is_selected {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.text)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                let prefix = if is_selected { "  ▶ " } else { "    " };

                let status_cell = if let Some(summary) = &wt.status_summary {
                    let (color, icon) = if summary == "clean" {
                        (theme.success, Icons::CLEAN)
                    } else {
                        (theme.accent, Icons::DIRTY)
                    };
                    Cell::from(format!("{} {}", icon, summary)).style(Style::default().fg(color))
                } else {
                    Cell::from("-")
                };

                Row::new(vec![
                    Cell::from(format!("{}{}", prefix, icon)),
                    Cell::from(branch_str).style(branch_style),
                    status_cell,
                    Cell::from(wt.commit.clone()).style(Style::default().fg(theme.primary)),
                    Cell::from(wt.path.clone()).style(Style::default().fg(theme.subtle)),
                ])
                .style(row_style)
                .height(1)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Percentage(25),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Min(20),
            ],
        )
        .header(
            Row::new(vec![
                "",
                "  BRANCH / INTENT",
                "STATUS",
                "COMMIT",
                "LOCAL PATH",
            ])
            .style(
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    format!(" ACTIVE WORKTREES ({}) ", self.worktrees.len()),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

        StatefulWidget::render(table, area, buf, state);
    }
}
