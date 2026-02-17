use crate::domain::repository::GitCommit;
use crate::ui::theme::CyberTheme;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::helpers::centered_rect;

pub fn render_history(f: &mut Frame, branch: &str, commits: &[GitCommit], selected_index: usize) {
    let theme = CyberTheme::default();
    let area = centered_rect(85, 80, f.area());
    f.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.secondary))
        .title(Span::styled(
            format!(" 󰊚 COMMIT LOG: {branch} "),
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let mut items = Vec::new();
    for (i, commit) in commits.iter().enumerate() {
        let is_selected = i == selected_index;
        let row_style = if is_selected && !commit.hash.is_empty() {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected && !commit.hash.is_empty() { " ▶" } else { "  " };

        let mut entry = vec![
            Span::styled(format!("{:<15}", commit.graph), row_style.fg(theme.primary)),
            Span::styled(prefix, row_style.fg(theme.primary)),
        ];

        if !commit.hash.is_empty() {
            entry.extend(vec![
                Span::styled(format!(" {} ", commit.hash), row_style.fg(theme.warning)),
                Span::styled(format!(" {:<10} ", commit.date), row_style.fg(theme.subtle)),
                Span::styled(
                    format!(" {:<15} ", commit.author),
                    row_style.fg(theme.accent),
                ),
                Span::styled(&commit.message, row_style),
            ]);
        }

        items.push(Line::from(entry));
    }

    let p = Paragraph::new(items).alignment(Alignment::Left);
    f.render_widget(p, inner_area);

    // Footer
    let footer_area = Rect::new(area.x + 2, area.y + area.height - 1, area.width - 4, 1);
    let help_text = Paragraph::new(Line::from(vec![
        Span::styled(
            " [UP/DOWN]",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" NAVIGATE  "),
        Span::styled(
            " [ESC/Q]",
            Style::default()
                .fg(theme.subtle)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" BACK "),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help_text, footer_area);
}
