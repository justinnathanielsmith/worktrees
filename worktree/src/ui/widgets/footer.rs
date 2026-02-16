use crate::app::model::AppState;
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct FooterWidget<'a> {
    pub state: &'a AppState,
}

impl<'a> Widget for FooterWidget<'a> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let shortcuts = match self.state {
            AppState::ListingWorktrees { .. } => vec![
                ("[j/k]", "NAV", theme.primary),
                ("[ENT]", "OPEN", theme.primary),
                ("[V]", "STATUS", theme.text),
                ("[L]", "LOG", theme.text),
                ("[S]", "SYNC", theme.success),
                ("[P]", "PUSH", theme.success),
                ("[B]", "BRANCH", theme.success),
                ("[A]", "ADD", theme.success),
                ("[C]", "COMMIT", theme.success),
                ("[d/x/D]", "DEL", theme.error),
                ("[Q]", "EXIT", theme.accent),
            ],
            AppState::ViewingStatus { .. } => vec![
                ("[j/k]", "NAV", theme.primary),
                ("[TAB]", "STAGE", theme.success),
                ("[C]", "COMMIT", theme.success),
                ("[ESC]", "BACK", theme.accent),
            ],
            AppState::ViewingHistory { .. } => vec![
                ("[j/k]", "NAV", theme.primary),
                ("[ESC]", "BACK", theme.accent),
            ],
            AppState::SwitchingBranch { .. } | AppState::PickingBaseRef { .. } => vec![
                ("[j/k]", "NAV", theme.primary),
                ("[ENT]", "SELECT", theme.success),
                ("[ESC]", "BACK", theme.accent),
            ],
            AppState::Committing { .. } => vec![
                ("[j/k]", "NAV", theme.primary),
                ("[ENT]", "SELECT", theme.success),
                ("[ESC]", "BACK", theme.accent),
            ],
            AppState::SelectingEditor { .. } => vec![
                ("[j/k]", "NAV", theme.primary),
                ("[ENT]", "OPEN", theme.success),
                ("[ESC]", "BACK", theme.accent),
            ],
            AppState::Prompting { .. } => vec![
                ("[ENT]", "SUBMIT", theme.success),
                ("[ESC]", "CANCEL", theme.error),
            ],
            AppState::Confirming { .. } => vec![
                ("[y]", "YES", theme.success),
                ("[n]", "NO", theme.error),
                ("[ESC]", "CANCEL", theme.subtle),
            ],
            AppState::Help { .. } => vec![("[ESC]", "BACK", theme.accent)],
            AppState::Welcome => vec![
                ("[I]", "INIT", theme.primary),
                ("[C]", "CONVERT", theme.secondary),
                ("[Q]", "EXIT", theme.accent),
            ],
            _ => vec![("[Q]", "EXIT", theme.accent)],
        };

        let mut spans = Vec::new();
        for (i, (key, label, color)) in shortcuts.into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", Style::default().fg(theme.border)));
            }
            spans.push(Span::styled(
                format!(" {} ", key),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(label, Style::default().fg(theme.subtle)));
        }

        let footer_text = vec![Line::from(spans)];

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
