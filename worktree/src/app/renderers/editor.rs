use crate::app::model::EditorConfig;
use crate::ui::theme::CyberTheme;
use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::helpers::centered_rect;

pub fn render_editor_selection(
    f: &mut Frame,
    branch: &str,
    options: &[EditorConfig],
    selected: usize,
) {
    let theme = CyberTheme::default();
    let area = centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            format!(" 📝 SELECT EDITOR: {branch} "),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut items = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let is_selected = i == selected;
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
            Span::styled(&option.name, style),
            Span::raw(" "),
            Span::styled(
                format!("({})", option.command),
                if is_selected {
                    style.fg(theme.subtle)
                } else {
                    Style::default().fg(theme.subtle)
                },
            ),
        ]));
    }

    f.render_widget(Paragraph::new(items), inner_area);
}
