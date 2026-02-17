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

impl Widget for FooterWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let shortcuts = match self.state {
            AppState::ListingWorktrees { mode, .. } => match mode {
                crate::app::model::AppMode::Normal => vec![
                    ("[j/k]", "NAV", theme.primary),
                    ("[ENT]", "OPEN", theme.primary),
                    ("[v]", "STATUS", theme.text),
                    ("[l]", "LOG", theme.text),
                    ("[m]", "MANAGE", theme.success),
                    ("[g]", "GIT", theme.success),
                    ("[/]", "FILTER", theme.success),
                    ("[q]", "EXIT", theme.accent),
                ],
                crate::app::model::AppMode::Manage => vec![
                    ("[a]", "ADD", theme.success),
                    ("[d/x]", "DEL", theme.error),
                    ("[c]", "PRUNE", theme.success),
                    ("[C]", "CLEAN", theme.error),
                    ("[ESC]", "BACK", theme.accent),
                ],
                crate::app::model::AppMode::Git => vec![
                    ("[s]", "SYNC", theme.success),
                    ("[p]", "PUSH", theme.success),
                    ("[P]", "PULL", theme.success),
                    ("[f]", "FETCH", theme.success),
                    ("[ESC]", "BACK", theme.accent),
                ],
                crate::app::model::AppMode::Filter => vec![
                    ("[Typing...]", "SEARCH", theme.primary),
                    ("[ENT]", "DONE", theme.success),
                    ("[ESC]", "CLEAR", theme.error),
                ],
            },
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
            AppState::SwitchingBranch { .. }
            | AppState::PickingBaseRef { .. }
            | AppState::Committing { .. } => vec![
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
                format!(" {key} "),
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
