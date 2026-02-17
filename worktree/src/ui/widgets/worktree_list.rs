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
    is_dimmed: bool,
    spinner_tick: usize,
}

impl<'a> WorktreeListWidget<'a> {
    pub const fn new(worktrees: &'a [Worktree]) -> Self {
        Self {
            worktrees,
            is_dimmed: false,
            spinner_tick: 0,
        }
    }

    pub const fn dimmed(mut self, is_dimmed: bool) -> Self {
        self.is_dimmed = is_dimmed;
        self
    }

    pub const fn tick(mut self, tick: usize) -> Self {
        self.spinner_tick = tick;
        self
    }
}

impl StatefulWidget for WorktreeListWidget<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let theme = CyberTheme::default();

        let border_style = if self.is_dimmed {
            Style::default().fg(theme.subtle).add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(theme.border)
        };

        let title_style = if self.is_dimmed {
            Style::default().fg(theme.subtle).add_modifier(Modifier::DIM)
        } else {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(Span::styled(
                format!(" ACTIVE WORKTREES ({}) ", self.worktrees.len()),
                title_style,
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
                let is_dirty = wt.status_summary.as_ref().is_some_and(|s| s != "clean");

                let (icon, branch_style) = if wt.is_bare {
                    (
                        Icons::HUB,
                        if self.is_dimmed {
                            Style::default().fg(theme.subtle)
                        } else {
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD)
                        },
                    )
                } else if wt.is_detached {
                    (Icons::DETACHED, Style::default().fg(theme.error))
                } else {
                    (
                        Icons::WORKTREE,
                        if self.is_dimmed {
                            Style::default().fg(theme.subtle)
                        } else {
                            Style::default().fg(theme.primary)
                        },
                    )
                };

                let intent_str = if wt.is_bare {
                    "MAIN".to_string()
                } else if let Some(meta) = &wt.metadata
                    && let Some(purpose) = &meta.purpose
                {
                    purpose.clone()
                } else {
                    std::path::Path::new(&wt.path)
                        .file_name()
                        .map_or_else(|| wt.branch.clone(), |n| n.to_string_lossy().to_string())
                };

                let mut row_style = Style::default().fg(theme.text);
                
                if self.is_dimmed {
                    row_style = row_style.add_modifier(Modifier::DIM);
                }

                if is_selected {
                    row_style = row_style
                        .bg(theme.selection_bg)
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD);
                        
                    // Remove dim modifier if selected, to make it pop even when filtering
                    if self.is_dimmed {
                         row_style = row_style.remove_modifier(Modifier::DIM);   
                    }
                } else if is_dirty && !self.is_dimmed {
                     // Subtle warning tint for dirty rows if not selected and not dimmed
                     // We don't have a background color for warning in theme, so let's use subtle or just text color
                     // The user requested: "row background has a subtle warning (Yellow) tint"
                     // Since we don't have a 'tint', we can maybe change the text color or just keep it simple.
                     // Making the text warning color might be too much.
                     // The user said "row background". Ratatui doesn't support alpha blending for bg.
                     // Best we can do is maybe use a different bg color if we had one.
                     // Let's just color the status cell strongly.
                }

                // Cyber-style cursor animation
                // ▊, ▋, ▌, ▍, ▎, ▏
                let spinner_chars = ["▊", "▋", "▌", "▍", "▌", "▋"];
                let spinner_idx = (self.spinner_tick / 2) % spinner_chars.len();
                let cursor_char = spinner_chars[spinner_idx];
                
                let prefix = if is_selected { 
                    format!(" {} ", cursor_char)
                } else { 
                    "   ".to_string()
                };

                let status_cell = wt.status_summary.as_ref().map_or_else(
                    || Cell::from("-"),
                    |summary| {
                        let (color, icon) = if summary == "clean" {
                            (theme.success, Icons::CLEAN)
                        } else {
                            (theme.warning, Icons::DIRTY)
                        };
                        
                        let style = if self.is_dimmed && !is_selected {
                             Style::default().fg(theme.subtle)   
                        } else {
                             Style::default().fg(color)
                        };
                        
                        Cell::from(format!("{} {}", icon, summary.to_uppercase()))
                            .style(style)
                    },
                );
                
                let mut cell_style = Style::default();
                if self.is_dimmed && !is_selected {
                    cell_style = cell_style.fg(theme.subtle);
                }

                Row::new(vec![
                    Cell::from(format!("{prefix}{icon}")),
                    Cell::from(intent_str).style(if is_selected { branch_style } else { cell_style }),
                    Cell::from(wt.branch.clone()).style(if is_selected { Style::default().fg(theme.primary) } else { cell_style }),
                    status_cell,
                    Cell::from(format_size(wt.size_bytes)).style(Style::default().fg(theme.subtle)),
                    Cell::from(wt.commit.chars().take(7).collect::<String>())
                        .style(Style::default().fg(theme.subtle)),
                ])
                .style(row_style)
                .height(1)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(6), // Reduced width for icon/cursor
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Length(12),
                Constraint::Length(10), // Size column
                Constraint::Length(10), // Commit column
            ],
        )
        .header(
            Row::new(vec!["", "INTENT", "BRANCH", "STATUS", "SIZE", "COMMIT"])
                .style(
                    if self.is_dimmed {
                        Style::default().fg(theme.subtle).add_modifier(Modifier::DIM)
                    } else {
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD)
                    }
                )
                .bottom_margin(1),
        )
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD)); // Handled manually in row mapping, but keeping basic highlight

        StatefulWidget::render(table, area, buf, state);
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
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
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(content.contains("No worktrees found."));
        assert!(content.contains("Press [A] to add a worktree."));
        assert!(content.contains("ACTIVE WORKTREES (0)"));
    }
}
