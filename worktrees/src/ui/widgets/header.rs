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

impl<'a> Widget for HeaderWidget<'a> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " WORKTREE_MANAGER_OS ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);

        let inner_area = block.inner(area);
        block.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner_area);

        // -- Left Side: Project Info --
        let project_info = Line::from(vec![
            Span::styled(" PROJECT: ", Style::default().fg(theme.subtle)),
            Span::styled(
                format!("{} ", self.project_name.to_uppercase()),
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        Paragraph::new(project_info)
            .alignment(Alignment::Left)
            .render(layout[0], buf);

        // -- Right Side: Status & Context --
        let mode_str = match self.state {
            AppState::ListingWorktrees { .. } => "LISTING",
            AppState::ViewingStatus { .. } => "STATUS",
            AppState::ViewingHistory { .. } => "HISTORY",
            AppState::SwitchingBranch { .. } => "SWITCHING",
            AppState::Committing { .. } => "COMMITTING",
            AppState::Prompting { .. } => "INPUT",
            AppState::Syncing { .. } => "SYNCING",
            AppState::Fetching { .. } => "FETCHING",
            AppState::Pushing { .. } => "PUSHING",
            AppState::Error(..) => "ERROR",
            _ => "IDLE",
        };

        let context_str = match self.context {
            ProjectContext::Standard => "STANDARD",
            ProjectContext::KmpAndroid => "KMP/ANDROID",
        };

        let status_info = Line::from(vec![
            Span::styled("MODE: ", Style::default().fg(theme.subtle)),
            Span::styled(
                mode_str,
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled("CONTEXT: ", Style::default().fg(theme.subtle)),
            Span::styled(context_str, Style::default().fg(theme.text)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled("● SYSTEM_READY", Style::default().fg(theme.success)),
        ]);

        Paragraph::new(status_info)
            .alignment(Alignment::Right)
            .render(layout[1], buf);
    }
}
