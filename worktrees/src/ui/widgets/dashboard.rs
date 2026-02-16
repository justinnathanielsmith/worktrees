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
    pub status: &'a Option<GitStatus>,
}

impl<'a> Widget for StatusTabWidget<'a> {
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
                if !status.staged.is_empty() {
                    lines.push(Line::from(Span::styled(
                        " 󰄬 STAGED:",
                        Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for file in &status.staged {
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(file, Style::default().fg(theme.success)),
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
                    for file in &status.unstaged {
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(file, Style::default().fg(theme.warning)),
                        ]));
                    }
                    for file in &status.untracked {
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(file, Style::default().fg(theme.error)),
                        ]));
                    }
                }
            }
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
    pub history: &'a Option<Vec<GitCommit>>,
}

impl<'a> Widget for LogTabWidget<'a> {
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
    context: ProjectContext,
    active_tab: DashboardTab,
    status: &'a Option<GitStatus>,
    history: &'a Option<Vec<GitCommit>>,
}

impl<'a> DashboardWidget<'a> {
    pub fn new(
        worktree: Option<&'a Worktree>,
        context: ProjectContext,
        active_tab: DashboardTab,
        status: &'a Option<GitStatus>,
        history: &'a Option<Vec<GitCommit>>,
    ) -> Self {
        Self {
            worktree,
            context,
            active_tab,
            status,
            history,
        }
    }
}

impl<'a> Widget for DashboardWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Tab Bar
        let tabs = vec![
            (DashboardTab::Info, " 󰋼 INFO [1] ", theme.primary),
            (DashboardTab::Status, "  STATUS [2] ", theme.success),
            (DashboardTab::Log, " 󰜘 LOG [3] ", theme.secondary),
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
            tab_spans.push(Span::styled(label, style));
            tab_spans.push(Span::styled("  ", Style::default()));
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
                DetailsWidget::new(self.worktree, self.context).render(chunks[1], buf)
            }
            DashboardTab::Status => StatusTabWidget {
                status: self.status,
            }
            .render(chunks[1], buf),
            DashboardTab::Log => LogTabWidget {
                history: self.history,
            }
            .render(chunks[1], buf),
        }
    }
}
