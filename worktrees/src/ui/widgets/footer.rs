use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct FooterWidget;

impl Widget for FooterWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let footer_text = vec![Line::from(vec![
            // NAVIGATION GROUP
            Span::styled(
                " [j/k] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("NAV", Style::default().fg(theme.subtle)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(
                " [ENT] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("OPEN", Style::default().fg(theme.subtle)),
            Span::styled(" ║ ", Style::default().fg(theme.secondary)), // Separator
            // VIEW GROUP
            Span::styled(
                " [V] ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("STATUS", Style::default().fg(theme.subtle)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(
                " [L] ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("LOG", Style::default().fg(theme.subtle)),
            Span::styled(" ║ ", Style::default().fg(theme.secondary)), // Separator
            // GIT ACTIONS GROUP
            Span::styled(
                " [S] ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("SYNC", Style::default().fg(theme.subtle)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(
                " [P] ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("PUSH", Style::default().fg(theme.subtle)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(
                " [B] ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("BRANCH", Style::default().fg(theme.subtle)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(
                " [A] ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("ADD", Style::default().fg(theme.subtle)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(
                " [C] ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("PRUNE", Style::default().fg(theme.subtle)),
            Span::styled(" ║ ", Style::default().fg(theme.secondary)), // Separator
            // DESTRUCTIVE GROUP
            Span::styled(
                " [D/X] ",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("DEL", Style::default().fg(theme.subtle)),
            Span::styled(" ║ ", Style::default().fg(theme.secondary)), // Separator
            // SYSTEM GROUP
            Span::styled(
                " [Q] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("EXIT", Style::default().fg(theme.subtle)),
        ])];

        Paragraph::new(footer_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border)),
            )
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}
