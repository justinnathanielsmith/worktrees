use crate::ui::theme::CyberTheme;
use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::helpers::centered_rect;

pub fn render_commit_menu(f: &mut Frame, branch: &str, selected_index: usize) {
    let theme = CyberTheme::default();
    let area = centered_rect(50, 40, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            format!(" 󰊚 COMMIT: {branch} "),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let options = ["Manual Commit", "AI Commit", "Set API Key"];
    let mut items = Vec::new();

    items.push(Line::from(""));

    for (i, option) in options.iter().enumerate() {
        let is_selected = i == selected_index;
        let style = if is_selected {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { " ▶ " } else { "   " };
        items.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(*option, style),
        ]));
        items.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(items), inner_area);
}
