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
                    vec![
                        ("[j/k]", "NAV", theme.primary),
                        ("[ENT]", "OPEN", theme.primary),
                    ],
                    vec![
                        ("[m]", "MANAGE", theme.secondary),
                        ("[g]", "GIT", theme.secondary),
                        ("[/]", "FILTER", theme.warning),
                    ],
                    vec![
                        ("[v]", "STATUS", theme.text),
                        ("[l]", "LOG", theme.text),
                        ("[q]", "EXIT", theme.error),
                    ],
                ],
                crate::app::model::AppMode::Manage => vec![
                    vec![
                        ("[a]", "ADD", theme.success),
                        ("[ESC]", "BACK", theme.accent),
                    ],
                    vec![
                        ("[r]", "REMOVE", theme.error),
                        ("[c]", "CLEAN", theme.error), // Changed to error for safety
                    ],
                ],
                crate::app::model::AppMode::Git => vec![
                    vec![
                        ("[p]", "PULL", theme.success),
                        ("[P]", "PUSH", theme.success),
                        ("[s]", "SYNC", theme.success),
                        ("[R]", "REBASE", theme.success),
                    ],
                    vec![("[ESC]", "BACK", theme.accent)],
                ],
                crate::app::model::AppMode::Filter => vec![
                    vec![
                        ("[Typing...]", "SEARCH", theme.primary),
                        ("[ENT]", "DONE", theme.success),
                    ],
                    vec![("[ESC]", "CLEAR", theme.error)],
                ],
            },
            AppState::ViewingStatus { .. } => vec![
                vec![
                    ("[j/k]", "NAV", theme.primary),
                    ("[TAB]", "STAGE", theme.success),
                    ("[C]", "COMMIT", theme.success),
                ],
                vec![("[ESC]", "BACK", theme.accent)],
            ],
            AppState::ViewingHistory { .. } => vec![vec![
                ("[j/k]", "NAV", theme.primary),
                ("[ESC]", "BACK", theme.accent),
            ]],
            AppState::SwitchingBranch { .. }
            | AppState::PickingBaseRef { .. }
            | AppState::Committing { .. } => vec![
                vec![
                    ("[j/k]", "NAV", theme.primary),
                    ("[ENT]", "SELECT", theme.success),
                ],
                vec![("[ESC]", "BACK", theme.accent)],
            ],
            AppState::SelectingEditor { .. } => vec![
                vec![
                    ("[j/k]", "NAV", theme.primary),
                    ("[ENT]", "OPEN", theme.success),
                ],
                vec![("[ESC]", "BACK", theme.accent)],
            ],
            AppState::Prompting { .. } => vec![vec![
                ("[ENT]", "SUBMIT", theme.success),
                ("[ESC]", "CANCEL", theme.error),
            ]],
            AppState::Confirming { .. } => vec![
                vec![("[y]", "YES", theme.success), ("[n]", "NO", theme.error)],
                vec![("[ESC]", "CANCEL", theme.subtle)],
            ],
            AppState::Help { .. } => vec![vec![("[ESC]", "BACK", theme.accent)]],
            AppState::Welcome => vec![vec![
                ("[I]", "INIT", theme.primary),
                ("[C]", "CONVERT", theme.secondary),
                ("[Q]", "EXIT", theme.error),
            ]],
            _ => vec![vec![("[Q]", "EXIT", theme.error)]],
        };

        let mut spans = Vec::new();
        for (group_idx, group) in shortcuts.into_iter().enumerate() {
            if group_idx > 0 {
                spans.push(Span::styled(
                    "   │   ",
                    Style::default()
                        .fg(theme.subtle)
                        .add_modifier(Modifier::DIM),
                ));
            }
            for (i, (key, label, color)) in group.into_iter().enumerate() {
                if i > 0 {
                    spans.push(Span::styled(" ", Style::default()));
                }
                spans.push(Span::styled(
                    key.to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!(" {label}"),
                    Style::default().fg(theme.subtle),
                ));
            }
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
