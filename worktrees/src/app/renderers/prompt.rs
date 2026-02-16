use crate::app::model::PromptType;
use crate::ui::theme::CyberTheme;
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use super::helpers::centered_rect;

pub fn render_prompt(
    f: &mut Frame,
    prompt_type: &PromptType,
    input: &str,
) {
    let theme = CyberTheme::default();
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let title = match prompt_type {
        PromptType::AddIntent => " ADD WORKTREE INTENT ",
        PromptType::InitUrl => " REPOSITORY URL ",
        PromptType::CommitMessage => " COMMIT MESSAGE ",
        PromptType::ApiKey => " GEMINI API KEY ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            title,
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::raw(input),
            Span::styled("█", Style::default().fg(theme.primary).add_modifier(Modifier::SLOW_BLINK)),
        ]),
    ])
    .alignment(Alignment::Left);

    f.render_widget(p, inner_area);
}
