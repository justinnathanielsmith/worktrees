use crate::domain::repository::ProjectContext;
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct HeaderWidget {
    pub context: ProjectContext,
}

impl Widget for HeaderWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let theme = CyberTheme::default();
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " WORKTREE_MANAGER_OS ",
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Left);
        
        let context_str = match self.context {
            ProjectContext::Standard => "STANDARD",
            ProjectContext::KmpAndroid => "KMP/ANDROID",
        };

        let header_text = vec![Line::from(vec![
            Span::styled(" ● ", Style::default().fg(theme.success)),
            Span::styled("SYSTEM_READY", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled("CONTEXT: ", Style::default().fg(theme.subtle)),
            Span::styled(context_str, Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled("TYPE: ", Style::default().fg(theme.subtle)),
            Span::styled("BARE_REPO", Style::default().fg(theme.secondary)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled("NETWORK: ", Style::default().fg(theme.subtle)),
            Span::styled("CONNECTED", Style::default().fg(theme.accent)),
        ])];
        
        Paragraph::new(header_text)
            .block(block)
            .alignment(Alignment::Right)
            .render(area, buf);
    }
}
