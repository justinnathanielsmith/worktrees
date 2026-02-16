use crate::domain::repository::Worktree;
use crate::ui::theme::{CyberTheme, Icons};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, StatefulWidget, Table, TableState, Widget,
    },
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

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                format!(" ACTIVE WORKTREES ({}) ", self.worktrees.len()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));

        if self.worktrees.is_empty() {
            let inner_area = block.inner(area);
            block.render(area, buf);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(2), // Height for 2 lines of text
                    Constraint::Min(0),
                ])
                .split(inner_area);

            let text = vec![
                Line::from(Span::styled(
                    "No worktrees found.",
                    Style::default().fg(theme.subtle),
                )),
                Line::from(Span::styled(
                    "Press [A] to add a worktree.",
                    Style::default().fg(theme.secondary),
                )),
            ];

            Paragraph::new(text)
                .alignment(Alignment::Center)
                .render(chunks[1], buf);
            return;
        }

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
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

        StatefulWidget::render(table, area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_render_empty_worktree_list() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        let worktrees = vec![];
        let widget = WorktreeListWidget::new(&worktrees);

        terminal
            .draw(|f| {
                let area = f.area();
                f.render_stateful_widget(widget, area, &mut state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(content.contains("No worktrees found."));
        assert!(content.contains("Press [A] to add a worktree."));
        assert!(content.contains("ACTIVE WORKTREES (0)"));
    }
}
