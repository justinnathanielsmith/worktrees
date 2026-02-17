use crate::app::model::AppState;
use crate::domain::repository::ProjectContext;
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct HeaderWidget<'a> {
    pub context: ProjectContext,
    pub project_name: String,
    pub state: &'a AppState,
}

impl Widget for HeaderWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick) // Thicker border for header
            .border_style(Style::default().fg(theme.primary))
            .style(Style::default().bg(theme.header_bg));

        let inner_area = block.inner(area);
        block.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner_area);

        // -- Left Side: Branding & Project --
        let title = Line::from(vec![
            Span::styled(
                " WORKTREE",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" // ", Style::default().fg(theme.subtle)),
            Span::styled(
                "HUB",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" // ", Style::default().fg(theme.subtle)),
            Span::styled(
                "OS ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(":: ", Style::default().fg(theme.subtle)),
            Span::styled(
                self.project_name.to_uppercase(),
                Style::default()
                    .fg(theme.text)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ),
        ]);

        Paragraph::new(title)
            .alignment(Alignment::Left)
            .render(layout[0], buf);

        // -- Right Side: Status Matrix --
        let mode_str = match self.state {
            AppState::ListingWorktrees { mode, .. } => match mode {
                crate::app::model::AppMode::Normal => "READY",
                crate::app::model::AppMode::Manage => "MANAGE",
                crate::app::model::AppMode::Git => "GIT_OPS",
                crate::app::model::AppMode::Filter => "FILTERING",
            },
            AppState::ViewingStatus { .. } => "INSPECT",
            AppState::ViewingHistory { .. } => "LOG_VIEW",
            AppState::SwitchingBranch { .. } => "SWITCH",
            AppState::Committing { .. } => "COMMIT",
            AppState::Prompting { .. } => "INPUT",
            AppState::Syncing { .. } => "NET_SYNC",
            AppState::Fetching { .. } => "NET_FETCH",
            AppState::Pushing { .. } => "NET_PUSH",
            AppState::Error(..) => "SYS_ERR",
            _ => "UNKNOWN",
        };

        let context_str = match self.context {
            ProjectContext::Standard => "STD_GIT",
            ProjectContext::KmpAndroid => "KMP_DROID",
        };

        let status_info = Line::from(vec![
            Span::styled("CTX: ", Style::default().fg(theme.subtle)),
            Span::styled(context_str, Style::default().fg(theme.secondary)),
            Span::styled(" | ", Style::default().fg(theme.subtle)),
            Span::styled("MODE: ", Style::default().fg(theme.subtle)),
            Span::styled(
                mode_str,
                Style::default()
                    .fg(if let AppState::Error(..) = self.state {
                        theme.error
                    } else {
                        theme.accent
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        Paragraph::new(status_info)
            .alignment(Alignment::Right)
            .render(layout[1], buf);
    }
}
