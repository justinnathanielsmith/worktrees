use crate::ui::theme::CyberTheme;
use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::helpers::centered_rect;

pub fn render_branch_selection(f: &mut Frame, branches: &[String], selected_index: usize) {
    let theme = CyberTheme::default();
    let area = centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .title(Span::styled(
            "  SWITCH BRANCH ",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut items = Vec::new();
    for (i, branch) in branches.iter().enumerate() {
        let is_selected = i == selected_index;
        let style = if is_selected {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { " ▶ " } else { "   " };
        items.push(Line::from(vec![
            Span::styled(prefix, style.fg(theme.primary)),
            Span::styled(branch, style),
        ]));
    }

    f.render_widget(Paragraph::new(items), inner_area);
}
