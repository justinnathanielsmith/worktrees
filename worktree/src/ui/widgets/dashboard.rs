use crate::app::model::DashboardTab;
use crate::domain::repository::{GitCommit, GitStatus, ProjectContext, Worktree};
use crate::ui::theme::CyberTheme;
use crate::ui::widgets::details::DetailsWidget;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct StatusTabWidget<'a> {
    pub status: Option<&'a GitStatus>,
    pub is_bare: bool,
}

impl Widget for StatusTabWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();
        let mut lines = Vec::new();

        if let Some(status) = self.status {
            if status.staged.is_empty() && status.unstaged.is_empty() && status.untracked.is_empty()
            {
                lines.push(Line::from(vec![Span::styled(
                    " ✨ Working directory clean.",
                    Style::default().fg(theme.success),
                )]));
            } else {
                let max_items = 15;
                
                if !status.staged.is_empty() {
                    lines.push(Line::from(Span::styled(
                        " 󰄬 STAGED:",
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    )));
                    
                    for (i, (file, code)) in status.staged.iter().enumerate() {
                        if i >= max_items {
                            lines.push(Line::from(vec![
                                Span::raw("   "),
                                Span::styled(
                                    format!("... and {} more", status.staged.len() - max_items),
                                    Style::default().fg(theme.subtle).add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                            break;
                        }

                        let is_deleted = code.contains('D');
                        let style = if is_deleted {
                            Style::default().fg(theme.error)
                        } else {
                            Style::default()
                                .fg(theme.success)
                                .add_modifier(Modifier::BOLD)
                        };
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(file, style),
                        ]));
                    }
                    lines.push(Line::from(""));
                }

                if !status.unstaged.is_empty() || !status.untracked.is_empty() {
                    lines.push(Line::from(Span::styled(
                        " 󱇨 UNSTAGED:",
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    )));
                    
                    let mut count = 0;
                    for (file, code) in &status.unstaged {
                        if count >= max_items {
                            let remaining = (status.unstaged.len() + status.untracked.len()) - count;
                             lines.push(Line::from(vec![
                                Span::raw("   "),
                                Span::styled(
                                    format!("... and {} more", remaining),
                                    Style::default().fg(theme.subtle).add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                            count += 1000; // Force break outer or just break here?
                            break;
                        }
                        
                        let is_deleted = code.contains('D');
                        let style = if is_deleted {
                            Style::default().fg(theme.error)
                        } else {
                            Style::default()
                                .fg(theme.warning)
                                .add_modifier(Modifier::DIM)
                        };
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(file, style),
                        ]));
                        count += 1;
                    }

                    if count < max_items {
                         for file in &status.untracked {
                            if count >= max_items {
                                // check render count vs total
                                lines.push(Line::from(vec![
                                    Span::raw("   "),
                                    Span::styled(
                                        format!("... and {} more", status.untracked.len() - (count - status.unstaged.len())), 
                                        Style::default().fg(theme.subtle).add_modifier(Modifier::ITALIC),
                                    ),
                                ]));
                                break;
                            }
                            lines.push(Line::from(vec![
                                Span::raw("   "),
                                Span::styled(file, Style::default().fg(theme.error)),
                            ]));
                            count += 1;
                        }
                    } else if count >= 1000 {
                        // Already handled overflow
                    }
                }
            }
        } else if self.is_bare {
            lines.push(Line::from(vec![Span::styled(
                " 󱗗 Not available for bare repository.",
                Style::default().fg(theme.subtle),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                " 󰚰 Loading status...",
                Style::default().fg(theme.subtle),
            )]));
        }

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
                    .title(Span::styled(
                        " STATUS ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .render(area, buf);
    }
}

pub struct LogTabWidget<'a> {
    pub history: Option<&'a [GitCommit]>,
    pub is_bare: bool,
}

impl Widget for LogTabWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();
        let mut lines = Vec::new();

        if let Some(history) = self.history {
            if history.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    " 󰜘 No commit history found.",
                    Style::default().fg(theme.subtle),
                )]));
            } else {
                for commit in history {
                    lines.push(Line::from(vec![
                        Span::styled(&commit.hash, Style::default().fg(theme.secondary)),
                        Span::raw(" "),
                        Span::styled(&commit.message, Style::default().fg(theme.text)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::raw("   "),
                        Span::styled(&commit.author, Style::default().fg(theme.subtle)),
                        Span::raw(" • "),
                        Span::styled(&commit.date, Style::default().fg(theme.subtle)),
                    ]));
                }
            }
        } else if self.is_bare {
            lines.push(Line::from(vec![Span::styled(
                " 󱗗 Not available for bare repository.",
                Style::default().fg(theme.subtle),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                " 󰚰 Loading history...",
                Style::default().fg(theme.subtle),
            )]));
        }

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
                    .title(Span::styled(
                        " HISTORY ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .render(area, buf);
    }
}

pub struct DashboardWidget<'a> {
    worktree: Option<&'a Worktree>,
    all_worktrees: &'a [Worktree],
    context: ProjectContext,
    active_tab: DashboardTab,
    status: Option<&'a GitStatus>,
    history: Option<&'a [GitCommit]>,
}

impl<'a> DashboardWidget<'a> {
    pub const fn new(
        worktree: Option<&'a Worktree>,
        all_worktrees: &'a [Worktree],
        context: ProjectContext,
        active_tab: DashboardTab,
        status: Option<&'a GitStatus>,
        history: Option<&'a [GitCommit]>,
    ) -> Self {
        Self {
            worktree,
            all_worktrees,
            context,
            active_tab,
            status,
            history,
        }
    }
}

impl Widget for DashboardWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Tab Bar
        let tabs = vec![
            (DashboardTab::Info, " INFO [1] ", theme.primary),
            (DashboardTab::Status, " STATUS [2] ", theme.success),
            (DashboardTab::Log, " LOG [3] ", theme.secondary),
        ];

        let mut tab_spans = Vec::new();
        for (tab, label, color) in tabs {
            let is_active = tab == self.active_tab;
            let style = if is_active {
                Style::default()
                    .fg(color)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().fg(theme.subtle)
            };

            // Add icon based on tab
            let icon = match tab {
                DashboardTab::Info => "󰋼",
                DashboardTab::Status => "",
                DashboardTab::Log => "󰜘",
            };

            tab_spans.push(Span::styled(
                format!(" {icon} "),
                if is_active {
                    Style::default().fg(color).bg(theme.selection_bg)
                } else {
                    Style::default().fg(theme.subtle)
                },
            ));
            tab_spans.push(Span::styled(label, style));
            tab_spans.push(Span::styled(" ", Style::default())); // Spacer
        }

        Paragraph::new(Line::from(tab_spans))
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.border)),
            )
            .render(chunks[0], buf);

        // Content
        match self.active_tab {
            DashboardTab::Info => {
                DetailsWidget::new(self.worktree, self.all_worktrees, self.context)
                    .render(chunks[1], buf);
            }
            DashboardTab::Status => StatusTabWidget {
                status: self.status,
                is_bare: self.worktree.is_some_and(|wt| wt.is_bare),
            }
            .render(chunks[1], buf),
            DashboardTab::Log => LogTabWidget {
                history: self.history,
                is_bare: self.worktree.is_some_and(|wt| wt.is_bare),
            }
            .render(chunks[1], buf),
        }
    }
}
