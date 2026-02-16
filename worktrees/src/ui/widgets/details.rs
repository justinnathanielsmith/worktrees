use crate::domain::repository::{ProjectContext, Worktree};
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct DetailsWidget<'a> {
    worktree: Option<&'a Worktree>,
    context: ProjectContext,
}

impl<'a> DetailsWidget<'a> {
    pub fn new(worktree: Option<&'a Worktree>, context: ProjectContext) -> Self {
        Self { worktree, context }
    }
}

impl<'a> Widget for DetailsWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();
        
        let details_text = if let Some(wt) = self.worktree {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(" 🔹 NAME:   ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(&wt.branch, Style::default().fg(theme.text)),
                    Span::styled(" [", Style::default().fg(theme.subtle)),
                    Span::styled(
                        if wt.is_bare { "BASE" } else if wt.is_detached { "DETACHED" } else { "ACTIVE" },
                        Style::default().fg(if wt.is_bare { theme.secondary } else if wt.is_detached { theme.error } else { theme.success })
                    ),
                    Span::styled("]", Style::default().fg(theme.subtle)),
                ]),
                Line::from(vec![
                    Span::styled(" 📍 COMMIT: ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(&wt.commit, Style::default().fg(theme.secondary)),
                ]),
                Line::from(vec![
                    Span::styled(" 📁 PATH:   ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(&wt.path, Style::default().fg(theme.text).add_modifier(Modifier::ITALIC)),
                ]),
            ];

            if self.context == ProjectContext::KmpAndroid {
                lines.push(Line::from(vec![
                    Span::styled(" 🚀 OPTIM:   ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                    Span::styled("GRADLE_CACHE_ACTIVE", Style::default().fg(theme.text)),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled(" ✨ STATUS: ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if wt.is_bare { "Main Repository" } else if wt.is_detached { "Detached State" } else { "Active Worktree" },
                    Style::default().fg(if wt.is_bare { theme.subtle } else if wt.is_detached { theme.error } else { theme.success })
                ),
            ]));

            lines
        } else {
            vec![Line::from(vec![
                Span::styled(" ℹ️ ", Style::default().fg(theme.warning)),
                Span::styled("Select a worktree to view details", Style::default().fg(theme.subtle).add_modifier(Modifier::ITALIC)),
            ])]
        };

        Paragraph::new(details_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
                    .title(Span::styled(" DETAILS ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)))
            )
            .render(area, buf);
    }
}
