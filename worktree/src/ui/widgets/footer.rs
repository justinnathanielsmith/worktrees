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
                        ("[?]", "HELP", theme.accent),
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
                    vec![
                        ("[^U]", "CLEAR", theme.warning),
                        ("[ESC]", "CANCEL", theme.error),
                    ],
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::{AppMode, AppState, DashboardState, DashboardTab, RefreshType};
    use ratatui::{Terminal, backend::TestBackend, widgets::TableState};

    #[test]
    fn test_footer_render_help_shortcut() {
        let backend = TestBackend::new(100, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let worktrees = vec![];
        let state = AppState::ListingWorktrees {
            worktrees: worktrees.clone(),
            filtered_indices: Vec::new(),
            table_state: TableState::default(),
            refresh_needed: RefreshType::None,
            selection_mode: false,
            dashboard: DashboardState {
                active_tab: DashboardTab::Info,
                cached_status: None,
                cached_history: None,
                loading: false,
            },
            filter_query: String::new(),
            is_filtering: false,
            mode: AppMode::Normal,
            last_selection_change: std::time::Instant::now(),
        };

        terminal
            .draw(|f| {
                let area = f.area();
                let widget = FooterWidget { state: &state };
                f.render_widget(widget, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(content.contains("[?]"));
        assert!(content.contains("HELP"));
    }

    #[test]
    fn test_footer_render_filter_shortcuts() {
        let backend = TestBackend::new(100, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let worktrees = vec![];
        let state = AppState::ListingWorktrees {
            worktrees: worktrees.clone(),
            filtered_indices: Vec::new(),
            table_state: TableState::default(),
            refresh_needed: RefreshType::None,
            selection_mode: false,
            dashboard: DashboardState {
                active_tab: DashboardTab::Info,
                cached_status: None,
                cached_history: None,
                loading: false,
            },
            filter_query: String::new(),
            is_filtering: true,
            mode: AppMode::Filter,
            last_selection_change: std::time::Instant::now(),
        };

        terminal
            .draw(|f| {
                let area = f.area();
                let widget = FooterWidget { state: &state };
                f.render_widget(widget, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();

        assert!(content.contains("[^U]"));
        assert!(content.contains("CLEAR"));
        assert!(content.contains("[ESC]"));
        assert!(content.contains("CANCEL"));
    }
}
