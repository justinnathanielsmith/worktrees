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
            Span::styled(
                " [j/k/g/G] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("NAV", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [ENT] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("OPEN", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [V] ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("STATUS", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [L] ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("LOG", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [B] ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("BRANCH", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [F] ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("FETCH", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [S] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("SYNC", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [P] ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("PUSH", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [Shift+P] ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("PULL", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [A] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("ADD", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [D/X] ",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("DEL", Style::default().fg(theme.text)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled(
                " [Q] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("EXIT", Style::default().fg(theme.text)),
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
