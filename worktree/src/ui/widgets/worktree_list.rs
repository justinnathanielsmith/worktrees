use crate::domain::repository::Worktree;
use crate::ui::theme::{CyberTheme, Icons};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};
use std::borrow::Cow;

pub struct WorktreeListWidget<'a> {
    worktrees: &'a [Worktree],
    is_dimmed: bool,
    spinner_tick: usize,
    filter_query: Option<&'a str>,
}

impl<'a> WorktreeListWidget<'a> {
    pub const fn new(worktrees: &'a [Worktree]) -> Self {
        Self {
            worktrees,
            is_dimmed: false,
            spinner_tick: 0,
            filter_query: None,
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

    pub const fn with_filter(mut self, query: Option<&'a str>) -> Self {
        self.filter_query = query;
        self
    }
}

impl StatefulWidget for WorktreeListWidget<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer, state: &mut Self::State) {
        let theme = CyberTheme::default();

        let border_style = if self.is_dimmed {
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(theme.border)
        };

        let title_style = if self.is_dimmed {
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::DIM)
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

        let inner_area = block.inner(area);

        if self.worktrees.is_empty() {
            block.render(area, buf);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(2), // Height for 2 lines of text
                    Constraint::Min(0),
                ])
                .split(inner_area);

            let text = if let Some(query) = self.filter_query
                && !query.is_empty()
            {
                vec![
                    Line::from(vec![
                        Span::styled("No worktrees match '", Style::default().fg(theme.subtle)),
                        Span::styled(
                            query,
                            Style::default()
                                .fg(theme.warning)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("'.", Style::default().fg(theme.subtle)),
                    ]),
                    Line::from(Span::styled(
                        "Press [Esc] to clear filter.",
                        Style::default().fg(theme.subtle),
                    )),
                ]
            } else {
                vec![
                    Line::from(Span::styled(
                        "No worktrees found.",
                        Style::default().fg(theme.subtle),
                    )),
                    Line::from(Span::styled(
                        "Press [A] to add a worktree.",
                        Style::default().fg(theme.secondary),
                    )),
                ]
            };

            Paragraph::new(text)
                .alignment(Alignment::Center)
                .render(chunks[1], buf);
            return;
        }

        let rows = self.worktrees.iter().enumerate().map(|(i, wt)| {
            let is_selected = Some(i) == state.selected();

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
                Cow::Borrowed("MAIN")
            } else if let Some(meta) = &wt.metadata
                && let Some(purpose) = &meta.purpose
            {
                Cow::Borrowed(purpose.as_str())
            } else {
                std::path::Path::new(&wt.path).file_name().map_or_else(
                    || Cow::Borrowed(wt.branch.as_str()),
                    |n| n.to_string_lossy(),
                )
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
            }

            // Cyber-style cursor animation
            // ▊, ▋, ▌, ▍, ▎, ▏
            let spinner_prefixes = [" ▊ ", " ▋ ", " ▌ ", " ▍ ", " ▌ ", " ▋ "];
            let spinner_idx = (self.spinner_tick / 2) % spinner_prefixes.len();

            let prefix = if is_selected {
                spinner_prefixes[spinner_idx]
            } else {
                "   "
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

                    let summary_text = if summary == "clean" {
                        "CLEAN"
                    } else {
                        summary.as_str()
                    };
                    Cell::from(Line::from(vec![
                        Span::raw(icon),
                        Span::raw(" "),
                        Span::raw(summary_text),
                    ]))
                    .style(style)
                },
            );

            let mut cell_style = Style::default();
            if self.is_dimmed && !is_selected {
                cell_style = cell_style.fg(theme.subtle);
            }

            Row::new(vec![
                Cell::from(Line::from(vec![Span::raw(prefix), Span::raw(icon)])),
                Cell::from(intent_str).style(if is_selected {
                    branch_style
                } else {
                    cell_style
                }),
                Cell::from(wt.branch.as_str()).style(if is_selected {
                    Style::default().fg(theme.primary)
                } else {
                    cell_style
                }),
                status_cell,
                Cell::from(format_size(wt.size_bytes)).style(Style::default().fg(theme.subtle)),
                Cell::from(&wt.commit[..wt.commit.len().min(7)])
                    .style(Style::default().fg(theme.subtle)),
            ])
            .style(row_style)
            .height(1)
        });

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
                .style(if self.is_dimmed {
                    Style::default()
                        .fg(theme.subtle)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD)
                })
                .bottom_margin(1),
        )
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD)); // Handled manually in row mapping, but keeping basic highlight

        StatefulWidget::render(table, area, buf, state);

        if self.worktrees.len() > inner_area.height as usize {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(None)
                .thumb_symbol("▐")
                .thumb_style(if self.is_dimmed {
                    Style::default()
                        .fg(theme.subtle)
                        .add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(theme.secondary)
                });

            let mut scrollbar_state = ScrollbarState::new(self.worktrees.len())
                .position(state.offset())
                .viewport_content_length(inner_area.height as usize);

            StatefulWidget::render(scrollbar, inner_area, buf, &mut scrollbar_state);
        }
    }
}

fn format_size(bytes: u64) -> Cow<'static, str> {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes == 0 {
        return Cow::Borrowed("0 B");
    }

    let s = if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    };
    Cow::Owned(s)
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

    #[test]
    fn test_render_empty_worktree_list_with_filter() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = TableState::default();
        let worktrees = vec![];
        let widget = WorktreeListWidget::new(&worktrees).with_filter(Some("foobar"));

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

        assert!(content.contains("No worktrees match 'foobar'."));
        assert!(content.contains("Press [Esc] to clear filter."));
        assert!(content.contains("ACTIVE WORKTREES (0)"));
    }
}
